use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    Empty,
    Unknown,
}

impl BatteryState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Charging => "charging",
            Self::Discharging => "discharging",
            Self::Full => "full",
            Self::Empty => "empty",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub present: bool,
    pub state: Option<BatteryState>,
    pub percent: Option<u32>,
    pub power_watts: Option<f32>,
    pub energy_wh: Option<f32>,
    pub energy_full_wh: Option<f32>,
    pub energy_design_wh: Option<f32>,
    pub health_percent: Option<u32>,
    pub voltage_volts: Option<f32>,
    pub temperature_celsius: Option<f32>,
    pub cycle_count: Option<u32>,
    pub time_to_empty_secs: Option<u64>,
    pub time_to_full_secs: Option<u64>,
}

impl Snapshot {
    pub const fn state_str(&self) -> Option<&'static str> {
        match self.state {
            Some(state) => Some(state.as_str()),
            None => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct Sampler;

impl Sampler {
    pub const fn new() -> Self {
        Self
    }

    pub fn sample(&mut self) -> Snapshot {
        sample_platform()
    }
}

#[cfg(target_os = "macos")]
fn sample_platform() -> Snapshot {
    let Ok(output) = Command::new("ioreg")
        .args(["-rc", "AppleSmartBattery"])
        .output()
    else {
        return Snapshot::default();
    };
    if !output.status.success() {
        return Snapshot::default();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_macos_ioreg(&text)
}

#[cfg(target_os = "linux")]
fn sample_platform() -> Snapshot {
    read_linux_power_supply(Path::new("/sys/class/power_supply"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn sample_platform() -> Snapshot {
    Snapshot::default()
}

pub fn sanitize_non_negative(value: f32) -> Option<f32> {
    (value.is_finite() && value >= 0.0).then_some(value)
}

pub fn sanitize_positive(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}

pub fn derive_health_percent(
    energy_full_wh: Option<f32>,
    energy_design_wh: Option<f32>,
) -> Option<u32> {
    let full = energy_full_wh.and_then(sanitize_positive)?;
    let design = energy_design_wh.and_then(sanitize_positive)?;
    let ratio = (full / design) * 100.0;
    ratio
        .is_finite()
        .then(|| (ratio.round() as u32).clamp(1, 150))
}

pub fn parse_macos_ioreg(text: &str) -> Snapshot {
    let mut fields: HashMap<&str, &str> = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix('"') else {
            continue;
        };
        let Some((key, rhs)) = rest.split_once('"') else {
            continue;
        };
        let Some((_, value)) = rhs.split_once('=') else {
            continue;
        };
        fields.insert(key.trim(), value.trim());
    }

    let installed = fields
        .get("BatteryInstalled")
        .is_some_and(|value| *value == "Yes");
    if !installed {
        return Snapshot::default();
    }

    let is_charging = fields
        .get("IsCharging")
        .is_some_and(|value| *value == "Yes");
    let fully_charged = fields
        .get("FullyCharged")
        .is_some_and(|value| *value == "Yes");
    let external_connected = fields
        .get("ExternalConnected")
        .is_some_and(|value| *value == "Yes");

    let current_cap = parse_u64_field(&fields, "CurrentCapacity");
    let max_cap = parse_u64_field(&fields, "MaxCapacity");
    let raw_current_mah =
        parse_u64_field(&fields, "AppleRawCurrentCapacity").or(current_cap);
    let raw_max_mah = parse_u64_field(&fields, "AppleRawMaxCapacity")
        .or_else(|| parse_u64_field(&fields, "NominalChargeCapacity"))
        .or(max_cap);
    let design_mah = parse_u64_field(&fields, "DesignCapacity");

    let percent = match (current_cap, max_cap) {
        (Some(cur), Some(100)) => Some((cur as u32).min(100)),
        (Some(cur), Some(max)) if max > 0 => {
            Some((((cur as f64 / max as f64) * 100.0).round() as u32).min(100))
        }
        _ => None,
    };

    let state = if fully_charged
        || (external_connected && percent == Some(100) && !is_charging)
    {
        Some(BatteryState::Full)
    } else if is_charging {
        Some(BatteryState::Charging)
    } else if !external_connected {
        if percent == Some(0) {
            Some(BatteryState::Empty)
        } else {
            Some(BatteryState::Discharging)
        }
    } else {
        Some(BatteryState::Unknown)
    };

    let voltage_mv = parse_u64_field(&fields, "Voltage");
    let voltage_volts = voltage_mv
        .filter(|mv| *mv > 0)
        .and_then(|mv| sanitize_positive(mv as f32 / 1000.0));
    let nominal_volts = voltage_volts.unwrap_or(11.4);

    let amperage_ma = parse_signed_i64_field(&fields, "InstantAmperage")
        .or_else(|| parse_signed_i64_field(&fields, "Amperage"));
    let power_watts = match (voltage_volts, amperage_ma) {
        (Some(volts), Some(ma)) => {
            sanitize_non_negative(volts * (ma.unsigned_abs() as f32 / 1000.0))
        }
        _ => None,
    };

    let energy_wh = raw_current_mah.filter(|mah| *mah > 0).and_then(|mah| {
        sanitize_non_negative((mah as f32 / 1000.0) * nominal_volts)
    });
    let energy_full_wh = raw_max_mah.filter(|mah| *mah > 0).and_then(|mah| {
        sanitize_positive((mah as f32 / 1000.0) * nominal_volts)
    });
    let energy_design_wh = design_mah.filter(|mah| *mah > 0).and_then(|mah| {
        sanitize_positive((mah as f32 / 1000.0) * nominal_volts)
    });
    let health_percent =
        derive_health_percent(energy_full_wh, energy_design_wh);

    let temperature_celsius = parse_u64_field(&fields, "Temperature")
        .filter(|raw| *raw > 0)
        .and_then(|raw| {
            // AppleSmartBattery reports deci-Kelvin (~2900..3500) or centi-Celsius.
            let celsius = if raw > 2000 {
                (raw as f32 / 10.0) - 273.15
            } else {
                raw as f32 / 100.0
            };
            (celsius.is_finite() && (-20.0..=120.0).contains(&celsius))
                .then_some(celsius)
        });

    let cycle_count =
        parse_u64_field(&fields, "CycleCount").map(|cycles| cycles as u32);

    let time_remaining_mins = parse_u64_field(&fields, "TimeRemaining")
        .filter(|mins| *mins > 0 && *mins < 6000);
    let (time_to_empty_secs, time_to_full_secs) =
        match (state, time_remaining_mins) {
            (Some(BatteryState::Discharging), Some(mins)) => {
                (Some(mins * 60), None)
            }
            (Some(BatteryState::Charging), Some(mins)) => {
                (None, Some(mins * 60))
            }
            _ => (None, None),
        };

    Snapshot {
        present: true,
        state,
        percent,
        power_watts,
        energy_wh,
        energy_full_wh,
        energy_design_wh,
        health_percent,
        voltage_volts,
        temperature_celsius,
        cycle_count,
        time_to_empty_secs,
        time_to_full_secs,
    }
}

pub fn read_linux_power_supply(root: &Path) -> Snapshot {
    let Ok(entries) = fs::read_dir(root) else {
        return Snapshot::default();
    };
    let mut battery_dirs: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let is_bat_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("BAT"));
            let is_bat_type = read_trimmed(&path.join("type"))
                .is_some_and(|kind| kind.eq_ignore_ascii_case("Battery"));
            is_bat_name || is_bat_type
        })
        .collect();
    battery_dirs.sort();

    let Some(dir) = battery_dirs.first() else {
        return Snapshot::default();
    };
    if read_trimmed(&dir.join("present")).as_deref() == Some("0") {
        return Snapshot::default();
    }

    let state = match read_trimmed(&dir.join("status")).as_deref() {
        Some("Charging") => Some(BatteryState::Charging),
        Some("Discharging") => Some(BatteryState::Discharging),
        Some("Full" | "Not charging") => Some(BatteryState::Full),
        Some("Empty") => Some(BatteryState::Empty),
        Some(_) => Some(BatteryState::Unknown),
        None => None,
    };

    let voltage_uv = read_u64(&dir.join("voltage_now"));
    let voltage_volts = voltage_uv
        .filter(|uv| *uv > 0)
        .and_then(|uv| sanitize_positive(uv as f32 / 1_000_000.0));
    let nominal_volts = voltage_volts.unwrap_or(11.4);

    let energy_wh = read_u64(&dir.join("energy_now"))
        .map(|uw| uw as f32 / 1_000_000.0)
        .or_else(|| {
            read_u64(&dir.join("charge_now"))
                .map(|uah| (uah as f32 / 1_000_000.0) * nominal_volts)
        })
        .and_then(sanitize_non_negative);

    let energy_full_wh = read_u64(&dir.join("energy_full"))
        .map(|uw| uw as f32 / 1_000_000.0)
        .or_else(|| {
            read_u64(&dir.join("charge_full"))
                .map(|uah| (uah as f32 / 1_000_000.0) * nominal_volts)
        })
        .and_then(sanitize_positive);

    let energy_design_wh = read_u64(&dir.join("energy_full_design"))
        .map(|uw| uw as f32 / 1_000_000.0)
        .or_else(|| {
            read_u64(&dir.join("charge_full_design"))
                .map(|uah| (uah as f32 / 1_000_000.0) * nominal_volts)
        })
        .and_then(sanitize_positive);

    let percent = read_u64(&dir.join("capacity"))
        .map(|cap| (cap as u32).min(100))
        .or_else(|| match (energy_wh, energy_full_wh) {
            (Some(now), Some(full)) if full > 0.0 => {
                Some((((now / full) * 100.0).round() as u32).min(100))
            }
            _ => None,
        });

    let power_watts = read_u64(&dir.join("power_now"))
        .map(|uw| uw as f32 / 1_000_000.0)
        .or_else(|| {
            read_u64(&dir.join("current_now"))
                .map(|ua| (ua as f32 / 1_000_000.0) * nominal_volts)
        })
        .and_then(sanitize_non_negative);

    let health_percent =
        derive_health_percent(energy_full_wh, energy_design_wh);
    let cycle_count = read_u64(&dir.join("cycle_count")).map(|c| c as u32);
    let temperature_celsius = read_u64(&dir.join("temp"))
        .map(|tenths| tenths as f32 / 10.0)
        .filter(|celsius| {
            celsius.is_finite() && (-20.0..=120.0).contains(celsius)
        });

    let (time_to_empty_secs, time_to_full_secs) = match (state, power_watts) {
        (Some(BatteryState::Discharging), Some(watts)) if watts > 0.05 => (
            energy_wh.map(|now| ((now / watts) * 3600.0).round() as u64),
            None,
        ),
        (Some(BatteryState::Charging), Some(watts)) if watts > 0.05 => (
            None,
            match (energy_wh, energy_full_wh) {
                (Some(now), Some(full)) if full > now => {
                    Some((((full - now) / watts) * 3600.0).round() as u64)
                }
                _ => None,
            },
        ),
        _ => (None, None),
    };

    Snapshot {
        present: true,
        state,
        percent,
        power_watts,
        energy_wh,
        energy_full_wh,
        energy_design_wh,
        health_percent,
        voltage_volts,
        temperature_celsius,
        cycle_count,
        time_to_empty_secs,
        time_to_full_secs,
    }
}

fn parse_u64_field(fields: &HashMap<&str, &str>, key: &str) -> Option<u64> {
    fields.get(key)?.parse::<u64>().ok()
}

fn parse_signed_i64_field(
    fields: &HashMap<&str, &str>,
    key: &str,
) -> Option<i64> {
    let raw = fields.get(key)?;
    if let Ok(signed) = raw.parse::<i64>() {
        return Some(signed);
    }
    // macOS `ioreg` prints negative amperage as two's-complement u64.
    let unsigned = raw.parse::<u64>().ok()?;
    Some(unsigned.cast_signed())
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn read_u64(path: &Path) -> Option<u64> {
    read_trimmed(path)?.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        BatteryState, derive_health_percent, parse_macos_ioreg,
        read_linux_power_supply, sanitize_non_negative,
    };

    #[test]
    fn macos_desktop_without_battery_reports_not_present() {
        let sample = parse_macos_ioreg(
            r#"
            +-o AppleSmartBattery
              {
                "BatteryInstalled" = No
                "ExternalConnected" = Yes
                "CurrentCapacity" = 0
                "MaxCapacity" = 0
              }
            "#,
        );

        assert!(!sample.present);
        assert_eq!(sample.state, None);
        assert_eq!(sample.percent, None);
    }

    #[test]
    fn macos_discharging_battery_parses_twos_complement_amperage_and_health() {
        // 18446744073709550366 is -1250 mA in 64-bit two's complement.
        let sample = parse_macos_ioreg(
            r#"
            +-o AppleSmartBattery
              {
                "BatteryInstalled" = Yes
                "ExternalConnected" = No
                "IsCharging" = No
                "FullyCharged" = No
                "CurrentCapacity" = 78
                "MaxCapacity" = 100
                "AppleRawCurrentCapacity" = 3900
                "AppleRawMaxCapacity" = 5000
                "DesignCapacity" = 5400
                "Voltage" = 12000
                "InstantAmperage" = 18446744073709550366
                "Temperature" = 3032
                "CycleCount" = 142
                "TimeRemaining" = 195
              }
            "#,
        );

        assert!(sample.present);
        assert_eq!(sample.state, Some(BatteryState::Discharging));
        assert_eq!(sample.percent, Some(78));
        assert_eq!(sample.power_watts, Some(15.0));
        assert!(sample.energy_wh.is_some_and(|wh| (wh - 46.8).abs() < 0.01));
        assert!(
            sample
                .energy_full_wh
                .is_some_and(|wh| (wh - 60.0).abs() < 0.01)
        );
        assert!(
            sample
                .energy_design_wh
                .is_some_and(|wh| (wh - 64.8).abs() < 0.01)
        );
        assert_eq!(sample.health_percent, Some(93));
        assert_eq!(sample.voltage_volts, Some(12.0));
        assert!(
            sample
                .temperature_celsius
                .is_some_and(|temp| (temp - 30.05).abs() < 0.1)
        );
        assert_eq!(sample.cycle_count, Some(142));
        assert_eq!(sample.time_to_empty_secs, Some(11_700));
        assert_eq!(sample.time_to_full_secs, None);
    }

    #[test]
    fn non_finite_and_negative_values_are_sanitized() {
        assert_eq!(sanitize_non_negative(f32::NAN), None);
        assert_eq!(sanitize_non_negative(f32::INFINITY), None);
        assert_eq!(sanitize_non_negative(-0.1), None);
        assert_eq!(sanitize_non_negative(0.0), Some(0.0));
        assert_eq!(derive_health_percent(Some(0.0), Some(50.0)), None);
        assert_eq!(derive_health_percent(Some(45.0), Some(0.0)), None);
    }

    #[test]
    fn linux_sysfs_power_supply_parses_charging_battery() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("karen-bat-test-{nanos}"));
        let bat0 = root.join("BAT0");
        fs::create_dir_all(&bat0).expect("create BAT0");
        fs::write(bat0.join("type"), "Battery\n").expect("write type");
        fs::write(bat0.join("status"), "Charging\n").expect("write status");
        fs::write(bat0.join("capacity"), "64\n").expect("write capacity");
        fs::write(bat0.join("energy_now"), "32000000\n")
            .expect("write energy_now");
        fs::write(bat0.join("energy_full"), "50000000\n")
            .expect("write energy_full");
        fs::write(bat0.join("energy_full_design"), "55000000\n")
            .expect("write energy_full_design");
        fs::write(bat0.join("power_now"), "18000000\n")
            .expect("write power_now");
        fs::write(bat0.join("voltage_now"), "12400000\n")
            .expect("write voltage_now");
        fs::write(bat0.join("cycle_count"), "88\n").expect("write cycle_count");
        fs::write(bat0.join("temp"), "315\n").expect("write temp");

        let sample = read_linux_power_supply(&root);
        let _ = fs::remove_dir_all(&root);

        assert!(sample.present);
        assert_eq!(sample.state, Some(BatteryState::Charging));
        assert_eq!(sample.percent, Some(64));
        assert_eq!(sample.power_watts, Some(18.0));
        assert_eq!(sample.energy_wh, Some(32.0));
        assert_eq!(sample.energy_full_wh, Some(50.0));
        assert_eq!(sample.energy_design_wh, Some(55.0));
        assert_eq!(sample.health_percent, Some(91));
        assert_eq!(sample.voltage_volts, Some(12.4));
        assert_eq!(sample.temperature_celsius, Some(31.5));
        assert_eq!(sample.cycle_count, Some(88));
        assert_eq!(sample.time_to_full_secs, Some(3600));
    }
}
