use std::ffi::OsStr;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

const MAX_PROCESS_ROWS: usize = 20;

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub total_percent: Option<f32>,
    pub processes: Option<Vec<ProcessUsage>>,
}

#[derive(Debug)]
pub struct Sampler {
    system: System,
    warmed: bool,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            warmed: false,
        }
    }

    pub fn sample(&mut self) -> Snapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            ProcessRefreshKind::new().with_cpu(),
        );

        let total_percent = self
            .warmed
            .then(|| self.system.global_cpu_usage())
            .and_then(valid_total_percent);
        let processes = self.warmed.then(|| process_rows(&self.system));
        self.warmed = true;

        Snapshot {
            total_percent,
            processes,
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

fn valid_total_percent(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 100.0))
}

fn process_rows(system: &System) -> Vec<ProcessUsage> {
    let rows = system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let cpu_percent = valid_process_percent(process.cpu_usage())?;
            Some(ProcessUsage {
                pid: pid.as_u32(),
                name: display_name(process.name()),
                cpu_percent,
            })
        })
        .collect();

    rank_and_bound(rows)
}

/// A process reading that is a measurement: finite and not negative.
fn valid_process_percent(value: f32) -> Option<f32> {
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn display_name(name: &OsStr) -> String {
    let name = name.to_string_lossy();
    if name.trim().is_empty() {
        "Unknown process".to_owned()
    } else {
        name.into_owned()
    }
}

fn rank_and_bound(mut rows: Vec<ProcessUsage>) -> Vec<ProcessUsage> {
    rows.sort_by(|left, right| {
        right
            .cpu_percent
            .total_cmp(&left.cpu_percent)
            .then_with(|| left.pid.cmp(&right.pid))
    });
    rows.truncate(MAX_PROCESS_ROWS);
    rows
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::{
        MAX_PROCESS_ROWS, ProcessUsage, Sampler, display_name, rank_and_bound,
        valid_process_percent, valid_total_percent,
    };

    #[test]
    fn process_rows_sort_by_cpu_then_pid_and_stay_bounded() {
        let rows: Vec<_> = (0_u8..23)
            .map(|pid| ProcessUsage {
                pid: u32::from(pid),
                name: format!("process-{pid}"),
                cpu_percent: if pid < 2 { 40.0 } else { f32::from(pid) },
            })
            .collect();

        let rows = rank_and_bound(rows);

        assert_eq!(rows.len(), MAX_PROCESS_ROWS);
        assert_eq!(rows[0].pid, 0);
        assert_eq!(rows[1].pid, 1);
        assert!((rows[2].cpu_percent - 22.0).abs() < f32::EPSILON);
        assert!((rows[MAX_PROCESS_ROWS - 1].cpu_percent - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_process_name_uses_safe_fallback() {
        assert_eq!(display_name(OsStr::new(" ")), "Unknown process");
    }

    #[test]
    fn sampler_marks_first_process_sample_unavailable() {
        let first = Sampler::new().sample();

        assert_eq!(first.total_percent, None);
        assert_eq!(first.processes, None);
    }

    #[test]
    fn sampler_reports_finite_process_snapshot_after_warmup() {
        let mut sampler = Sampler::new();
        let _ = sampler.sample();
        let second = sampler.sample();

        assert!(second.total_percent.is_some_and(f32::is_finite));
        assert!(second.processes.is_some());
        assert!(
            second
                .processes
                .as_ref()
                .is_some_and(|rows| rows.len() <= MAX_PROCESS_ROWS)
        );
    }

    #[test]
    fn unmeasurable_readings_are_dropped_and_totals_clamped() {
        assert_eq!(valid_process_percent(f32::NAN), None);
        assert_eq!(valid_process_percent(-1.0), None);
        assert_eq!(valid_process_percent(0.0), Some(0.0));
        assert_eq!(valid_total_percent(f32::INFINITY), None);
        assert_eq!(valid_total_percent(140.0), Some(100.0));
        assert_eq!(display_name(OsStr::new("karen")), "karen");
    }

    #[test]
    fn the_default_sampler_starts_cold() {
        assert_eq!(Sampler::default().sample().processes, None);
    }
}
