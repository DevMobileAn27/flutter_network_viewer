//! Owner-clock resource sampling: immediate, then every 2 seconds, no worker or
//! OS process spawn. Dropping the owner leaves no timer/callback behind.
use std::io;
pub struct ResourceUsage<S> {
    sampler: S,
    next: Option<u64>,
    memory: Option<u64>,
    stopped: bool,
    pub samples: u64,
}
impl<S: FnMut() -> io::Result<u64>> ResourceUsage<S> {
    pub fn new(sampler: S) -> Self {
        Self {
            sampler,
            next: None,
            memory: None,
            stopped: false,
            samples: 0,
        }
    }
    pub fn tick(&mut self, now: u64) {
        if self.stopped || self.next.is_some_and(|n| now < n) {
            return;
        }
        self.next = Some(now.saturating_add(2000));
        self.samples += 1;
        self.memory = (self.sampler)().ok();
    }
    pub fn memory_bytes(&self) -> Option<u64> {
        self.memory
    }
    pub fn label(&self) -> String {
        memory_label(self.memory)
    }
    pub fn stop(&mut self) {
        self.stopped = true;
        self.next = None;
        self.memory = None;
    }
}
pub fn memory_label(bytes: Option<u64>) -> String {
    bytes.map_or_else(
        || "RAM —".into(),
        |n| format!("RAM {:.0} MB", n as f64 / 1_000_000.),
    )
}
#[cfg(target_os = "macos")]
pub fn sample_process_memory() -> io::Result<u64> {
    unsafe extern "C" {
        static mach_task_self_: u32;
        fn task_info(task: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
    }
    // TASK_VM_INFO's fixed prefix through phys_footprint: two u32 fields
    // (region_count/page_size) share slot 1; phys_footprint is u64 slot 18.
    // Oversized aligned buffer accepts the current kernel's later revisions.
    let mut data = [0u64; 128];
    let mut count = (std::mem::size_of_val(&data) / 4) as u32;
    let status = unsafe { task_info(mach_task_self_, 22, data.as_mut_ptr().cast(), &mut count) };
    if status == 0 && count >= 38 {
        Ok(data[18])
    } else {
        Err(io::Error::other("task_info unavailable"))
    }
}
#[cfg(target_os = "linux")]
pub fn sample_process_memory() -> io::Result<u64> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u64>().ok())
        .and_then(|n| n.checked_mul(1024))
        .ok_or_else(|| io::Error::other("VmRSS unavailable"))
}
#[cfg(windows)]
pub fn sample_process_memory() -> io::Result<u64> {
    #[repr(C)]
    struct Counters {
        cb: u32,
        page_faults: u32,
        peak_working_set: usize,
        working_set: usize,
        rest: [usize; 6],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut Counters,
            size: u32,
        ) -> i32;
    }
    let size = std::mem::size_of::<Counters>() as u32;
    let mut c = Counters {
        cb: size,
        page_faults: 0,
        peak_working_set: 0,
        working_set: 0,
        rest: [0; 6],
    };
    if unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut c, size) } != 0 {
        Ok(c.working_set as u64)
    } else {
        Err(io::Error::last_os_error())
    }
}
#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub fn sample_process_memory() -> io::Result<u64> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "resource sampling unavailable",
    ))
}
