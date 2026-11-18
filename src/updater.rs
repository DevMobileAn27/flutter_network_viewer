//! Owner-thread updater. Native frameworks own confirmation and signature verification;
//! this controller never starts a silent download or invokes an installer itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Idle,
    Checking,
    Available,
    Current,
    Failed,
    Downloading,
    Downloaded,
    Installing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Available,
    Current,
    Failed,
    Downloading,
    Downloaded,
    Installing,
    Finished,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct View {
    pub supported: bool,
    pub status: Status,
    pub busy: bool,
}
pub fn feed_for(os: &str, cpu: &str) -> Option<String> {
    let platform = match (os, cpu) {
        ("macos", "aarch64") => "macos-arm64",
        ("macos", "x86_64") => "macos-x86_64",
        ("windows", "x86_64") => "windows-x64",
        _ => return None,
    };
    Some(format!(
        "https://github.com/DevMobileAn27/flutter_network_viewer/releases/latest/download/appcast-{platform}.xml"
    ))
}
pub trait Backend {
    fn configure(&mut self, feed: &str) -> Result<(), String>;
    fn check(&mut self, background: bool) -> Result<(), String>;
    fn poll(&mut self) -> Vec<Event>;
    fn close(&mut self);
}
pub struct Updater<B: Backend> {
    backend: B,
    feed: Option<String>,
    windows: bool,
    configured: bool,
    listening: bool,
    background: bool,
    deadline: Option<u64>,
    session: bool,
    pub status: Status,
}
impl<B: Backend> Updater<B> {
    pub fn new(backend: B, feed: Option<String>, windows: bool) -> Self {
        Self {
            backend,
            feed,
            windows,
            configured: false,
            listening: false,
            background: false,
            deadline: None,
            session: false,
            status: Status::Idle,
        }
    }
    pub fn busy(&self) -> bool {
        self.session
            || matches!(
                self.status,
                Status::Checking | Status::Downloading | Status::Downloaded | Status::Installing
            )
    }
    pub fn view(&self) -> View {
        View {
            supported: self.feed.is_some(),
            // Startup checks are silent when offline, misconfigured, timed out,
            // or already current. Preserve the internal result for retry/state;
            // only a manual check should surface these outcomes in the UI.
            status: if self.background && matches!(self.status, Status::Failed | Status::Current) {
                Status::Idle
            } else {
                self.status
            },
            busy: self.busy(),
        }
    }
    pub fn check(&mut self, background: bool, now: u64) {
        if self.feed.is_none() || self.busy() {
            return;
        }
        self.status = Status::Checking;
        self.background = background;
        self.deadline = Some(now.saturating_add(60_000));
        self.listening = true;
        if !self.configured {
            if self.backend.configure(self.feed.as_ref().unwrap()).is_err() {
                self.fail();
                return;
            }
            self.configured = true;
        }
        if self.backend.check(background).is_err() {
            self.fail();
        }
    }
    fn fail(&mut self) {
        self.status = Status::Failed;
        self.deadline = None;
        self.session = false;
    }
    pub fn tick(&mut self, now: u64) {
        if !self.listening {
            return;
        }
        for event in self.backend.poll() {
            match event {
                Event::Available => {
                    self.status = Status::Available;
                    self.deadline = None;
                }
                Event::Current => {
                    self.status = Status::Current;
                    self.deadline = None;
                    self.session = false;
                }
                Event::Failed => {
                    if self.windows || self.status != Status::Current {
                        self.fail();
                    }
                }
                Event::Downloading => {
                    self.status = Status::Downloading;
                    self.session = true;
                    self.deadline = None;
                }
                Event::Downloaded => {
                    self.status = Status::Downloaded;
                    self.session = true;
                    self.deadline = None;
                }
                Event::Installing => {
                    self.status = Status::Installing;
                    self.session = true;
                    self.deadline = None;
                }
                Event::Finished => {
                    self.session = false;
                    self.deadline = None;
                    if matches!(
                        self.status,
                        Status::Checking | Status::Downloading | Status::Downloaded
                    ) {
                        self.status = Status::Idle;
                    }
                }
            }
        }
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.backend.close();
            self.configured = false;
            self.listening = false;
            self.fail();
        }
    }
}
impl<B: Backend> Drop for Updater<B> {
    fn drop(&mut self) {
        if self.listening {
            self.backend.close();
        }
    }
}
#[derive(Default)]
pub struct NativeBackend {
    #[cfg(windows)]
    windows: Option<windows::WindowsBackend>,
    #[cfg(target_os = "macos")]
    handle: *mut std::ffi::c_void,
}
impl Backend for NativeBackend {
    fn configure(&mut self, feed: &str) -> Result<(), String> {
        self.close();
        #[cfg(target_os = "macos")]
        {
            let feed = std::ffi::CString::new(feed).map_err(|_| "Invalid update feed")?;
            // FFI is owner/main-thread only. Native retains its delegate; no callback
            // holds a Rust pointer, and drop detaches the listener before releasing it.
            self.handle = unsafe { native::fnv_updater_create(feed.as_ptr()) };
            if !self.handle.is_null() {
                return Ok(());
            }
        }
        #[cfg(windows)]
        {
            self.windows = Some(windows::WindowsBackend::load(feed)?);
            Ok(())
        }
        #[cfg(not(windows))]
        let _ = feed;
        #[cfg(not(windows))]
        {
            Err("Native updater unavailable or bundle/signing configuration invalid".into())
        }
    }
    fn check(&mut self, background: bool) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        if !self.handle.is_null() && unsafe { native::fnv_updater_check(self.handle, background) } {
            return Ok(());
        }
        #[cfg(windows)]
        if let Some(w) = &mut self.windows {
            return w.check(background);
        }
        let _ = background;
        Err("Native updater could not start a check".into())
    }
    fn poll(&mut self) -> Vec<Event> {
        #[cfg(windows)]
        if let Some(w) = &mut self.windows {
            return w.poll();
        }
        #[allow(unused_mut)]
        let mut events = Vec::new();
        #[cfg(target_os = "macos")]
        if !self.handle.is_null() {
            for _ in 0..32 {
                let value = unsafe { native::fnv_updater_poll(self.handle) };
                let event = match value {
                    1 => Event::Available,
                    2 => Event::Current,
                    3 => Event::Failed,
                    4 => Event::Downloading,
                    5 => Event::Downloaded,
                    6 => Event::Installing,
                    7 => Event::Finished,
                    _ => break,
                };
                events.push(event);
            }
        }
        events
    }
    fn close(&mut self) {
        #[cfg(windows)]
        if let Some(mut w) = self.windows.take() {
            w.close();
        }
        #[cfg(target_os = "macos")]
        if !self.handle.is_null() {
            unsafe { native::fnv_updater_destroy(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}
impl Drop for NativeBackend {
    fn drop(&mut self) {
        self.close();
    }
}
impl Updater<NativeBackend> {
    pub fn desktop() -> Self {
        Self::new(
            NativeBackend::default(),
            feed_for(std::env::consts::OS, std::env::consts::ARCH),
            cfg!(windows),
        )
    }
}
#[cfg(target_os = "macos")]
mod native {
    use std::ffi::{c_char, c_void};
    unsafe extern "C" {
        pub fn fnv_updater_create(feed: *const c_char) -> *mut c_void;
        pub fn fnv_updater_check(handle: *mut c_void, background: bool) -> bool;
        pub fn fnv_updater_poll(handle: *mut c_void) -> i32;
        pub fn fnv_updater_destroy(handle: *mut c_void);
    }
}

#[cfg(windows)]
#[path = "updater_windows.rs"]
mod windows;
