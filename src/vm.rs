//! VM Service wire protocol and HTTP profile adapter. Lifecycle/polling are separate.
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    net::{Shutdown, TcpStream, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};
use url::Url;

pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum RpcError {
    InvalidUri,
    InvalidMessage,
    InvalidRequest,
    Remote {
        code: i64,
        message: String,
        data: Value,
    },
    Timeout,
    Transport(String),
}
impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Remote { code, message, .. } => write!(f, "VM RPC {code}: {message}"),
            Self::Transport(_) => write!(f, "VM transport failure"),
            other => write!(f, "{other:?}"),
        }
    }
}
impl std::error::Error for RpcError {}

/// Preserve encoded auth path segments; discard query only after unwrapping uri=.
pub fn normalize_uri(input: &str) -> Result<String, RpcError> {
    let input = input.trim();
    // WHATWG URL parsing repairs missing authority slashes; Dart rejects those.
    fn parse(input: &str) -> Result<Url, RpcError> {
        let (_, authority) = input.split_once("://").ok_or(RpcError::InvalidUri)?;
        if authority.is_empty()
            || authority.starts_with('/')
            || input.contains(['\\', '\n', '\r', '\t'])
        {
            return Err(RpcError::InvalidUri);
        }
        Url::parse(input).map_err(|_| RpcError::InvalidUri)
    }
    let mut uri = parse(input)?;
    let nested = uri
        .query_pairs()
        .find(|(k, _)| k == "uri")
        .map(|(_, v)| v.into_owned());
    if let Some(nested) = nested {
        uri = parse(&nested)?;
    }
    if !matches!(uri.scheme(), "http" | "https" | "ws" | "wss")
        || uri.host_str().is_none()
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.fragment().is_some()
        || uri.path().contains("/devtools")
    {
        return Err(RpcError::InvalidUri);
    }
    let secure = matches!(uri.scheme(), "https" | "wss");
    uri.set_scheme(if secure { "wss" } else { "ws" })
        .map_err(|_| RpcError::InvalidUri)?;
    let mut path = uri.path().to_owned();
    if !path.ends_with("/ws") {
        if !path.ends_with('/') {
            path.push('/');
        }
        path.push_str("ws");
    }
    uri.set_path(&path);
    uri.set_query(None);
    Ok(uri.to_string())
}

#[derive(Debug, PartialEq)]
pub enum Incoming {
    Response {
        id: u64,
        result: Result<Value, RpcError>,
    },
    Notification {
        method: String,
        params: Value,
    },
    Ignored,
}
/// Pure correlation/deadline state; caller supplies a monotonic millisecond clock.
#[derive(Default)]
pub struct RpcCore {
    next_id: u64,
    pending: BTreeMap<u64, u64>,
}
impl RpcCore {
    pub fn request(
        &mut self,
        method: &str,
        params: Value,
        now: u64,
        timeout: u64,
    ) -> Result<Value, RpcError> {
        if method.is_empty() || !params.is_object() {
            return Err(RpcError::InvalidRequest);
        }
        let id = self
            .next_id
            .checked_add(1)
            .ok_or(RpcError::InvalidRequest)?;
        self.next_id = id;
        self.pending.insert(id, now.saturating_add(timeout));
        Ok(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
    }
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
    pub fn expire(&mut self, now: u64) -> Vec<u64> {
        let ids = self
            .pending
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in &ids {
            self.pending.remove(id);
        }
        ids
    }
    pub fn receive(&mut self, raw: &str) -> Result<Incoming, RpcError> {
        let value: Value = serde_json::from_str(raw).map_err(|_| RpcError::InvalidMessage)?;
        if value["jsonrpc"] != "2.0" {
            return Err(RpcError::InvalidMessage);
        }
        if value.get("id").is_none() {
            return value["method"]
                .as_str()
                .map(|method| Incoming::Notification {
                    method: method.into(),
                    params: value["params"].clone(),
                })
                .ok_or(RpcError::InvalidMessage);
        }
        let id = value["id"].as_u64().ok_or(RpcError::InvalidMessage)?;
        if !self.pending.contains_key(&id) {
            return Ok(Incoming::Ignored);
        }
        let result = match (value.get("result"), value.get("error")) {
            (Some(result), None) => Ok(result.clone()),
            (None, Some(error)) => Err(RpcError::Remote {
                code: error["code"].as_i64().ok_or(RpcError::InvalidMessage)?,
                message: error["message"]
                    .as_str()
                    .ok_or(RpcError::InvalidMessage)?
                    .into(),
                data: error["data"].clone(),
            }),
            _ => return Err(RpcError::InvalidMessage),
        };
        self.pending.remove(&id);
        Ok(Incoming::Response { id, result })
    }
}
fn object(value: &Value) -> Value {
    if value.is_object() {
        value.clone()
    } else {
        json!({})
    }
}
fn text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}
pub fn declared_body_length(raw: &Value) -> Option<u64> {
    if let Some(n) = raw["contentLength"].as_u64() {
        return Some(n);
    }
    if let Some(n) = raw["contentLength"]
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0.0)
    {
        return Some(n as u64);
    }
    raw["headers"].as_object()?.iter().find_map(|(key, v)| {
        if !key.eq_ignore_ascii_case("content-length") {
            return None;
        }
        let v = if let Some(a) = v.as_array() {
            a.first()?
        } else {
            v
        };
        text(v).trim().parse().ok()
    })
}
pub fn exceeds_body_limit(summary: &Value) -> bool {
    declared_body_length(&summary["request"])
        .unwrap_or(0)
        .saturating_add(declared_body_length(&summary["response"]).unwrap_or(0))
        > MAX_BODY_BYTES as u64
}
pub fn omit_bodies(summary: &Value, reason: &str) -> Value {
    let mut out = object(summary);
    out.as_object_mut().unwrap().remove("requestBody");
    out.as_object_mut().unwrap().remove("responseBody");
    for side in ["request", "response"] {
        out[side] = object(&summary[side]);
        out[side]["bodyOmitted"] = json!(reason);
        out[side]["bodyCaptureLimit"] = json!(MAX_BODY_BYTES);
        if let Some(n) = declared_body_length(&summary[side]) {
            out[side]["bodyByteLength"] = json!(n);
        }
    }
    out
}
pub fn body(raw: &Value) -> Value {
    if raw.is_string() {
        return json!({"body":raw});
    }
    let Some(array) = raw.as_array() else {
        return json!({});
    };
    if array.len() > MAX_BODY_BYTES {
        return json!({"bodyOmitted":"profile-size-limit","bodyCaptureLimit":MAX_BODY_BYTES,"bodyByteLength":array.len()});
    }
    let bytes: Option<Vec<u8>> = array
        .iter()
        .map(|n| n.as_u64().and_then(|n| u8::try_from(n).ok()))
        .collect();
    let Some(bytes) = bytes else {
        return json!({"bodyOmitted":"invalid-body-bytes"});
    };
    match std::str::from_utf8(&bytes) {
        Ok(s) => json!({"body":s,"bodyEncoding":"utf-8","bodyByteLength":bytes.len()}),
        Err(_) => {
            json!({"body":STANDARD.encode(&bytes),"bodyEncoding":"base64","bodyByteLength":bytes.len()})
        }
    }
}
fn headers(raw: &Value) -> Value {
    let mut out = json!({});
    if let Some(map) = raw.as_object() {
        for (k, v) in map {
            out[k] = json!(if let Some(a) = v.as_array() {
                a.iter().map(text).collect::<Vec<_>>().join(", ")
            } else {
                text(v)
            });
        }
    }
    out
}
fn timestamp(raw: &Value) -> String {
    let date = raw
        .as_i64()
        .and_then(DateTime::from_timestamp_micros)
        .unwrap_or_else(Utc::now);
    // Dart emits three decimals when microseconds are zero, otherwise six.
    date.to_rfc3339_opts(
        if date.timestamp_subsec_micros().is_multiple_of(1000) {
            SecondsFormat::Millis
        } else {
            SecondsFormat::Micros
        },
        true,
    )
}
pub fn network(profile: &Value) -> Vec<Value> {
    let request = &profile["request"];
    let response = &profile["response"];
    let error = if !request["error"].is_null() {
        &request["error"]
    } else {
        &response["error"]
    };
    let end = if !response["endTime"].is_null() {
        &response["endTime"]
    } else if !error.is_null() {
        &profile["endTime"]
    } else {
        &Value::Null
    };
    let mut req = object(request);
    req["method"] = profile["method"].clone();
    req["url"] = profile["uri"].clone();
    req["headers"] = headers(&request["headers"]);
    req.as_object_mut()
        .unwrap()
        .extend(body(&profile["requestBody"]).as_object().unwrap().clone());
    let id = text(&profile["id"]);
    let mut events = vec![
        json!({"requestId":id,"source":"dart.io.http","phase":"request","timestamp":timestamp(&profile["startTime"]),"request":req,"profileEvents":profile["events"]}),
    ];
    if !end.is_null() || !error.is_null() {
        let mut res = object(response);
        res["headers"] = headers(&response["headers"]);
        res["statusText"] = response["reasonPhrase"].clone();
        res.as_object_mut()
            .unwrap()
            .extend(body(&profile["responseBody"]).as_object().unwrap().clone());
        let mut event = json!({"requestId":id,"source":"dart.io.http","phase":if error.is_null(){"response"}else{"error"},"timestamp":timestamp(end),"response":res});
        if let (Some(start), Some(end)) = (profile["startTime"].as_f64(), end.as_f64()) {
            event["durationMs"] = json!((end - start) / 1000.0);
        }
        if !error.is_null() {
            event["error"] = json!({"message":text(error)});
        }
        events.push(event);
    }
    events
}
pub fn scoped_events(profile: &Value, isolate_id: &str, isolate_name: &str) -> Vec<Value> {
    network(profile)
        .into_iter()
        .map(|mut event| {
            event["vmRequestId"] = event["requestId"].clone();
            event["requestId"] = json!(json!([isolate_id, event["vmRequestId"]]).to_string());
            event["isolateId"] = json!(isolate_id);
            event["isolateName"] = json!(isolate_name);
            event
        })
        .collect()
}

/// Cancellation is registered before TLS/WebSocket handshake or subscriptions.
/// Cancelling closes the cloned TCP socket, waking a blocked setup read.
#[derive(Clone, Default)]
pub struct ConnectionCancel(Arc<CancelState>);
#[derive(Default)]
struct CancelState {
    cancelled: AtomicBool,
    socket: Mutex<Option<TcpStream>>,
}
impl ConnectionCancel {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        if let Some(s) = self.0.socket.lock().unwrap().take() {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    fn check(&self) -> Result<(), RpcError> {
        if self.is_cancelled() {
            Err(transport("connection cancelled"))
        } else {
            Ok(())
        }
    }
    pub(crate) fn close_registered(&self) {
        if let Some(socket) = self.0.socket.lock().unwrap().take() {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
    pub(crate) fn register(&self, socket: &TcpStream) -> Result<(), RpcError> {
        let mut slot = self.0.socket.lock().unwrap();
        self.check()?;
        *slot = Some(socket.try_clone().map_err(transport)?);
        Ok(())
    }
}
/// Blocking wire client. Run on a worker. One outstanding call per client;
/// RpcCore can also be used by a future multiplexed transport. No reconnect loop.
pub struct VmClient {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    rpc: RpcCore,
    timeout: Duration,
    notifications: VecDeque<Value>,
}
fn transport(error: impl fmt::Display) -> RpcError {
    RpcError::Transport(error.to_string())
}
impl VmClient {
    pub fn connect(uri: &str, timeout: Duration) -> Result<Self, RpcError> {
        Self::connect_cancellable(uri, timeout, &ConnectionCancel::default())
    }
    pub(crate) fn connect_cancellable(
        uri: &str,
        timeout: Duration,
        cancel: &ConnectionCancel,
    ) -> Result<Self, RpcError> {
        cancel.check()?;
        if timeout.is_zero() {
            return Err(RpcError::Timeout);
        }
        let uri = normalize_uri(uri)?;
        let url = Url::parse(&uri).map_err(|_| RpcError::InvalidUri)?;
        let host = url
            .host_str()
            .ok_or(RpcError::InvalidUri)?
            .trim_matches(['[', ']']);
        let start = Instant::now();
        let mut stream = None;
        for address in (
            host,
            url.port_or_known_default().ok_or(RpcError::InvalidUri)?,
        )
            .to_socket_addrs()
            .map_err(transport)?
        {
            cancel.check()?;
            let remaining = timeout
                .checked_sub(start.elapsed())
                .ok_or(RpcError::Timeout)?;
            if let Ok(s) = TcpStream::connect_timeout(&address, remaining) {
                stream = Some(s);
                break;
            }
        }
        cancel.check()?;
        let stream = stream.ok_or_else(|| transport("TCP connection failed"))?;
        cancel.register(&stream)?;
        let remaining = timeout
            .checked_sub(start.elapsed())
            .ok_or(RpcError::Timeout)?;
        stream
            .set_read_timeout(Some(remaining))
            .map_err(transport)?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(transport)?;
        let config = tungstenite::protocol::WebSocketConfig::default()
            .max_message_size(Some(48 * 1024 * 1024))
            .max_frame_size(Some(48 * 1024 * 1024));
        let (socket, _) = tungstenite::client_tls_with_config(uri, stream, Some(config), None)
            .map_err(transport)?;
        cancel.check()?;
        Ok(Self {
            socket,
            rpc: RpcCore::default(),
            timeout,
            notifications: VecDeque::new(),
        })
    }
    fn set_timeout(&mut self, remaining: Duration) -> Result<(), RpcError> {
        let stream = match self.socket.get_mut() {
            MaybeTlsStream::Plain(s) => s,
            MaybeTlsStream::NativeTls(s) => s.get_mut(),
            _ => return Err(transport("unsupported TLS backend")),
        };
        stream
            .set_read_timeout(Some(remaining))
            .map_err(transport)?;
        stream.set_write_timeout(Some(remaining)).map_err(transport)
    }
    pub fn take_notification(&mut self) -> Option<Value> {
        self.notifications.pop_front()
    }
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        self.call_inner(method, params, None)
    }
    /// Deliver stream messages immediately while waiting for the RPC response.
    pub(crate) fn call_streaming(
        &mut self,
        method: &str,
        params: Value,
        sink: &mut dyn FnMut(Value),
    ) -> Result<Value, RpcError> {
        self.call_inner(method, params, Some(sink))
    }
    pub(crate) fn shutdown_handle(&self) -> Result<TcpStream, RpcError> {
        match self.socket.get_ref() {
            MaybeTlsStream::Plain(s) => s,
            MaybeTlsStream::NativeTls(s) => s.get_ref(),
            _ => return Err(transport("unsupported TLS backend")),
        }
        .try_clone()
        .map_err(transport)
    }
    /// A short idle read preserves tungstenite's partial-frame state and ignores
    /// late replies whose RPC deadline has already expired.
    pub(crate) fn pump(&mut self) -> Result<Option<Value>, RpcError> {
        self.set_timeout(Duration::from_millis(10))?;
        match self.socket.read() {
            Ok(Message::Text(raw)) => match self.rpc.receive(&raw)? {
                Incoming::Notification { method, params } => {
                    Ok(Some(json!({"method":method,"params":params})))
                }
                _ => Ok(None),
            },
            Ok(Message::Close(_)) => Err(transport("socket closed")),
            Ok(Message::Binary(_)) => Err(RpcError::InvalidMessage),
            Ok(_) => Ok(None),
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(None)
            }
            Err(e) => Err(transport(e)),
        }
    }
    fn call_inner(
        &mut self,
        method: &str,
        params: Value,
        mut sink: Option<&mut dyn FnMut(Value)>,
    ) -> Result<Value, RpcError> {
        let request = self.rpc.request(method, params, 0, u64::MAX)?;
        let id = request["id"].as_u64().unwrap();
        let start = Instant::now();
        let result = (|| {
            self.set_timeout(self.timeout)?;
            self.socket
                .send(Message::Text(request.to_string().into()))
                .map_err(transport)?;
            loop {
                let remaining = self
                    .timeout
                    .checked_sub(start.elapsed())
                    .filter(|d| !d.is_zero())
                    .ok_or(RpcError::Timeout)?;
                self.set_timeout(remaining)?;
                let message = match self.socket.read() {
                    Ok(m) => m,
                    Err(tungstenite::Error::Io(e))
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        return Err(RpcError::Timeout);
                    }
                    Err(e) => return Err(transport(e)),
                };
                match message {
                    Message::Text(raw) => match self.rpc.receive(&raw)? {
                        Incoming::Response { id: reply, result } if reply == id => return result,
                        Incoming::Notification { method, params } => {
                            let exited = params["streamId"] == "Isolate"
                                && params["event"]["kind"] == "IsolateExit"
                                && request["params"]["isolateId"].is_string()
                                && params["event"]["isolate"]["id"]
                                    == request["params"]["isolateId"];
                            let notification = json!({"method":method,"params":params});
                            if let Some(ref mut sink) = sink {
                                sink(notification);
                            } else {
                                if self.notifications.len() == 1024 {
                                    self.notifications.pop_front();
                                }
                                self.notifications.push_back(notification);
                            }
                            if exited {
                                return Err(RpcError::Remote {
                                    code: -32800,
                                    message: "isolate exited".into(),
                                    data: Value::Null,
                                });
                            }
                        }
                        _ => {}
                    },
                    Message::Close(_) => return Err(transport("socket closed")),
                    Message::Binary(_) => return Err(RpcError::InvalidMessage),
                    _ => {}
                }
            }
        })();
        self.rpc.pending.remove(&id);
        result
    }
    /// A detail RPC failure belongs to the profile; transport/capture lifecycle is
    /// decided by the caller. Preflight prevents serializing known oversized data.
    pub fn profile_detail(&mut self, isolate: &str, summary: &Value) -> Result<Value, RpcError> {
        if exceeds_body_limit(summary) {
            return Ok(omit_bodies(summary, "profile-size-limit"));
        }
        match self.call(
            "ext.dart.io.getHttpProfileRequest",
            json!({"isolateId":isolate,"id":text(&summary["id"])}),
        ) {
            Ok(detail) => Ok(detail),
            Err(RpcError::Remote { .. } | RpcError::Timeout) => {
                Ok(omit_bodies(summary, "profile-read-error"))
            }
            Err(error) => Err(error),
        }
    }
}
