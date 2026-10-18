use std::collections::{BTreeSet, HashMap};
use std::net::IpAddr;

use netstat2::{
    AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, SocketInfo,
    TcpState, get_sockets_info,
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub const MAX_PROCESS_NETWORK_ROWS: usize = 20;
pub const MAX_SOCKET_CONNECTIONS: usize = 50;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocketConnection {
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub protocol: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: Option<String>,
    pub remote_port: Option<u16>,
    pub state: String,
    pub service_label: Option<String>,
    pub is_lan: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessNetworkUsage {
    pub pid: u32,
    pub name: String,
    pub connection_count: usize,
    pub remote_hosts: Vec<String>,
    pub protocols: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawSocketEntry {
    pub pid: Option<u32>,
    pub protocol: String,
    pub local_addr: IpAddr,
    pub local_port: u16,
    pub remote_addr: Option<IpAddr>,
    pub remote_port: Option<u16>,
    pub state: String,
}

#[must_use]
pub const fn is_lan_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // Unique Local (fc00::/7)
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // Link-Local (fe80::/10)
        }
    }
}

#[must_use]
pub const fn service_label_for_port(port: u16) -> Option<&'static str> {
    match port {
        20 | 21 => Some("FTP"),
        22 => Some("SSH"),
        23 => Some("TELNET"),
        25 | 465 | 587 => Some("SMTP"),
        53 => Some("DNS"),
        67 | 68 => Some("DHCP"),
        80 | 8000 | 8080 | 8888 => Some("HTTP"),
        110 | 995 => Some("POP3"),
        123 => Some("NTP"),
        143 | 993 => Some("IMAP"),
        443 | 8443 => Some("HTTPS"),
        1433 => Some("MSSQL"),
        1521 => Some("ORACLE"),
        3306 => Some("MYSQL"),
        3389 => Some("RDP"),
        5060 | 5061 => Some("SIP"),
        5432 => Some("POSTGRES"),
        5900 => Some("VNC"),
        6379 => Some("REDIS"),
        8086 => Some("INFLUXDB"),
        9090 => Some("PROMETHEUS"),
        9092 => Some("KAFKA"),
        9200 | 9300 => Some("ELASTICSEARCH"),
        27017 => Some("MONGODB"),
        _ => None,
    }
}

#[must_use]
pub const fn tcp_state_str(state: &TcpState) -> &'static str {
    match state {
        TcpState::Closed => "CLOSED",
        TcpState::Listen => "LISTEN",
        TcpState::SynSent => "SYN_SENT",
        TcpState::SynReceived => "SYN_RECEIVED",
        TcpState::Established => "ESTABLISHED",
        TcpState::FinWait1 => "FIN_WAIT_1",
        TcpState::FinWait2 => "FIN_WAIT_2",
        TcpState::CloseWait => "CLOSE_WAIT",
        TcpState::Closing => "CLOSING",
        TcpState::LastAck => "LAST_ACK",
        TcpState::TimeWait => "TIME_WAIT",
        TcpState::DeleteTcb => "DELETE_TCB",
        TcpState::Unknown => "UNKNOWN",
    }
}

pub fn collect_raw_sockets() -> Vec<RawSocketEntry> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let Ok(sockets_info) = get_sockets_info(af_flags, proto_flags) else {
        return Vec::new();
    };

    let mut entries = Vec::with_capacity(sockets_info.len());
    for item in sockets_info {
        let SocketInfo {
            associated_pids,
            protocol_socket_info,
        } = item;
        let pid = associated_pids.first().copied();

        match protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp) => {
                entries.push(RawSocketEntry {
                    pid,
                    protocol: "TCP".to_owned(),
                    local_addr: tcp.local_addr,
                    local_port: tcp.local_port,
                    remote_addr: Some(tcp.remote_addr),
                    remote_port: Some(tcp.remote_port),
                    state: tcp_state_str(&tcp.state).to_owned(),
                });
            }
            ProtocolSocketInfo::Udp(udp) => {
                entries.push(RawSocketEntry {
                    pid,
                    protocol: "UDP".to_owned(),
                    local_addr: udp.local_addr,
                    local_port: udp.local_port,
                    remote_addr: None,
                    remote_port: None,
                    state: "NONE".to_owned(),
                });
            }
        }
    }
    entries
}

#[derive(Default)]
struct ProcessAccumulator {
    name: String,
    connection_count: usize,
    remote_hosts: BTreeSet<String>,
    protocols: BTreeSet<String>,
}

pub fn correlate_sockets<F>(
    raw_entries: &[RawSocketEntry],
    mut process_name_lookup: F,
) -> (Vec<ProcessNetworkUsage>, Vec<SocketConnection>)
where
    F: FnMut(u32) -> Option<String>,
{
    let mut process_map: HashMap<u32, ProcessAccumulator> = HashMap::new();
    let mut connections: Vec<SocketConnection> =
        Vec::with_capacity(raw_entries.len());

    for raw in raw_entries {
        let proc_name = raw.pid.and_then(&mut process_name_lookup);

        let remote_addr_str = raw.remote_addr.map(|addr| addr.to_string());
        let is_lan = raw
            .remote_addr
            .as_ref()
            .map_or_else(|| is_lan_ip(&raw.local_addr), is_lan_ip);

        let effective_port = raw.remote_port.unwrap_or(raw.local_port);
        let service_label = service_label_for_port(effective_port)
            .map(std::borrow::ToOwned::to_owned);

        if let Some(pid) = raw.pid {
            let entry = process_map.entry(pid).or_insert_with(|| {
                let name = proc_name.clone().unwrap_or_else(|| {
                    if pid == 0 {
                        "kernel".to_owned()
                    } else {
                        format!("pid-{pid}")
                    }
                });
                ProcessAccumulator {
                    name,
                    connection_count: 0,
                    remote_hosts: BTreeSet::new(),
                    protocols: BTreeSet::new(),
                }
            });

            entry.connection_count = entry.connection_count.saturating_add(1);
            entry.protocols.insert(raw.protocol.clone());
            if let Some(rem) = remote_addr_str.as_deref().filter(|s| {
                !s.starts_with("127.") && *s != "::1" && *s != "0.0.0.0"
            }) {
                entry.remote_hosts.insert(rem.to_owned());
            }
        }

        connections.push(SocketConnection {
            pid: raw.pid,
            process_name: proc_name,
            protocol: raw.protocol.clone(),
            local_addr: raw.local_addr.to_string(),
            local_port: raw.local_port,
            remote_addr: remote_addr_str,
            remote_port: raw.remote_port,
            state: raw.state.clone(),
            service_label,
            is_lan,
        });
    }

    let mut processes: Vec<ProcessNetworkUsage> = process_map
        .into_iter()
        .map(|(pid, acc)| ProcessNetworkUsage {
            pid,
            name: acc.name,
            connection_count: acc.connection_count,
            remote_hosts: acc.remote_hosts.into_iter().collect(),
            protocols: acc.protocols.into_iter().collect(),
        })
        .collect();

    // Sort processes: by connection_count desc, remote_hosts len desc, pid asc
    processes.sort_by(|left, right| {
        right
            .connection_count
            .cmp(&left.connection_count)
            .then_with(|| {
                right.remote_hosts.len().cmp(&left.remote_hosts.len())
            })
            .then_with(|| left.pid.cmp(&right.pid))
    });
    processes.truncate(MAX_PROCESS_NETWORK_ROWS);

    // Sort connections: WAN established first, then WAN other, then LAN established, then Listen, then others
    connections.sort_by(|left, right| {
        let left_priority = connection_priority(left);
        let right_priority = connection_priority(right);
        right_priority
            .cmp(&left_priority)
            .then_with(|| left.pid.cmp(&right.pid))
            .then_with(|| left.local_port.cmp(&right.local_port))
    });
    connections.truncate(MAX_SOCKET_CONNECTIONS);

    (processes, connections)
}

fn connection_priority(conn: &SocketConnection) -> u8 {
    let has_remote = conn.remote_addr.is_some()
        && conn.remote_addr.as_deref() != Some("0.0.0.0")
        && conn.remote_addr.as_deref() != Some("::");
    if has_remote {
        if !conn.is_lan {
            if conn.state == "ESTABLISHED" { 5 } else { 4 }
        } else if conn.state == "ESTABLISHED" {
            3
        } else {
            2
        }
    } else {
        u8::from(conn.state == "LISTEN")
    }
}

#[derive(Debug, Default)]
pub struct SocketSampler {
    system: System,
}

impl SocketSampler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            system: System::new(),
        }
    }

    pub fn sample(
        &mut self,
    ) -> (Vec<ProcessNetworkUsage>, Vec<SocketConnection>) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            ProcessRefreshKind::new(),
        );

        let raw = collect_raw_sockets();
        correlate_sockets(&raw, |pid| {
            self.system
                .process(Pid::from_u32(pid))
                .map(|p| p.name().to_string_lossy().into_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn test_is_lan_ip() {
        assert!(is_lan_ip(&IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(is_lan_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))));
        assert!(is_lan_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_lan_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 5))));
        assert!(is_lan_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
        assert!(is_lan_ip(&IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
        assert!(is_lan_ip(&IpAddr::V4(Ipv4Addr::BROADCAST)));
        assert!(!is_lan_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_lan_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));

        assert!(is_lan_ip(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(is_lan_ip(&IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
        assert!(is_lan_ip(&IpAddr::V6("fd00::1".parse().unwrap())));
        assert!(is_lan_ip(&IpAddr::V6("fe80::1".parse().unwrap())));
        assert!(!is_lan_ip(&IpAddr::V6(
            "2606:4700:4700::1111".parse().unwrap()
        )));
    }

    #[test]
    fn test_service_label_for_port() {
        assert_eq!(service_label_for_port(20), Some("FTP"));
        assert_eq!(service_label_for_port(22), Some("SSH"));
        assert_eq!(service_label_for_port(23), Some("TELNET"));
        assert_eq!(service_label_for_port(25), Some("SMTP"));
        assert_eq!(service_label_for_port(53), Some("DNS"));
        assert_eq!(service_label_for_port(67), Some("DHCP"));
        assert_eq!(service_label_for_port(80), Some("HTTP"));
        assert_eq!(service_label_for_port(110), Some("POP3"));
        assert_eq!(service_label_for_port(123), Some("NTP"));
        assert_eq!(service_label_for_port(143), Some("IMAP"));
        assert_eq!(service_label_for_port(443), Some("HTTPS"));
        assert_eq!(service_label_for_port(1433), Some("MSSQL"));
        assert_eq!(service_label_for_port(1521), Some("ORACLE"));
        assert_eq!(service_label_for_port(3306), Some("MYSQL"));
        assert_eq!(service_label_for_port(3389), Some("RDP"));
        assert_eq!(service_label_for_port(5060), Some("SIP"));
        assert_eq!(service_label_for_port(5432), Some("POSTGRES"));
        assert_eq!(service_label_for_port(5900), Some("VNC"));
        assert_eq!(service_label_for_port(6379), Some("REDIS"));
        assert_eq!(service_label_for_port(8086), Some("INFLUXDB"));
        assert_eq!(service_label_for_port(9090), Some("PROMETHEUS"));
        assert_eq!(service_label_for_port(9092), Some("KAFKA"));
        assert_eq!(service_label_for_port(9200), Some("ELASTICSEARCH"));
        assert_eq!(service_label_for_port(27017), Some("MONGODB"));
        assert_eq!(service_label_for_port(9999), None);
    }

    #[test]
    fn test_tcp_state_str() {
        assert_eq!(tcp_state_str(&TcpState::Established), "ESTABLISHED");
        assert_eq!(tcp_state_str(&TcpState::Listen), "LISTEN");
        assert_eq!(tcp_state_str(&TcpState::SynSent), "SYN_SENT");
        assert_eq!(tcp_state_str(&TcpState::SynReceived), "SYN_RECEIVED");
        assert_eq!(tcp_state_str(&TcpState::FinWait1), "FIN_WAIT_1");
        assert_eq!(tcp_state_str(&TcpState::FinWait2), "FIN_WAIT_2");
        assert_eq!(tcp_state_str(&TcpState::CloseWait), "CLOSE_WAIT");
        assert_eq!(tcp_state_str(&TcpState::Closing), "CLOSING");
        assert_eq!(tcp_state_str(&TcpState::LastAck), "LAST_ACK");
        assert_eq!(tcp_state_str(&TcpState::TimeWait), "TIME_WAIT");
        assert_eq!(tcp_state_str(&TcpState::DeleteTcb), "DELETE_TCB");
        assert_eq!(tcp_state_str(&TcpState::Closed), "CLOSED");
        assert_eq!(tcp_state_str(&TcpState::Unknown), "UNKNOWN");
    }

    #[test]
    fn test_correlate_sockets_processes_and_connections() {
        let raw = vec![
            RawSocketEntry {
                pid: Some(101),
                protocol: "TCP".to_owned(),
                local_addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
                local_port: 54321,
                remote_addr: Some(IpAddr::V4(Ipv4Addr::new(140, 82, 121, 3))),
                remote_port: Some(443),
                state: "ESTABLISHED".to_owned(),
            },
            RawSocketEntry {
                pid: Some(101),
                protocol: "TCP".to_owned(),
                local_addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)),
                local_port: 54322,
                remote_addr: Some(IpAddr::V4(Ipv4Addr::new(140, 82, 121, 4))),
                remote_port: Some(443),
                state: "ESTABLISHED".to_owned(),
            },
            RawSocketEntry {
                pid: Some(202),
                protocol: "UDP".to_owned(),
                local_addr: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                local_port: 5353,
                remote_addr: None,
                remote_port: None,
                state: "NONE".to_owned(),
            },
            RawSocketEntry {
                pid: Some(0),
                protocol: "TCP".to_owned(),
                local_addr: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                local_port: 80,
                remote_addr: None,
                remote_port: None,
                state: "LISTEN".to_owned(),
            },
            RawSocketEntry {
                pid: None,
                protocol: "TCP".to_owned(),
                local_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
                local_port: 9000,
                remote_addr: None,
                remote_port: None,
                state: "LISTEN".to_owned(),
            },
        ];

        let (processes, connections) =
            correlate_sockets(&raw, |pid| match pid {
                101 => Some("git".to_owned()),
                202 => Some("mDNSResponder".to_owned()),
                _ => None,
            });

        assert_eq!(processes.len(), 3);
        assert_eq!(processes[0].pid, 101);
        assert_eq!(processes[0].name, "git");
        assert_eq!(processes[0].connection_count, 2);
        assert_eq!(processes[0].remote_hosts.len(), 2);
        assert_eq!(processes[0].protocols, vec!["TCP"]);

        assert_eq!(processes[1].pid, 0);
        assert_eq!(processes[1].name, "kernel");
        assert_eq!(processes[1].connection_count, 1);

        assert_eq!(processes[2].pid, 202);
        assert_eq!(processes[2].name, "mDNSResponder");
        assert_eq!(processes[2].connection_count, 1);
        assert_eq!(processes[2].protocols, vec!["UDP"]);

        assert_eq!(connections.len(), 5);
        assert_eq!(connections[0].state, "ESTABLISHED");
        assert_eq!(connections[0].service_label, Some("HTTPS".to_owned()));
        assert!(!connections[0].is_lan);
    }

    #[test]
    fn test_correlate_sockets_truncation() {
        let mut raw = Vec::new();
        for i in 0..60 {
            raw.push(RawSocketEntry {
                pid: Some(1000 + i),
                protocol: "TCP".to_owned(),
                local_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
                local_port: u16::try_from(10000 + i).unwrap(),
                remote_addr: None,
                remote_port: None,
                state: "LISTEN".to_owned(),
            });
        }

        let (processes, connections) =
            correlate_sockets(&raw, |pid| Some(format!("proc-{pid}")));
        assert_eq!(processes.len(), MAX_PROCESS_NETWORK_ROWS);
        assert_eq!(connections.len(), MAX_SOCKET_CONNECTIONS);
    }

    #[test]
    fn test_live_socket_sampler() {
        let mut sampler = SocketSampler::new();
        let (_procs, _conns) = sampler.sample();
    }
}
