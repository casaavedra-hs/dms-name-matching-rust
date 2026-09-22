use std::sync::{Mutex, OnceLock};

use crate::model::PerformanceConfig;

#[derive(Debug, Clone, Default)]
pub struct ResourceSnapshot {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_used_gb: f32,
    pub memory_total_gb: f32,
    pub memory_available_gb: f32,
}

#[cfg(windows)]
mod platform {
    use super::*;

    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    }

    fn ft(v: FileTime) -> u64 {
        ((v.high as u64) << 32) | v.low as u64
    }

    static LAST_CPU: OnceLock<Mutex<Option<(u64, u64, u64)>>> = OnceLock::new();

    pub fn snapshot() -> ResourceSnapshot {
        let mut mem = MemoryStatusEx {
            length: std::mem::size_of::<MemoryStatusEx>() as u32,
            memory_load: 0,
            total_phys: 0,
            avail_phys: 0,
            total_page_file: 0,
            avail_page_file: 0,
            total_virtual: 0,
            avail_virtual: 0,
            avail_extended_virtual: 0,
        };
        let mut idle = FileTime { low: 0, high: 0 };
        let mut kernel = FileTime { low: 0, high: 0 };
        let mut user = FileTime { low: 0, high: 0 };

        let mem_ok = unsafe { GlobalMemoryStatusEx(&mut mem) != 0 };
        let cpu_ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0 };

        let gib = 1024.0_f32 * 1024.0 * 1024.0;
        let total = if mem_ok { mem.total_phys as f32 / gib } else { 0.0 };
        let available = if mem_ok { mem.avail_phys as f32 / gib } else { 0.0 };
        let used = (total - available).max(0.0);
        let memory_percent = if total > 0.0 { used / total * 100.0 } else { 0.0 };

        let cpu_percent = if cpu_ok {
            let now = (ft(idle), ft(kernel), ft(user));
            let lock = LAST_CPU.get_or_init(|| Mutex::new(None));
            let mut previous = lock.lock().unwrap_or_else(|e| e.into_inner());
            let pct = if let Some(old) = *previous {
                let idle_delta = now.0.saturating_sub(old.0);
                let kernel_delta = now.1.saturating_sub(old.1);
                let user_delta = now.2.saturating_sub(old.2);
                let total_delta = kernel_delta.saturating_add(user_delta);
                if total_delta > 0 {
                    (100.0 * (1.0 - idle_delta as f32 / total_delta as f32)).clamp(0.0, 100.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            *previous = Some(now);
            pct
        } else {
            0.0
        };

        ResourceSnapshot {
            cpu_percent,
            memory_percent,
            memory_used_gb: used,
            memory_total_gb: total,
            memory_available_gb: available,
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    pub fn snapshot() -> ResourceSnapshot { ResourceSnapshot::default() }
}

pub fn system_snapshot() -> ResourceSnapshot {
    platform::snapshot()
}

pub struct ResourceGovernor {
    cfg: PerformanceConfig,
}

impl ResourceGovernor {
    pub fn new(cfg: PerformanceConfig) -> Self {
        Self { cfg }
    }

    pub fn snapshot(&mut self) -> ResourceSnapshot {
        system_snapshot()
    }

    pub fn fuzzy_source_chunk(&mut self) -> usize {
        let s = self.snapshot();
        if s.memory_total_gb <= 0.0 {
            return self.cfg.fuzzy_source_chunk.clamp(250, 20_000);
        }
        let reserve = self.cfg.min_free_ram_gb.max(
            s.memory_total_gb * (100.0 - self.cfg.memory_limit_percent) / 100.0
        );
        let free_after_reserve = (s.memory_available_gb - reserve).max(0.25);
        let estimated_candidate_bytes = 520.0_f32;
        let by_ram = ((free_after_reserve * 1024.0 * 1024.0 * 1024.0 * 0.30)
            / (self.cfg.fuzzy_candidate_cap.max(1) as f32 * estimated_candidate_bytes)) as usize;
        let mut target = self.cfg.fuzzy_source_chunk.min(by_ram.max(250));
        if s.memory_percent >= self.cfg.memory_limit_percent || s.memory_available_gb <= reserve {
            target = (target / 2).max(250);
        } else if s.memory_percent < 65.0 && s.memory_available_gb > reserve * 1.75 {
            target = (target * 2).min(20_000).min(by_ram.max(250));
        }
        target.clamp(250, 20_000)
    }
}
