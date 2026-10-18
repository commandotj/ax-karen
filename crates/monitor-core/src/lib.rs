use std::io;
use std::path::{Path, PathBuf};

use monitor_disk::scan::{ScanHandle, ScanOptions, ScanSnapshot};
use monitor_disk::tree::Node;

pub mod disk_marks;
pub mod settings;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResourceSnapshot {
    pub cpu_percent: Option<u32>,
    pub cpu_processes: Option<Vec<monitor_cpu::ProcessUsage>>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub memory_processes: Option<Vec<monitor_memory::ProcessUsage>>,
    pub battery: monitor_battery::Snapshot,
    pub network: monitor_network::Snapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VolumeSnapshot {
    pub mount_point: String,
    pub device: Option<String>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
}

#[derive(Debug)]
pub struct MonitorCore {
    active_scan: Option<ScanHandle>,
    cpu: monitor_cpu::Sampler,
    memory: monitor_memory::Sampler,
    battery: monitor_battery::Sampler,
    network: monitor_network::Sampler,
    disk_marks: disk_marks::DiskMarks,
}

impl MonitorCore {
    pub fn new() -> Self {
        Self {
            active_scan: None,
            cpu: monitor_cpu::Sampler::new(),
            memory: monitor_memory::Sampler::default(),
            battery: monitor_battery::Sampler::new(),
            network: monitor_network::Sampler::new(),
            disk_marks: disk_marks::DiskMarks::default(),
        }
    }

    pub fn start_scan(&mut self, root: PathBuf, options: ScanOptions) -> bool {
        if self.active_scan.is_some() {
            return false;
        }
        self.active_scan = Some(ScanHandle::spawn(root, options));
        true
    }

    pub const fn disk_marks(&self) -> &disk_marks::DiskMarks {
        &self.disk_marks
    }

    pub const fn disk_marks_mut(&mut self) -> &mut disk_marks::DiskMarks {
        &mut self.disk_marks
    }

    pub fn scan_progress(&self) -> Option<ScanSnapshot> {
        self.active_scan
            .as_ref()
            .map(|scan| scan.progress.snapshot())
    }

    pub fn cancel_scan(&self) -> bool {
        let Some(scan) = self.active_scan.as_ref() else {
            return false;
        };
        scan.cancel();
        true
    }

    pub fn poll_scan(&mut self) -> Option<io::Result<Node>> {
        let scan = self.active_scan.as_ref()?;
        let result = scan.poll()?;
        self.active_scan = None;
        Some(result)
    }

    pub fn sample_resources(&mut self) -> ResourceSnapshot {
        let cpu = self.cpu.sample();
        let memory = self.memory.sample();
        let battery = self.battery.sample();
        let network = self.network.sample();
        ResourceSnapshot {
            cpu_percent: cpu.total_percent.map(|value| value.round() as u32),
            cpu_processes: cpu.processes,
            memory_used_bytes: memory.used_bytes,
            memory_total_bytes: memory.total_bytes,
            memory_processes: memory.processes,
            battery,
            network,
        }
    }

    pub fn sample_volumes(&self) -> Vec<VolumeSnapshot> {
        monitor_disk::space::volumes()
            .into_iter()
            .filter(|volume| is_dashboard_volume(&volume.point))
            .map(|volume| VolumeSnapshot {
                mount_point: volume.point.display().to_string(),
                device: volume.device,
                total_bytes: volume.space.map(|space| space.total),
                free_bytes: volume.space.map(|space| space.free),
                available_bytes: volume.space.map(|space| space.available),
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
fn is_dashboard_volume(point: &Path) -> bool {
    // macOS reports system and simulator mounts beside real disks. Keep the
    // boot disk and user volumes in the dashboard.
    point == Path::new("/") || point.starts_with("/Volumes")
}

#[cfg(not(target_os = "macos"))]
fn is_dashboard_volume(_point: &Path) -> bool {
    true
}

impl Default for MonitorCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::MonitorCore;
    use monitor_disk::scan::ScanOptions;

    #[test]
    fn idle_core_has_nothing_to_cancel_or_poll() {
        let mut core = MonitorCore::default();
        assert!(!core.cancel_scan());
        assert!(core.scan_progress().is_none());
        assert!(core.poll_scan().is_none());
        assert_eq!(core.disk_marks().items(std::path::Path::new("/")), []);
    }

    #[test]
    fn a_second_scan_is_refused_until_the_first_is_polled() {
        let root = tempfile::tempdir().expect("temp root");
        std::fs::write(root.path().join("a"), "a").expect("write");
        let mut core = MonitorCore::new();
        assert!(core.start_scan(root.path().into(), ScanOptions::default()));
        assert!(!core.start_scan(root.path().into(), ScanOptions::default()));
        assert!(core.cancel_scan());
        while core.poll_scan().is_none() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(core.start_scan(root.path().into(), ScanOptions::default()));
    }

    #[test]
    fn volumes_include_the_root_volume() {
        let volumes = MonitorCore::new().sample_volumes();
        assert!(volumes.iter().any(|volume| volume.mount_point == "/"));
    }
}
