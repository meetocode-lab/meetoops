#[cfg(windows)]
mod win_pdh {
    use std::ptr;
    use windows_sys::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue,
        PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE,
    };

    pub struct WinPdhSample {
        pub cpu_utility_percent: Option<f32>,
        pub disk_active_percent: Option<f32>,
        pub disk_read_bps: Option<f64>,
        pub disk_write_bps: Option<f64>,
    }

    /// Reads exact Windows Task Manager counters via native PDH APIs (`PdhAddEnglishCounterW`
    /// works identically on English, Indonesian, and all other Windows locales).
    pub struct WinPdhCollector {
        query: isize,
        cpu_counter: isize,
        disk_time_counter: isize,
        disk_read_counter: isize,
        disk_write_counter: isize,
    }

    // PDH query handles are safe to move across threads when guarded by a Mutex.
    unsafe impl Send for WinPdhCollector {}

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    impl WinPdhCollector {
        pub fn try_new() -> Option<Self> {
            unsafe {
                let mut query: isize = 0;
                if PdhOpenQueryW(ptr::null(), 0, &mut query) != 0 {
                    return None;
                }

                let mut cpu_counter: isize = 0;
                let cpu_path = to_wide(r"\Processor Information(_Total)\% Processor Utility");
                let _ = PdhAddEnglishCounterW(query, cpu_path.as_ptr(), 0, &mut cpu_counter);

                let mut disk_time_counter: isize = 0;
                let disk_time_path = to_wide(r"\PhysicalDisk(_Total)\% Disk Time");
                let _ = PdhAddEnglishCounterW(
                    query,
                    disk_time_path.as_ptr(),
                    0,
                    &mut disk_time_counter,
                );

                let mut disk_read_counter: isize = 0;
                let disk_read_path = to_wide(r"\PhysicalDisk(_Total)\Disk Read Bytes/sec");
                let _ = PdhAddEnglishCounterW(
                    query,
                    disk_read_path.as_ptr(),
                    0,
                    &mut disk_read_counter,
                );

                let mut disk_write_counter: isize = 0;
                let disk_write_path = to_wide(r"\PhysicalDisk(_Total)\Disk Write Bytes/sec");
                let _ = PdhAddEnglishCounterW(
                    query,
                    disk_write_path.as_ptr(),
                    0,
                    &mut disk_write_counter,
                );

                // Prime baseline sample (PDH rate counters require 2 collections)
                let _ = PdhCollectQueryData(query);

                Some(Self {
                    query,
                    cpu_counter,
                    disk_time_counter,
                    disk_read_counter,
                    disk_write_counter,
                })
            }
        }

        pub fn sample(&mut self) -> WinPdhSample {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return WinPdhSample {
                        cpu_utility_percent: None,
                        disk_active_percent: None,
                        disk_read_bps: None,
                        disk_write_bps: None,
                    };
                }

                let cpu_utility_percent = read_double(self.cpu_counter).map(|v| {
                    let clamped = v.clamp(0.0, 100.0);
                    ((clamped * 10.0).round() / 10.0) as f32
                });

                let disk_active_percent = read_double(self.disk_time_counter).map(|v| {
                    // Task Manager caps % Active time display at 100%
                    let clamped = v.clamp(0.0, 100.0);
                    ((clamped * 10.0).round() / 10.0) as f32
                });

                let disk_read_bps = read_double(self.disk_read_counter)
                    .map(|v| ((v.max(0.0)) * 10.0).round() / 10.0);
                let disk_write_bps = read_double(self.disk_write_counter)
                    .map(|v| ((v.max(0.0)) * 10.0).round() / 10.0);

                WinPdhSample {
                    cpu_utility_percent,
                    disk_active_percent,
                    disk_read_bps,
                    disk_write_bps,
                }
            }
        }
    }

    unsafe fn read_double(counter: isize) -> Option<f64> {
        if counter == 0 {
            return None;
        }
        let mut val: PDH_FMT_COUNTERVALUE = std::mem::zeroed();
        let status = PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, ptr::null_mut(), &mut val);
        if status == 0 {
            let v = val.Anonymous.doubleValue;
            if v.is_finite() {
                return Some(v);
            }
        }
        None
    }

    impl Drop for WinPdhCollector {
        fn drop(&mut self) {
            unsafe {
                if self.query != 0 {
                    let _ = PdhCloseQuery(self.query);
                }
            }
        }
    }
}

use serde::Serialize;
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

/// Real-time host telemetry snapshot. Missing or unavailable metrics are `None` (serialized as `null`), never `0`.
#[derive(Debug, Clone, Serialize)]
pub struct HostMetricsSnapshot {
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub kernel_version: Option<String>,
    pub cpu_cores: usize,
    pub cpu_usage_percent: Option<f32>,
    pub memory_total_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_usage_percent: Option<f32>,
    pub disk_total_bytes: Option<u64>,
    pub disk_used_bytes: Option<u64>,
    pub disk_usage_percent: Option<f32>,
    pub disk_active_percent: Option<f32>,
    pub disk_read_bps: Option<f64>,
    pub disk_write_bps: Option<f64>,
    pub net_rx_bps: Option<f64>,
    pub net_tx_bps: Option<f64>,
    pub uptime_seconds: Option<u64>,
}

/// Stateful host collector that accurately tracks CPU usage (matching Windows Task Manager `% Processor Utility`
/// via PDH on Windows), Disk capacity + Disk Active Time / Read-Write throughput, and Network RX/TX deltas.
pub struct HostMetricsCollector {
    sys: System,
    networks: Networks,
    last_sample: Option<Instant>,
    #[cfg(windows)]
    pdh: Option<win_pdh::WinPdhCollector>,
}

impl Default for HostMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl HostMetricsCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let networks = Networks::new_with_refreshed_list();
        Self {
            sys,
            networks,
            last_sample: Some(Instant::now()),
            #[cfg(windows)]
            pdh: win_pdh::WinPdhCollector::try_new(),
        }
    }

    pub fn sample(&mut self) -> HostMetricsSnapshot {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.networks.refresh(true);

        let now = Instant::now();
        let elapsed_secs = self
            .last_sample
            .map(|prev| now.duration_since(prev).as_secs_f64())
            .unwrap_or(0.0);
        self.last_sample = Some(now);

        #[cfg(windows)]
        let pdh_sample = self.pdh.as_mut().map(|p| p.sample());

        let cpu_cores = self.sys.cpus().len();
        let sysinfo_cpu = if cpu_cores > 0 {
            let avg = self.sys.global_cpu_usage();
            if avg.is_finite() {
                Some((avg * 10.0).round() / 10.0)
            } else {
                None
            }
        } else {
            None
        };

        #[cfg(windows)]
        let cpu_usage_percent = pdh_sample
            .as_ref()
            .and_then(|s| s.cpu_utility_percent)
            .or(sysinfo_cpu);
        #[cfg(not(windows))]
        let cpu_usage_percent = sysinfo_cpu;

        #[cfg(windows)]
        let (disk_active_percent, disk_read_bps, disk_write_bps) = match pdh_sample {
            Some(s) => (s.disk_active_percent, s.disk_read_bps, s.disk_write_bps),
            None => (None, None, None),
        };
        #[cfg(not(windows))]
        let (disk_active_percent, disk_read_bps, disk_write_bps) = (None, None, None);

        let mem_total = self.sys.total_memory();
        let mem_used = self.sys.used_memory();
        let (memory_total_bytes, memory_used_bytes, memory_usage_percent) = if mem_total > 0 {
            let pct = ((mem_used as f64 / mem_total as f64) * 1000.0).round() as f32 / 10.0;
            (Some(mem_total), Some(mem_used), Some(pct))
        } else {
            (None, None, None)
        };

        let disks = Disks::new_with_refreshed_list();
        let mut disk_total: u64 = 0;
        let mut disk_available: u64 = 0;
        for disk in disks.list() {
            disk_total = disk_total.saturating_add(disk.total_space());
            disk_available = disk_available.saturating_add(disk.available_space());
        }
        let (disk_total_bytes, disk_used_bytes, disk_usage_percent) = if disk_total > 0 {
            let used = disk_total.saturating_sub(disk_available);
            let pct = ((used as f64 / disk_total as f64) * 1000.0).round() as f32 / 10.0;
            (Some(disk_total), Some(used), Some(pct))
        } else {
            (None, None, None)
        };

        let (net_rx_bps, net_tx_bps) = if elapsed_secs > 0.05 {
            let mut rx: u64 = 0;
            let mut tx: u64 = 0;
            for (_name, data) in self.networks.list() {
                rx = rx.saturating_add(data.received());
                tx = tx.saturating_add(data.transmitted());
            }
            (
                Some(((rx as f64 / elapsed_secs) * 10.0).round() / 10.0),
                Some(((tx as f64 / elapsed_secs) * 10.0).round() / 10.0),
            )
        } else {
            (None, None)
        };

        let uptime = System::uptime();
        let uptime_seconds = if uptime > 0 { Some(uptime) } else { None };

        HostMetricsSnapshot {
            hostname: System::host_name(),
            os_name: System::long_os_version(),
            kernel_version: System::kernel_version(),
            cpu_cores,
            cpu_usage_percent,
            memory_total_bytes,
            memory_used_bytes,
            memory_usage_percent,
            disk_total_bytes,
            disk_used_bytes,
            disk_usage_percent,
            disk_active_percent,
            disk_read_bps,
            disk_write_bps,
            net_rx_bps,
            net_tx_bps,
            uptime_seconds,
        }
    }
}

/// One-shot host snapshot (convenience helper for synchronous endpoints).
pub fn collect_host_snapshot() -> HostMetricsSnapshot {
    let mut collector = HostMetricsCollector::new();
    std::thread::sleep(std::time::Duration::from_millis(200));
    collector.sample()
}

/// Renders real host telemetry in Prometheus text exposition format.
/// Unavailable (`None`) metrics are omitted rather than emitted as fake `0`s.
pub fn render_prometheus_metrics(snap: &HostMetricsSnapshot) -> String {
    let mut out = String::new();
    if let Some(cpu) = snap.cpu_usage_percent {
        out.push_str("# HELP meetoops_host_cpu_usage_percent Host CPU utilization percentage\n");
        out.push_str("# TYPE meetoops_host_cpu_usage_percent gauge\n");
        out.push_str(&format!("meetoops_host_cpu_usage_percent {cpu}\n"));
    }
    if let Some(mem_used) = snap.memory_used_bytes {
        out.push_str("# HELP meetoops_host_memory_used_bytes Host used memory in bytes\n");
        out.push_str("# TYPE meetoops_host_memory_used_bytes gauge\n");
        out.push_str(&format!("meetoops_host_memory_used_bytes {mem_used}\n"));
    }
    if let Some(mem_total) = snap.memory_total_bytes {
        out.push_str("# HELP meetoops_host_memory_total_bytes Host total memory in bytes\n");
        out.push_str("# TYPE meetoops_host_memory_total_bytes gauge\n");
        out.push_str(&format!("meetoops_host_memory_total_bytes {mem_total}\n"));
    }
    if let Some(disk_used) = snap.disk_used_bytes {
        out.push_str("# HELP meetoops_host_disk_used_bytes Host used disk space in bytes\n");
        out.push_str("# TYPE meetoops_host_disk_used_bytes gauge\n");
        out.push_str(&format!("meetoops_host_disk_used_bytes {disk_used}\n"));
    }
    if let Some(disk_act) = snap.disk_active_percent {
        out.push_str("# HELP meetoops_host_disk_active_percent Host physical disk active I/O time percentage\n");
        out.push_str("# TYPE meetoops_host_disk_active_percent gauge\n");
        out.push_str(&format!("meetoops_host_disk_active_percent {disk_act}\n"));
    }
    if let Some(dr) = snap.disk_read_bps {
        out.push_str("# HELP meetoops_host_disk_read_bytes_per_sec Host disk read throughput\n");
        out.push_str("# TYPE meetoops_host_disk_read_bytes_per_sec gauge\n");
        out.push_str(&format!("meetoops_host_disk_read_bytes_per_sec {dr}\n"));
    }
    if let Some(dw) = snap.disk_write_bps {
        out.push_str("# HELP meetoops_host_disk_write_bytes_per_sec Host disk write throughput\n");
        out.push_str("# TYPE meetoops_host_disk_write_bytes_per_sec gauge\n");
        out.push_str(&format!("meetoops_host_disk_write_bytes_per_sec {dw}\n"));
    }
    if let Some(rx) = snap.net_rx_bps {
        out.push_str("# HELP meetoops_host_network_rx_bytes_per_sec Host network receive rate\n");
        out.push_str("# TYPE meetoops_host_network_rx_bytes_per_sec gauge\n");
        out.push_str(&format!("meetoops_host_network_rx_bytes_per_sec {rx}\n"));
    }
    if let Some(tx) = snap.net_tx_bps {
        out.push_str("# HELP meetoops_host_network_tx_bytes_per_sec Host network transmit rate\n");
        out.push_str("# TYPE meetoops_host_network_tx_bytes_per_sec gauge\n");
        out.push_str(&format!("meetoops_host_network_tx_bytes_per_sec {tx}\n"));
    }
    if let Some(uptime) = snap.uptime_seconds {
        out.push_str("# HELP meetoops_host_uptime_seconds Host uptime in seconds\n");
        out.push_str("# TYPE meetoops_host_uptime_seconds counter\n");
        out.push_str(&format!("meetoops_host_uptime_seconds {uptime}\n"));
    }
    out
}
