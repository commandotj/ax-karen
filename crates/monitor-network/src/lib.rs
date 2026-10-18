use std::time::{Duration, Instant};

use sysinfo::Networks;

pub const MAX_INTERFACE_ROWS: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceUsage {
    pub name: String,
    pub rx_bytes_per_sec: Option<u64>,
    pub tx_bytes_per_sec: Option<u64>,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub packets_rx: u64,
    pub packets_tx: u64,
    pub errors_rx: u64,
    pub errors_tx: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub rx_bytes_per_sec: Option<u64>,
    pub tx_bytes_per_sec: Option<u64>,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub interfaces: Vec<InterfaceUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawInterfaceCounter {
    pub name: String,
    pub delta_rx_bytes: u64,
    pub delta_tx_bytes: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub packets_rx: u64,
    pub packets_tx: u64,
    pub errors_rx: u64,
    pub errors_tx: u64,
}

#[derive(Debug)]
pub struct Sampler {
    networks: Networks,
    last_sample_at: Option<Instant>,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            networks: Networks::new(),
            last_sample_at: None,
        }
    }

    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let elapsed = if let Some(previous) = self.last_sample_at {
            self.networks.refresh();
            Some(now.saturating_duration_since(previous))
        } else {
            self.networks.refresh_list();
            None
        };
        self.last_sample_at = Some(now);

        let raw: Vec<RawInterfaceCounter> = self
            .networks
            .iter()
            .map(|(name, data)| RawInterfaceCounter {
                name: name.clone(),
                delta_rx_bytes: data.received(),
                delta_tx_bytes: data.transmitted(),
                total_rx_bytes: data.total_received(),
                total_tx_bytes: data.total_transmitted(),
                packets_rx: data.total_packets_received(),
                packets_tx: data.total_packets_transmitted(),
                errors_rx: data.total_errors_on_received(),
                errors_tx: data.total_errors_on_transmitted(),
            })
            .collect();

        build_snapshot(&raw, elapsed)
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_loopback(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "lo" || lower == "lo0" || lower.starts_with("loopback")
}

pub fn rate_bytes_per_sec(
    delta_bytes: u64,
    elapsed: Option<Duration>,
) -> Option<u64> {
    let secs = elapsed?.as_secs_f64();
    if !secs.is_finite() || secs <= 0.0 {
        return None;
    }
    let rate = (delta_bytes as f64) / secs;
    (rate.is_finite() && rate >= 0.0).then(|| rate.round() as u64)
}

pub fn build_snapshot(
    counters: &[RawInterfaceCounter],
    elapsed: Option<Duration>,
) -> Snapshot {
    let mut external_delta_rx: u64 = 0;
    let mut external_delta_tx: u64 = 0;
    let mut total_rx_bytes: u64 = 0;
    let mut total_tx_bytes: u64 = 0;
    let mut rows = Vec::with_capacity(counters.len());

    for item in counters {
        if !is_loopback(&item.name) {
            external_delta_rx =
                external_delta_rx.saturating_add(item.delta_rx_bytes);
            external_delta_tx =
                external_delta_tx.saturating_add(item.delta_tx_bytes);
            total_rx_bytes = total_rx_bytes.saturating_add(item.total_rx_bytes);
            total_tx_bytes = total_tx_bytes.saturating_add(item.total_tx_bytes);
        }
        rows.push(InterfaceUsage {
            name: if item.name.trim().is_empty() {
                "unknown".to_owned()
            } else {
                item.name.clone()
            },
            rx_bytes_per_sec: rate_bytes_per_sec(item.delta_rx_bytes, elapsed),
            tx_bytes_per_sec: rate_bytes_per_sec(item.delta_tx_bytes, elapsed),
            total_rx_bytes: item.total_rx_bytes,
            total_tx_bytes: item.total_tx_bytes,
            packets_rx: item.packets_rx,
            packets_tx: item.packets_tx,
            errors_rx: item.errors_rx,
            errors_tx: item.errors_tx,
        });
    }

    rank_and_bound(&mut rows);

    Snapshot {
        rx_bytes_per_sec: rate_bytes_per_sec(external_delta_rx, elapsed),
        tx_bytes_per_sec: rate_bytes_per_sec(external_delta_tx, elapsed),
        total_rx_bytes,
        total_tx_bytes,
        interfaces: rows,
    }
}

pub fn rank_and_bound(rows: &mut Vec<InterfaceUsage>) {
    rows.sort_by(|left, right| {
        let left_rate = left
            .rx_bytes_per_sec
            .unwrap_or(0)
            .saturating_add(left.tx_bytes_per_sec.unwrap_or(0));
        let right_rate = right
            .rx_bytes_per_sec
            .unwrap_or(0)
            .saturating_add(right.tx_bytes_per_sec.unwrap_or(0));
        let left_total =
            left.total_rx_bytes.saturating_add(left.total_tx_bytes);
        let right_total =
            right.total_rx_bytes.saturating_add(right.total_tx_bytes);

        right_rate
            .cmp(&left_rate)
            .then_with(|| right_total.cmp(&left_total))
            .then_with(|| left.name.cmp(&right.name))
    });
    rows.truncate(MAX_INTERFACE_ROWS);
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        MAX_INTERFACE_ROWS, RawInterfaceCounter, Sampler, build_snapshot,
    };

    #[test]
    fn warmup_snapshot_has_none_rates_and_excludes_loopback_from_totals() {
        let counters = vec![
            RawInterfaceCounter {
                name: "lo0".to_owned(),
                delta_rx_bytes: 5_000,
                delta_tx_bytes: 5_000,
                total_rx_bytes: 100_000,
                total_tx_bytes: 100_000,
                packets_rx: 50,
                packets_tx: 50,
                errors_rx: 0,
                errors_tx: 0,
            },
            RawInterfaceCounter {
                name: "en0".to_owned(),
                delta_rx_bytes: 2_000,
                delta_tx_bytes: 1_000,
                total_rx_bytes: 40_000,
                total_tx_bytes: 20_000,
                packets_rx: 30,
                packets_tx: 15,
                errors_rx: 0,
                errors_tx: 0,
            },
        ];

        let snapshot = build_snapshot(&counters, None);

        assert_eq!(snapshot.rx_bytes_per_sec, None);
        assert_eq!(snapshot.tx_bytes_per_sec, None);
        assert_eq!(snapshot.total_rx_bytes, 40_000);
        assert_eq!(snapshot.total_tx_bytes, 20_000);
        assert_eq!(snapshot.interfaces.len(), 2);
        assert_eq!(snapshot.interfaces[0].rx_bytes_per_sec, None);
    }

    #[test]
    fn elapsed_sample_computes_rates_sorts_by_throughput_and_bounds_rows() {
        let counters: Vec<_> = (0_u32..25)
            .map(|idx| RawInterfaceCounter {
                name: format!("eth{idx:02}"),
                delta_rx_bytes: u64::from(idx) * 1_000,
                delta_tx_bytes: u64::from(idx) * 500,
                total_rx_bytes: u64::from(idx) * 10_000,
                total_tx_bytes: u64::from(idx) * 5_000,
                packets_rx: u64::from(idx),
                packets_tx: u64::from(idx),
                errors_rx: 0,
                errors_tx: 0,
            })
            .collect();

        let snapshot =
            build_snapshot(&counters, Some(Duration::from_millis(500)));

        assert_eq!(snapshot.interfaces.len(), MAX_INTERFACE_ROWS);
        assert_eq!(snapshot.interfaces[0].name, "eth24");
        assert_eq!(snapshot.interfaces[0].rx_bytes_per_sec, Some(48_000));
        assert_eq!(snapshot.interfaces[0].tx_bytes_per_sec, Some(24_000));
    }

    #[test]
    fn live_sampler_warms_up_then_emits_rates() {
        let mut sampler = Sampler::new();
        let first = sampler.sample();
        assert_eq!(first.rx_bytes_per_sec, None);
        assert_eq!(first.tx_bytes_per_sec, None);
    }
}
