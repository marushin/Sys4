//! CPU / RAM / network / disk sampling.
//!
//! Every subsystem has its own lock so a slow operation (e.g. RAM cleanup) never blocks
//! the 2-second polling of the others. All functions here are blocking and are meant to be
//! called from `spawn_blocking`, never directly on the async runtime.

use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sysinfo::{Disks, Networks, System};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::{EmptyWorkingSet, EnumProcesses};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA};

use crate::gpu::{GpuInfo, GpuMonitor};

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
/// Disk free space is re-read at most this often.
const DISK_REFRESH: Duration = Duration::from_secs(5);
/// The drive list (new USB drives, unmounted volumes) is rebuilt at most this often.
const DISK_LIST_REFRESH: Duration = Duration::from_secs(30);
/// Volumes smaller than this (recovery/EFI partitions) are hidden.
const MIN_DISK_GB: f64 = 0.5;

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Lock a mutex, recovering the data if a previous holder panicked.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub download_kbps: f64,
    pub upload_kbps: f64,
    pub total_received_gb: f64,
    pub total_transmitted_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub cpu_usage_percent: f64,
    pub cpu_cores: usize,
    pub cpu_name: String,
    pub uptime_secs: u64,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub ram_usage_percent: f64,
    pub swap_used_gb: f64,
    pub swap_total_gb: f64,
    pub gpu: Option<GpuInfo>,
    pub network: Option<NetworkInfo>,
    pub disks: Option<Vec<DiskInfo>>,
}

/// Which sections to sample. Missing fields default to `false`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RefreshOptions {
    pub cpu: bool,
    pub ram: bool,
    pub gpu: bool,
    pub network: bool,
    pub disk: bool,
}

struct NetworkTracker {
    networks: Networks,
    last_refresh: Instant,
}

struct DiskCache {
    disks: Disks,
    last_refresh: Instant,
    last_list_refresh: Instant,
    snapshot: Vec<DiskInfo>,
}

pub struct SystemState {
    sys: Mutex<System>,
    gpu: Mutex<GpuMonitor>,
    network: Mutex<NetworkTracker>,
    disks: Mutex<DiskCache>,
    cpu_name: String,
    cpu_cores: usize,
}

impl Default for SystemState {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemState {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let cpu_name = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default();
        let cpu_cores = sys.cpus().len();

        let disks = Disks::new_with_refreshed_list();
        let snapshot = build_disk_snapshot(&disks);
        let now = Instant::now();

        Self {
            sys: Mutex::new(sys),
            gpu: Mutex::new(GpuMonitor::new()),
            network: Mutex::new(NetworkTracker {
                networks: Networks::new_with_refreshed_list(),
                last_refresh: now,
            }),
            disks: Mutex::new(DiskCache {
                disks,
                last_refresh: now,
                last_list_refresh: now,
                snapshot,
            }),
            cpu_name,
            cpu_cores,
        }
    }

    pub fn sample(&self, options: &RefreshOptions) -> SystemResources {
        let (cpu_usage_percent, ram) = {
            let mut sys = lock(&self.sys);
            if options.cpu {
                sys.refresh_cpu_usage();
            }
            if options.ram {
                sys.refresh_memory();
            }
            let cpu = if options.cpu {
                round1(sys.global_cpu_usage() as f64)
            } else {
                0.0
            };
            let ram = options.ram.then(|| {
                let total = sys.total_memory() as f64 / GIB;
                let used = sys.used_memory() as f64 / GIB;
                let pct = if total > 0.0 { used / total * 100.0 } else { 0.0 };
                (
                    round1(used),
                    round1(total),
                    round1(pct),
                    round1(sys.used_swap() as f64 / GIB),
                    round1(sys.total_swap() as f64 / GIB),
                )
            });
            (cpu, ram)
        };
        let (ram_used_gb, ram_total_gb, ram_usage_percent, swap_used_gb, swap_total_gb) =
            ram.unwrap_or_default();

        let gpu = if options.gpu {
            lock(&self.gpu).sample()
        } else {
            None
        };

        let network = options.network.then(|| self.sample_network());
        let disks = if options.disk {
            let list = self.sample_disks();
            (!list.is_empty()).then_some(list)
        } else {
            None
        };

        SystemResources {
            cpu_usage_percent,
            cpu_cores: if options.cpu { self.cpu_cores } else { 0 },
            cpu_name: self.cpu_name.clone(),
            uptime_secs: System::uptime(),
            ram_used_gb,
            ram_total_gb,
            ram_usage_percent,
            swap_used_gb,
            swap_total_gb,
            gpu,
            network,
            disks,
        }
    }

    fn sample_network(&self) -> NetworkInfo {
        let mut tracker = lock(&self.network);
        tracker.networks.refresh();
        let elapsed = tracker.last_refresh.elapsed().as_secs_f64();
        tracker.last_refresh = Instant::now();

        let (mut down, mut up, mut total_rx, mut total_tx) = (0u64, 0u64, 0u64, 0u64);
        for data in tracker.networks.values() {
            down += data.received();
            up += data.transmitted();
            total_rx += data.total_received();
            total_tx += data.total_transmitted();
        }

        let rate = |bytes: u64| {
            if elapsed > 0.05 {
                round1(bytes as f64 / elapsed / 1024.0)
            } else {
                0.0
            }
        };

        NetworkInfo {
            download_kbps: rate(down),
            upload_kbps: rate(up),
            total_received_gb: round2(total_rx as f64 / GIB),
            total_transmitted_gb: round2(total_tx as f64 / GIB),
        }
    }

    fn sample_disks(&self) -> Vec<DiskInfo> {
        let mut cache = lock(&self.disks);
        if cache.last_list_refresh.elapsed() >= DISK_LIST_REFRESH {
            cache.disks.refresh_list();
            cache.last_list_refresh = Instant::now();
            cache.last_refresh = Instant::now();
            cache.snapshot = build_disk_snapshot(&cache.disks);
        } else if cache.last_refresh.elapsed() >= DISK_REFRESH {
            cache.disks.refresh();
            cache.last_refresh = Instant::now();
            cache.snapshot = build_disk_snapshot(&cache.disks);
        }
        cache.snapshot.clone()
    }

    /// Trim the working set of every accessible process. Returns a short Korean summary.
    pub fn clear_ram_cache(&self) -> String {
        let available_before = {
            let mut sys = lock(&self.sys);
            sys.refresh_memory();
            sys.available_memory()
        };

        let trimmed = trim_all_working_sets();

        // Give the memory manager a moment to account the freed pages.
        std::thread::sleep(Duration::from_millis(300));
        let available_after = {
            let mut sys = lock(&self.sys);
            sys.refresh_memory();
            sys.available_memory()
        };

        let freed_gb = available_after.saturating_sub(available_before) as f64 / GIB;
        if freed_gb >= 0.1 {
            format!("{}개 정리 · {:.1} GB 확보", trimmed, freed_gb)
        } else {
            format!("{}개 정리 완료", trimmed)
        }
    }
}

fn disk_display_name(mount: &str, label: &str) -> String {
    match (mount.is_empty(), label.is_empty()) {
        (false, false) => format!("{} ({})", mount, label),
        (false, true) => mount.to_string(),
        (true, false) => label.to_string(),
        (true, true) => "디스크".to_string(),
    }
}

fn build_disk_snapshot(disks: &Disks) -> Vec<DiskInfo> {
    disks
        .iter()
        .filter_map(|disk| {
            let total = disk.total_space() as f64 / GIB;
            if total < MIN_DISK_GB {
                return None;
            }
            let used = (total - disk.available_space() as f64 / GIB).max(0.0);
            let mount = disk.mount_point().to_string_lossy().to_string();
            let label = disk.name().to_string_lossy().trim().to_string();
            Some(DiskInfo {
                name: disk_display_name(&mount, &label),
                used_gb: round1(used),
                total_gb: round1(total),
                usage_percent: round1(used / total * 100.0),
            })
        })
        .collect()
}

fn trim_all_working_sets() -> usize {
    let mut pids = vec![0u32; 4096];
    let mut needed = 0u32;
    loop {
        let cb = (pids.len() * std::mem::size_of::<u32>()) as u32;
        if unsafe { EnumProcesses(pids.as_mut_ptr(), cb, &mut needed) }.is_err() {
            return 0;
        }
        // A full buffer may mean truncation: grow and retry.
        if needed < cb {
            pids.truncate(needed as usize / std::mem::size_of::<u32>());
            break;
        }
        pids.resize(pids.len() * 2, 0);
    }

    pids.into_iter()
        .filter(|&pid| pid != 0)
        .filter(|&pid| unsafe {
            match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, false, pid) {
                Ok(handle) => {
                    let ok = EmptyWorkingSet(handle).is_ok();
                    let _ = CloseHandle(handle);
                    ok
                }
                Err(_) => false,
            }
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_names() {
        assert_eq!(disk_display_name("C:\\", "Windows"), "C:\\ (Windows)");
        assert_eq!(disk_display_name("D:\\", ""), "D:\\");
        assert_eq!(disk_display_name("", "Data"), "Data");
        assert_eq!(disk_display_name("", ""), "디스크");
    }

    #[test]
    fn refresh_options_default_missing_fields() {
        let o: RefreshOptions = serde_json::from_str(r#"{"ram":true}"#).unwrap();
        assert!(o.ram && !o.cpu && !o.gpu && !o.network && !o.disk);
    }

    #[test]
    fn sample_respects_options() {
        let state = SystemState::new();
        let r = state.sample(&RefreshOptions {
            ram: true,
            ..Default::default()
        });
        assert!(r.ram_total_gb > 0.0);
        assert_eq!(r.cpu_cores, 0);
        assert!(r.network.is_none() && r.disks.is_none() && r.gpu.is_none());
    }

    #[test]
    fn disk_snapshot_finds_system_drive() {
        let state = SystemState::new();
        let r = state.sample(&RefreshOptions {
            disk: true,
            ..Default::default()
        });
        let disks = r.disks.expect("at least one disk");
        assert!(disks.iter().all(|d| !d.name.is_empty() && d.total_gb >= MIN_DISK_GB));
    }
}
