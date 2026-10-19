export type SupportedLocale =
  | "en"
  | "zh-CN"
  | "zh-TW"
  | "ja"
  | "ko"
  | "fr"
  | "de"
  | "es"
  | "ru";

export interface LocaleOption {
  code: SupportedLocale;
  label: string;
  nativeName: string;
}

export interface LocaleMessageSchema {
  interfaceText: Record<string, string>;
  nav: {
    runtime: string;
    cpu: string;
    memory: string;
    battery: string;
    network: string;
    space: string;
  };
  masthead: {
    edition: string;
    traySummary: string;
    selectLanguage: string;
    review: string;
    help: string;
    connecting: string;
    streamActive: string;
    streamIdle: string;
  };
  runtime: {
    hostTelemetry: string;
    processes: string;
    hostname: string;
    os: string;
    kernel: string;
    uptime: string;
    loadAverage: string;
    batteryLevel: string;
    netThroughput: string;
    diskAllocated: string;
    cpuLoad: string;
    memoryLoad: string;
    memoryDetail: string;
    batteryDetail: string;
    networkDetail: string;
  };
  cpu: {
    physicalCores: string;
    logicalCores: string;
    frequency: string;
    avgUtilization: string;
    perCoreUtilization: string;
    core: string;
  };
  memory: {
    systemMemory: string;
    physicalRam: string;
    swapSpace: string;
    used: string;
    free: string;
    cached: string;
    total: string;
    utilization: string;
  };
  battery: {
    batteryStatus: string;
    powerMetrics: string;
    level: string;
    state: string;
    health: string;
    cycleCount: string;
    temperature: string;
    powerSource: string;
    charging: string;
    discharging: string;
    full: string;
    empty: string;
    unknown: string;
    acPower: string;
    batteryPower: string;
  };
  network: {
    title: string;
    processLedger: string;
    astrolabeTitle: string;
    astrolabeSubtitle: string;
    activeSockets: string;
    colProcess: string;
    colPid: string;
    colProtocol: string;
    colLocalAddr: string;
    colRemoteAddr: string;
    colState: string;
    noConnections: string;
    searchPlaceholder: string;
    zoomIn: string;
    zoomOut: string;
    resetZoom: string;
    totalRx: string;
    totalTx: string;
    interfaces: string;
  };
  space: {
    scanRoot: string;
    allocated: string;
    apparent: string;
    depthLimit: string;
    rescan: string;
    markForRemoval: string;
    unmark: string;
    revealFinder: string;
    revealFileManager: string;
    insideMarked: string;
    pathActions: string;
    markedItems: string;
    reclaimable: string;
    reviewMarks: string;
    scanning: string;
    emptyScan: string;
    crumbsRoot: string;
    inspector: string;
    name: string;
    path: string;
    size: string;
    items: string;
    files: string;
    directories: string;
    modified: string;
  };
  update: {
    available: string;
    install: string;
    installing: string;
    later: string;
    failed: string;
  };
  review: {
    title: string;
    subtitle: string;
    noMarks: string;
    clearMarks: string;
    exportList: string;
    executeRemoval: string;
    deleteConfirm: string;
    cancel: string;
    confirm: string;
    removing: string;
    reclaimed: string;
  };
  dataTable: {
    search: string;
    noData: string;
    totalRows: string;
  };
  help: {
    title: string;
    navTabs: string;
    navigateTreemap: string;
    zoomTreemap: string;
    markItem: string;
    openDir: string;
    upDir: string;
    review: string;
    close: string;
  };
}
