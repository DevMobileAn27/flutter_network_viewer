//! WinSparkle process singleton. The DLL is loaded beside the executable only.
//! Callbacks enqueue bounded enums, never borrowed Rust pointers.
use super::{Backend, Event};
use std::{
    ffi::{CString, c_void},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
static OWNED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());
fn push(e: Event) {
    if ACTIVE.load(Ordering::Acquire) {
        let mut q = EVENTS.lock().unwrap();
        if q.len() < 32 {
            q.push(e);
        }
    }
}
unsafe extern "C" fn available() {
    push(Event::Available);
}
unsafe extern "C" fn current() {
    push(Event::Current);
}
unsafe extern "C" fn failed() {
    push(Event::Failed);
}
unsafe extern "C" fn cancelled() {
    push(Event::Finished);
}
unsafe extern "C" fn quit() {
    push(Event::Installing);
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
}
type Void = unsafe extern "C" fn();
type Callback = unsafe extern "C" fn(Option<Void>);
pub struct WindowsBackend {
    module: *mut c_void,
    cleanup: Void,
    check: Void,
    background: Void,
}
impl WindowsBackend {
    unsafe fn symbol<T: Copy>(module: *mut c_void, name: &[u8]) -> Result<T, String> {
        let p = unsafe { GetProcAddress(module, name.as_ptr()) };
        if p.is_null() {
            return Err("WinSparkle API unavailable".into());
        }
        Ok(unsafe { std::mem::transmute_copy(&p) })
    }
    pub fn load(feed: &str) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        if OWNED.swap(true, Ordering::AcqRel) {
            return Err("WinSparkle already owned".into());
        }
        let result = (|| {
            let path = std::env::current_exe()
                .map_err(|_| "Executable unavailable")?
                .with_file_name("WinSparkle.dll");
            let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let module =
                unsafe { LoadLibraryExW(path.as_ptr(), std::ptr::null_mut(), 0x100 | 0x1000) };
            if module.is_null() {
                return Err("WinSparkle.dll unavailable".into());
            }
            let result = (|| unsafe {
                let init: Void = Self::symbol(module, b"win_sparkle_init\0")?;
                let cleanup = Self::symbol(module, b"win_sparkle_cleanup\0")?;
                let check = Self::symbol(module, b"win_sparkle_check_update_with_ui\0")?;
                let background = Self::symbol(module, b"win_sparkle_check_update_without_ui\0")?;
                let set_feed: unsafe extern "C" fn(*const i8) =
                    Self::symbol(module, b"win_sparkle_set_appcast_url\0")?;
                let set_key: unsafe extern "C" fn(*const i8) -> i32 =
                    Self::symbol(module, b"win_sparkle_set_dsa_pub_pem\0")?;
                let set_auto: unsafe extern "C" fn(i32) =
                    Self::symbol(module, b"win_sparkle_set_automatic_check_for_updates\0")?;
                let details: unsafe extern "C" fn(*const u16, *const u16, *const u16) =
                    Self::symbol(module, b"win_sparkle_set_app_details\0")?;
                for (symbol, callback) in [
                    (
                        b"win_sparkle_set_did_find_update_callback\0".as_slice(),
                        available as Void,
                    ),
                    (
                        b"win_sparkle_set_did_not_find_update_callback\0",
                        current as Void,
                    ),
                    (b"win_sparkle_set_error_callback\0", failed as Void),
                    (
                        b"win_sparkle_set_update_cancelled_callback\0",
                        cancelled as Void,
                    ),
                    (b"win_sparkle_set_shutdown_request_callback\0", quit as Void),
                ] {
                    let setter: Callback = Self::symbol(module, symbol)?;
                    setter(Some(callback));
                }
                let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
                details(
                    wide("Nguyễn Quốc An").as_ptr(),
                    wide("Flutter Network Viewer").as_ptr(),
                    wide(env!("CARGO_PKG_VERSION")).as_ptr(),
                );
                set_feed(CString::new(feed).map_err(|_| "Invalid feed")?.as_ptr());
                if set_key(
                    CString::new(include_str!("../native/windows_dsa_pub.pem"))
                        .unwrap()
                        .as_ptr(),
                ) == 0
                {
                    return Err("WinSparkle rejected signing key".into());
                }
                set_auto(0);
                ACTIVE.store(true, Ordering::Release);
                init();
                Ok(Self {
                    module,
                    cleanup,
                    check,
                    background,
                })
            })();
            if result.is_err() {
                unsafe { FreeLibrary(module) };
            }
            result
        })();
        if result.is_err() {
            OWNED.store(false, Ordering::Release);
        }
        result
    }
}
impl Backend for WindowsBackend {
    fn configure(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn check(&mut self, background: bool) -> Result<(), String> {
        unsafe {
            if background {
                (self.background)()
            } else {
                (self.check)()
            }
        };
        Ok(())
    }
    fn poll(&mut self) -> Vec<Event> {
        std::mem::take(&mut *EVENTS.lock().unwrap())
    }
    fn close(&mut self) {
        if self.module.is_null() {
            return;
        }
        ACTIVE.store(false, Ordering::Release);
        unsafe {
            (self.cleanup)();
            FreeLibrary(self.module);
        };
        self.module = std::ptr::null_mut();
        EVENTS.lock().unwrap().clear();
        OWNED.store(false, Ordering::Release);
    }
}
impl Drop for WindowsBackend {
    fn drop(&mut self) {
        self.close();
    }
}
