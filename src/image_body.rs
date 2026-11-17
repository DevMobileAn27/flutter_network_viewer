//! Captured image normalization and per-image pixel ownership. No global cache,
//! file reads, or HTTP requests. GUI hosts can share one Arc<ImageAsset>.
use crate::form_body::decode_base64;
use image::{DynamicImage, ImageDecoder, ImageReader, Limits};
use serde_json::Value;
use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};

#[derive(Debug, Clone)]
pub struct ImageBodyData {
    /// Image intent is retained even when the transport or pixel data is invalid.
    pub image: bool,
    pub bytes: Option<Arc<[u8]>>,
}
fn has_signature(bytes: &[u8]) -> bool {
    [
        b"\x89PNG\r\n\x1a\n".as_slice(),
        b"\xff\xd8\xff",
        b"GIF87a",
        b"GIF89a",
        b"BM",
    ]
    .iter()
    .any(|s| bytes.starts_with(s))
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
}
fn base64_image_prefix(text: &str) -> bool {
    let mut prefix = String::new();
    for c in text.chars() {
        if prefix.len() >= 64 {
            break;
        }
        if matches!(c, ' ' | '\t' | '\r' | '\n') {
            continue;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '-' | '_' | '=')) {
            return false;
        }
        prefix.push(c);
    }
    prefix.len() >= 4 && decode_base64(&prefix).is_some_and(|b| has_signature(&b))
}
pub fn may_contain_image_body(data: &Value) -> bool {
    let Some(body) = data["body"].as_str() else {
        return false;
    };
    if data["image"] == true || data["base64"] == true {
        return true;
    }
    // Dart takes the first 128 UTF-16 units before trimming.
    let mut units = 0;
    let prefix: String = body
        .chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= 128
        })
        .collect();
    let mut text = prefix.trim_start();
    if let Some(rest) = text.strip_prefix('"') {
        text = rest.trim_start();
    }
    text.to_ascii_lowercase().starts_with("data:image/") || base64_image_prefix(text)
}
/// Accepts the Flutter isolate envelope {body, image, base64}. Invalid data
/// leaves the raw captured Value untouched and retains any explicit image intent.
pub fn decode_image_body(data: &Value) -> ImageBodyData {
    let mut image = data["image"] == true;
    let mut bytes: Option<Vec<u8>> = None;
    if let Some(body) = data["body"].as_str() {
        let _ = (|| -> Option<()> {
            let mut text = body.trim().to_owned();
            if data["base64"] == true {
                let captured = decode_base64(&text)?;
                if has_signature(&captured) {
                    bytes = Some(captured);
                } else {
                    if image {
                        bytes = Some(captured.clone());
                    }
                    text = String::from_utf8(captured).ok()?;
                }
            }
            if !bytes.as_deref().is_some_and(has_signature) {
                if text.starts_with('"') {
                    let decoded: Value = serde_json::from_str(&text).ok()?;
                    if let Some(decoded) = decoded.as_str() {
                        text = decoded.trim().to_owned();
                    }
                }
                if text.to_ascii_lowercase().starts_with("data:image/") {
                    image = true;
                    bytes = Some(data_uri_bytes(&text)?);
                } else if base64_image_prefix(&text) {
                    if let Some(decoded) = decode_base64(&text) {
                        bytes = Some(decoded);
                    } else if image && bytes.is_none() {
                        bytes = Some(text.into_bytes());
                    }
                } else if image && bytes.is_none() {
                    bytes = Some(text.into_bytes());
                }
            }
            Some(())
        })();
    }
    image |= bytes.as_deref().is_some_and(has_signature);
    ImageBodyData {
        image,
        bytes: if image { bytes.map(Arc::from) } else { None },
    }
}
/// Maps the immutable request/response capture fields into the isolate contract.
pub fn image_body_from_capture(data: &Value) -> ImageBodyData {
    let image = data["headers"].as_object().is_some_and(|headers| {
        headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("content-type")
                && v.to_string().to_ascii_lowercase().contains("image/")
        })
    });
    let body = if data["body"].is_null() {
        &data["bodyPreview"]
    } else {
        &data["body"]
    };
    decode_image_body(
        &serde_json::json!({"body":body,"image":image,"base64":data["bodyEncoding"]=="base64"}),
    )
}
fn data_uri_bytes(text: &str) -> Option<Vec<u8>> {
    let (metadata, content) = text.split_once(',')?;
    if metadata
        .split(';')
        .any(|p| p.eq_ignore_ascii_case("base64"))
    {
        return decode_base64(content);
    }
    let mut out = Vec::new();
    let mut iter = content.bytes();
    while let Some(b) = iter.next() {
        out.push(if b == b'%' {
            let high = (iter.next()? as char).to_digit(16)?;
            let low = (iter.next()? as char).to_digit(16)?;
            (high * 16 + low) as u8
        } else {
            b
        });
    }
    Some(out)
}

#[derive(Debug)]
pub struct ImagePixels {
    pub width: u32,
    pub height: u32,
    /// Straight alpha RGBA8 for a future desktop texture upload.
    pub rgba: Vec<u8>,
}
/// One resource per captured image. Clone the Arc, rather than re-decoding when
/// opening an enlarged view. Dropping the last owner drops bytes and pixels.
#[derive(Debug)]
pub struct ImageAsset {
    bytes: Arc<[u8]>,
    pixels: OnceLock<Result<ImagePixels, String>>,
}
impl ImageAsset {
    pub fn new(bytes: Arc<[u8]>) -> Self {
        Self {
            bytes,
            pixels: OnceLock::new(),
        }
    }
    /// Transfer a prepared buffer to the UI worker result without duplicating RGBA.
    pub fn into_pixels(self) -> Result<ImagePixels, String> {
        self.pixels
            .into_inner()
            .unwrap_or_else(|| decode_pixels(&self.bytes).map_err(|e| e.to_string()))
    }
    pub fn bytes(&self) -> &Arc<[u8]> {
        &self.bytes
    }
    pub fn is_prepared(&self) -> bool {
        self.pixels.get().is_some()
    }
    /// Run on a worker thread in the GUI. Both success and failure are cached
    /// locally; changing view transforms never requires another pixel decode.
    pub fn pixels(&self) -> &Result<ImagePixels, String> {
        self.pixels
            .get_or_init(|| decode_pixels(&self.bytes).map_err(|e| e.to_string()))
    }
}
fn decode_pixels(bytes: &[u8]) -> image::ImageResult<ImagePixels> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = Limits::default();
    // Rust codecs allocate a full frame. ImageIO handles oversized macOS sources
    // by creating a bounded thumbnail before the Rust RGBA raster is allocated.
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    if decoder.total_bytes() > 64 * 1024 * 1024
        || u64::from(width) * u64::from(height) * 4 > 64 * 1024 * 1024
    {
        #[cfg(target_os = "macos")]
        return crate::image_thumbnail_macos::thumbnail(bytes)
            .map_err(|e| image::ImageError::IoError(std::io::Error::other(e)));
        #[cfg(not(target_os = "macos"))]
        return Err(image::ImageError::Limits(
            image::error::LimitError::from_kind(image::error::LimitErrorKind::InsufficientMemory),
        ));
    }
    let orientation = decoder.orientation()?;
    let decoded = DynamicImage::from_decoder(decoder)?;
    let mut thumbnail = if decoded.width().max(decoded.height()) > 4096 {
        let ratio = 4096.0 / f64::from(decoded.width().max(decoded.height()));
        let width = (f64::from(decoded.width()) * ratio).floor().max(1.0) as u32;
        let height = (f64::from(decoded.height()) * ratio).floor().max(1.0) as u32;
        decoded.resize_exact(width, height, image::imageops::FilterType::Triangle)
    } else {
        decoded
    };
    thumbnail.apply_orientation(orientation);
    let rgba = thumbnail.into_rgba8();
    Ok(ImagePixels {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}
