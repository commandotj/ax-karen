export type Volume = {
  mount_point: string;
  device?: string | null;
  total_bytes?: number | null;
  free_bytes?: number | null;
  available_bytes?: number | null;
};

export type CpuProcess = {
  pid: number;
  name: string;
  cpu_percent: number;
};

export type MemoryProcess = {
  pid: number;
  name: string;
  memory_bytes: number;
};

export type BatteryInfo = {
  present: boolean;
  state?: string | null;
  percent?: number | null;
  power_watts?: number | null;
  energy_wh?: number | null;
  energy_full_wh?: number | null;
  energy_design_wh?: number | null;
  health_percent?: number | null;
  voltage_volts?: number | null;
  temperature_celsius?: number | null;
  cycle_count?: number | null;
  time_to_empty_secs?: number | null;
  time_to_full_secs?: number | null;
};

export type NetworkInterface = {
  name: string;
  rx_bytes_per_sec?: number | null;
  tx_bytes_per_sec?: number | null;
  total_rx_bytes: number;
  total_tx_bytes: number;
  packets_rx: number;
  packets_tx: number;
  errors_rx: number;
  errors_tx: number;
};

export type NetworkProcess = {
  pid: number;
  name: string;
  connection_count: number;
  remote_hosts: string[];
  protocols: string[];
};

export type NetworkConnection = {
  pid?: number | null;
  process_name?: string | null;
  protocol: string;
  local_addr: string;
  local_port: number;
  remote_addr?: string | null;
  remote_port?: number | null;
  state: string;
  service_label?: string | null;
  is_lan: boolean;
};

export type NetworkInfo = {
  rx_bytes_per_sec?: number | null;
  tx_bytes_per_sec?: number | null;
  total_rx_bytes: number;
  total_tx_bytes: number;
  interfaces: NetworkInterface[];
  processes?: NetworkProcess[];
  connections?: NetworkConnection[];
};

export type Sibling = {
  name: string;
  path: string;
  bytes: number;
  is_dir: boolean;
  category?: string;
};

export type Crumb = {
  path: string;
  label: string;
  siblings?: Sibling[];
};

export type DiskMark = {
  path: string;
  bytes: number;
  is_dir: boolean;
  hidden: boolean;
};

export type Finding = {
  path: string;
  bytes: number;
  reason: string;
};

export type BlockedMark = {
  path: string;
  reason: string;
};

export type PowerPreset = "miser" | "balanced" | "aggressive" | "drain";

export type RemovalState = {
  phase: "running" | "done";
  total: number;
  completed: number;
  removed: number;
  bytes: number;
  failed: number;
  errors: string[];
  freeBefore: number | null;
  freeAfter: number | null;
};

export type TreeMode = "size" | "files" | "age";

export const POWER_PRESETS = ["miser", "balanced", "aggressive", "drain"] as const;
