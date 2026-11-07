//! GPU telemetry.
//!
//! * NVIDIA: NVML (utilization, VRAM, temperature, power).
//! * AMD / Intel / anything else: adapter picked via DXGI (name, real 64-bit VRAM size, LUID)
//!   plus live utilization & memory from the Windows "GPU Engine" / "GPU Adapter Memory"
//!   performance counters — the same source Task Manager uses.

use std::collections::HashMap;

use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;
use serde::{Deserialize, Serialize};
use windows::core::{w, PCWSTR};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W,
    PDH_FMT_DOUBLE, PDH_MORE_DATA,
};

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
/// Microsoft Basic Render Driver.
const VENDOR_MICROSOFT: u32 = 0x1414;
/// Below this much dedicated VRAM an adapter is treated as integrated (uses shared memory).
const INTEGRATED_THRESHOLD: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub memory_used_gb: Option<f64>,
    pub memory_total_gb: Option<f64>,
    pub memory_usage_percent: Option<f64>,
    pub utilization_percent: Option<f64>,
    pub temperature_c: Option<u32>,
    pub power_w: Option<f64>,
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

pub struct GpuMonitor {
    backend: Backend,
}

enum Backend {
    // Boxed: `Nvml` embeds its whole function table (~10 KB).
    Nvml(Box<Nvml>),
    Generic {
        adapter: Adapter,
        counters: Option<PdhGpuCounters>,
    },
    None,
}

impl Default for GpuMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuMonitor {
    pub fn new() -> Self {
        if let Ok(nvml) = Nvml::init() {
            if nvml.device_count().map(|c| c > 0).unwrap_or(false) {
                return Self {
                    backend: Backend::Nvml(Box::new(nvml)),
                };
            }
        }
        match pick_dxgi_adapter() {
            Some(adapter) => Self {
                backend: Backend::Generic {
                    adapter,
                    counters: PdhGpuCounters::open(),
                },
            },
            None => Self {
                backend: Backend::None,
            },
        }
    }

    pub fn sample(&mut self) -> Option<GpuInfo> {
        match &self.backend {
            Backend::Nvml(nvml) => sample_nvml(nvml),
            Backend::Generic { adapter, counters } => Some(sample_generic(adapter, counters.as_ref())),
            Backend::None => None,
        }
    }
}

// ───────────────────────────── NVIDIA ─────────────────────────────

fn sample_nvml(nvml: &Nvml) -> Option<GpuInfo> {
    let device = nvml.device_by_index(0).ok()?;
    let name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());

    let (used, total, pct) = match device.memory_info() {
        Ok(mem) => {
            let total = mem.total as f64 / GIB;
            let used = mem.used as f64 / GIB;
            let pct = if total > 0.0 { used / total * 100.0 } else { 0.0 };
            (Some(round1(used)), Some(round1(total)), Some(round1(pct)))
        }
        Err(_) => (None, None, None),
    };

    Some(GpuInfo {
        name,
        memory_used_gb: used,
        memory_total_gb: total,
        memory_usage_percent: pct,
        utilization_percent: device.utilization_rates().ok().map(|u| u.gpu as f64),
        temperature_c: device.temperature(TemperatureSensor::Gpu).ok(),
        power_w: device.power_usage().ok().map(|mw| round1(mw as f64 / 1000.0)),
    })
}

// ───────────────────────────── DXGI + PDH ─────────────────────────────

struct Adapter {
    name: String,
    /// Bytes of memory considered "VRAM" for this adapter.
    memory_total: u64,
    /// Read the "Shared Usage" counter instead of "Dedicated Usage" (integrated GPUs).
    uses_shared_memory: bool,
    /// e.g. `luid_0x00000000_0x0000d1b5`, matches PDH instance names (lower-case).
    luid_tag: String,
}

fn pick_dxgi_adapter() -> Option<Adapter> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    let mut best: Option<(u64, Adapter)> = None;

    for i in 0.. {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(i) }) else {
            break;
        };
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        if desc.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 || desc.VendorId == VENDOR_MICROSOFT {
            continue;
        }

        let name_len = desc.Description.iter().position(|&c| c == 0).unwrap_or(desc.Description.len());
        let name = String::from_utf16_lossy(&desc.Description[..name_len]).trim().to_string();
        let dedicated = desc.DedicatedVideoMemory as u64;
        let integrated = dedicated < INTEGRATED_THRESHOLD;
        let luid_tag = format!(
            "luid_0x{:08x}_0x{:08x}",
            desc.AdapterLuid.HighPart as u32, desc.AdapterLuid.LowPart
        );

        let candidate = Adapter {
            name,
            memory_total: if integrated { desc.SharedSystemMemory as u64 } else { dedicated },
            uses_shared_memory: integrated,
            luid_tag,
        };
        // Prefer the adapter with the most dedicated memory (the discrete GPU).
        if best.as_ref().map_or(true, |(d, _)| dedicated > *d) {
            best = Some((dedicated, candidate));
        }
    }
    best.map(|(_, a)| a)
}

fn sample_generic(adapter: &Adapter, counters: Option<&PdhGpuCounters>) -> GpuInfo {
    let total_gb = (adapter.memory_total > 0).then(|| adapter.memory_total as f64 / GIB);

    let (utilization, used_bytes) = match counters {
        Some(c) if c.collect() => {
            let engines = c.read(c.engine);
            let mem = c.read(if adapter.uses_shared_memory { c.shared } else { c.dedicated });
            (
                engine_utilization(&engines, &adapter.luid_tag),
                adapter_memory(&mem, &adapter.luid_tag),
            )
        }
        _ => (None, None),
    };


    let used_gb = used_bytes.map(|b| b / GIB);
    let pct = match (used_gb, total_gb) {
        (Some(u), Some(t)) if t > 0.0 => Some(round1((u / t * 100.0).min(100.0))),
        _ => None,
    };

    GpuInfo {
        name: adapter.name.clone(),
        memory_used_gb: used_gb.map(round1),
        memory_total_gb: total_gb.map(round1),
        memory_usage_percent: pct,
        utilization_percent: utilization.map(round1),
        temperature_c: None,
        power_w: None,
    }
}

/// Persistent PDH query. Rate counters (utilization) need two collections, so the query
/// lives across polls; the first sample after start-up reports no utilization.
struct PdhGpuCounters {
    query: isize,
    engine: isize,
    dedicated: isize,
    shared: isize,
}

// Raw PDH handles are plain integers with no thread affinity.
unsafe impl Send for PdhGpuCounters {}

impl PdhGpuCounters {
    fn open() -> Option<Self> {
        unsafe {
            let mut query = 0isize;
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let add = |path: PCWSTR| {
                let mut counter = 0isize;
                (PdhAddEnglishCounterW(query, path, 0, &mut counter) == 0).then_some(counter)
            };
            let engine = add(w!("\\GPU Engine(*)\\Utilization Percentage"));
            let dedicated = add(w!("\\GPU Adapter Memory(*)\\Dedicated Usage"));
            let shared = add(w!("\\GPU Adapter Memory(*)\\Shared Usage"));

            match (engine, dedicated, shared) {
                (Some(engine), Some(dedicated), Some(shared)) => {
                    PdhCollectQueryData(query); // prime the rate counters
                    Some(Self {
                        query,
                        engine,
                        dedicated,
                        shared,
                    })
                }
                _ => {
                    PdhCloseQuery(query);
                    None
                }
            }
        }
    }

    fn collect(&self) -> bool {
        unsafe { PdhCollectQueryData(self.query) == 0 }
    }

    /// All (instance name, value) pairs of a wildcard counter.
    fn read(&self, counter: isize) -> Vec<(String, f64)> {
        unsafe {
            let mut size = 0u32;
            let mut count = 0u32;
            let status =
                PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None);
            if status != PDH_MORE_DATA || size == 0 {
                return Vec::new();
            }
            // u64 backing store keeps the f64 values inside the items 8-byte aligned.
            let mut buf = vec![0u64; (size as usize).div_ceil(8)];
            let items_ptr = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                Some(items_ptr),
            ) != 0
            {
                return Vec::new();
            }
            std::slice::from_raw_parts(items_ptr, count as usize)
                .iter()
                .filter(|it| {
                    it.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA
                        || it.FmtValue.CStatus == PDH_CSTATUS_NEW_DATA
                })
                .filter_map(|it| {
                    let name = it.szName.to_string().ok()?;
                    Some((name, it.FmtValue.Anonymous.doubleValue))
                })
                .collect()
        }
    }
}

impl Drop for PdhGpuCounters {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.query);
        }
    }
}

// ───────────────────────────── Pure aggregation ─────────────────────────────

/// GPU utilization the way Task Manager computes it: sum per-process usage for each physical
/// engine (instance name minus its `pid_N_` prefix), then take the busiest engine.
/// Instance format: `pid_1234_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D`.
pub fn engine_utilization(samples: &[(String, f64)], luid_tag: &str) -> Option<f64> {
    let mut per_engine: HashMap<String, f64> = HashMap::new();
    for (instance, value) in samples {
        let lower = instance.to_ascii_lowercase();
        let Some(idx) = lower.find("luid_") else { continue };
        let engine = &lower[idx..];
        if engine.starts_with(luid_tag) {
            *per_engine.entry(engine.to_string()).or_default() += value.max(0.0);
        }
    }
    per_engine
        .into_values()
        .reduce(f64::max)
        .map(|v| v.clamp(0.0, 100.0))
}

/// Memory in bytes used on the adapter (summed over its physical instances).
/// Instance format: `luid_0x00000000_0x0000D1B5_phys_0`.
pub fn adapter_memory(samples: &[(String, f64)], luid_tag: &str) -> Option<f64> {
    let mut found = false;
    let mut total = 0.0;
    for (instance, value) in samples {
        if instance.to_ascii_lowercase().starts_with(luid_tag) {
            found = true;
            total += value.max(0.0);
        }
    }
    found.then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LUID: &str = "luid_0x00000000_0x0000d1b5";

    fn s(name: &str, v: f64) -> (String, f64) {
        (name.to_string(), v)
    }

    #[test]
    fn utilization_sums_processes_per_engine_and_takes_max() {
        let samples = [
            s("pid_10_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D", 20.0),
            s("pid_22_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D", 15.0),
            s("pid_10_luid_0x00000000_0x0000D1B5_phys_0_eng_3_engtype_VideoDecode", 30.0),
            // Other adapter: ignored.
            s("pid_10_luid_0x00000000_0x0000AAAA_phys_0_eng_0_engtype_3D", 99.0),
        ];
        assert_eq!(engine_utilization(&samples, LUID), Some(35.0));
    }

    #[test]
    fn utilization_is_capped_and_none_without_matches() {
        let samples = [
            s("pid_1_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D", 80.0),
            s("pid_2_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D", 70.0),
        ];
        assert_eq!(engine_utilization(&samples, LUID), Some(100.0));
        assert_eq!(engine_utilization(&samples, "luid_0x00000000_0x00000001"), None);
    }

    /// Hardware diagnostic: `cargo test -- --ignored --nocapture gpu_live_sample`
    #[test]
    #[ignore]
    fn gpu_live_sample() {
        let mut monitor = GpuMonitor::new();
        if let Backend::Generic { adapter, counters } = &monitor.backend {
            println!("DXGI adapter: {} ({}), pdh={}", adapter.name, adapter.luid_tag, counters.is_some());
        }
        let _ = monitor.sample();
        std::thread::sleep(std::time::Duration::from_millis(1000));
        println!("{:#?}", monitor.sample());
        // Also exercise the generic path on NVIDIA machines.
        if let (Some(adapter), Some(c)) = (pick_dxgi_adapter(), PdhGpuCounters::open()) {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            println!("generic path: {:#?}", sample_generic(&adapter, Some(&c)));
        }
    }

    #[test]
    fn memory_sums_matching_adapter_only() {
        let samples = [
            s("luid_0x00000000_0x0000D1B5_phys_0", 1024.0),
            s("luid_0x00000000_0x0000D1B5_phys_1", 512.0),
            s("luid_0x00000000_0x0000AAAA_phys_0", 9999.0),
        ];
        assert_eq!(adapter_memory(&samples, LUID), Some(1536.0));
        assert_eq!(adapter_memory(&[], LUID), None);
    }
}
