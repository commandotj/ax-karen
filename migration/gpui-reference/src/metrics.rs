//! Read-only CPU and memory samples for the resource panel.

use sysinfo::System;

/// One system sample; CPU stays absent until its delta has warmed up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub cpu_percent: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
}

/// Persistent collector, since CPU utilization needs two readings.
#[derive(Debug)]
pub struct Sampler {
    system: System,
    cpu_warmed: bool,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            system: System::new(),
            cpu_warmed: false,
        }
    }

    /// Refresh only host CPU and RAM counters; never enumerate processes.
    pub fn sample(&mut self) -> Snapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        let cpu_percent = if self.cpu_warmed {
            let usage = self.system.global_cpu_usage();
            usage.is_finite().then(|| usage.clamp(0.0, 100.0))
        } else {
            None
        };
        self.cpu_warmed = true;

        let memory_total = self.system.total_memory();
        let (memory_used_bytes, memory_total_bytes) = if memory_total == 0 {
            (None, None)
        } else {
            (Some(self.system.used_memory()), Some(memory_total))
        };

        Snapshot {
            cpu_percent,
            memory_used_bytes,
            memory_total_bytes,
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Snapshot {
    pub fn cpu_fraction(self) -> f32 {
        self.cpu_percent
            .filter(|usage| usage.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, 100.0)
            / 100.0
    }

    pub fn memory_fraction(self) -> f32 {
        match (self.memory_used_bytes, self.memory_total_bytes) {
            (Some(used), Some(total)) if total > 0 => {
                (used as f64 / total as f64).clamp(0.0, 1.0) as f32
            }
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Snapshot;

    #[test]
    fn missing_cpu_sample_draws_an_empty_meter() {
        assert!(Snapshot::default().cpu_fraction().abs() < f32::EPSILON);
    }

    #[test]
    fn cpu_meter_is_bounded_to_hundred_percent() {
        let sample = Snapshot {
            cpu_percent: Some(125.0),
            ..Snapshot::default()
        };
        assert!((sample.cpu_fraction() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn memory_fraction_handles_missing_and_full_samples() {
        let sample = Snapshot {
            memory_used_bytes: Some(6),
            memory_total_bytes: Some(8),
            ..Snapshot::default()
        };
        assert!(Snapshot::default().memory_fraction().abs() < f32::EPSILON);
        assert!((sample.memory_fraction() - 0.75).abs() < f32::EPSILON);
    }
}
