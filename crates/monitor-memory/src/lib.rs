use std::ffi::OsStr;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

const MAX_PROCESS_ROWS: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    /// Resident set on Unix; working set on Windows, in bytes.
    pub memory_bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub used_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub processes: Option<Vec<ProcessUsage>>,
}

#[derive(Debug, Default)]
pub struct Sampler {
    system: System,
}

impl Sampler {
    pub fn sample(&mut self) -> Snapshot {
        self.system.refresh_memory();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            ProcessRefreshKind::new().with_memory(),
        );
        let mut snapshot =
            snapshot(self.system.total_memory(), self.system.used_memory());
        snapshot.processes = Some(process_rows(&self.system));
        snapshot
    }
}

fn process_rows(system: &System) -> Vec<ProcessUsage> {
    let rows: Vec<_> = system
        .processes()
        .iter()
        .map(|(pid, process)| ProcessUsage {
            pid: pid.as_u32(),
            name: display_name(process.name()),
            memory_bytes: process.memory(),
        })
        .collect();
    rank_and_bound(rows)
}

fn rank_and_bound(mut rows: Vec<ProcessUsage>) -> Vec<ProcessUsage> {
    // sysinfo uses zero when per-process memory cannot be read. Excluding
    // zero avoids presenting an unknown reading as a real process value.
    rows.retain(|row| row.memory_bytes > 0);
    rows.sort_by(|left, right| {
        right
            .memory_bytes
            .cmp(&left.memory_bytes)
            .then_with(|| left.pid.cmp(&right.pid))
    });
    rows.truncate(MAX_PROCESS_ROWS);
    rows
}

fn display_name(name: &OsStr) -> String {
    let name = name.to_string_lossy();
    if name.trim().is_empty() {
        "Unknown process".to_owned()
    } else {
        name.into_owned()
    }
}

fn snapshot(total_bytes: u64, used_bytes: u64) -> Snapshot {
    if total_bytes == 0 {
        return Snapshot::default();
    }
    Snapshot {
        used_bytes: Some(used_bytes),
        total_bytes: Some(total_bytes),
        processes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_PROCESS_ROWS, ProcessUsage, Snapshot, display_name, rank_and_bound,
        snapshot,
    };
    use std::ffi::OsStr;

    #[test]
    fn zero_total_reports_memory_as_unavailable() {
        assert_eq!(snapshot(0, 0), Snapshot::default());
    }

    #[test]
    fn valid_total_preserves_used_and_total_bytes() {
        assert_eq!(
            snapshot(16_000, 6_000),
            Snapshot {
                used_bytes: Some(6_000),
                total_bytes: Some(16_000),
                processes: None
            },
        );
    }

    #[test]
    fn process_rows_sort_by_memory_then_pid_and_stay_bounded() {
        let rows: Vec<_> = (0_u8..23)
            .map(|pid| ProcessUsage {
                pid: u32::from(pid),
                name: format!("process-{pid}"),
                memory_bytes: if pid < 2 { 40 } else { u64::from(pid) },
            })
            .collect();
        let rows = rank_and_bound(rows);

        assert_eq!(rows.len(), MAX_PROCESS_ROWS);
        assert_eq!((rows[0].pid, rows[1].pid), (0, 1));
        assert_eq!(rows[2].memory_bytes, 22);
        assert_eq!(rows[MAX_PROCESS_ROWS - 1].memory_bytes, 5);
    }

    #[test]
    fn empty_process_name_uses_safe_fallback() {
        assert_eq!(display_name(OsStr::new(" ")), "Unknown process");
    }

    #[test]
    fn unreadable_zero_process_memory_is_not_presented_as_a_measurement() {
        let rows = rank_and_bound(vec![ProcessUsage {
            pid: 7,
            name: "restricted".to_owned(),
            memory_bytes: 0,
        }]);

        assert_eq!(rows.len(), 0);
    }
}
