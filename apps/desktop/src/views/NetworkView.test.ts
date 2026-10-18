import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import NetworkView from "./NetworkView.vue";
import type { NetworkInfo } from "../types";

describe("NetworkView component", () => {
  const sampleNetwork: NetworkInfo = {
    rx_bytes_per_sec: 1048576,
    tx_bytes_per_sec: 524288,
    total_rx_bytes: 1073741824,
    total_tx_bytes: 536870912,
    interfaces: [
      {
        name: "en0",
        rx_bytes_per_sec: 1048576,
        tx_bytes_per_sec: 524288,
        total_rx_bytes: 1073741824,
        total_tx_bytes: 536870912,
        packets_rx: 2500,
        packets_tx: 1800,
        errors_rx: 0,
        errors_tx: 1,
      },
    ],
    processes: [
      {
        pid: 1234,
        name: "brave",
        connection_count: 5,
        remote_hosts: ["142.250.190.46", "151.101.1.69"],
        protocols: ["TCP"],
      },
      {
        pid: 5678,
        name: "dns-proxy",
        connection_count: 1,
        remote_hosts: [],
        protocols: ["UDP"],
      },
    ],
    connections: [
      {
        pid: 1234,
        process_name: "brave",
        protocol: "TCP",
        local_addr: "192.168.1.50",
        local_port: 54321,
        remote_addr: "142.250.190.46",
        remote_port: 443,
        state: "ESTABLISHED",
        service_label: "HTTPS",
        is_lan: false,
      },
      {
        pid: 1234,
        process_name: null,
        protocol: "TCP",
        local_addr: "192.168.1.50",
        local_port: 54322,
        remote_addr: "192.168.1.1",
        remote_port: 80,
        state: "ESTABLISHED",
        service_label: "HTTP",
        is_lan: true,
      },
      {
        pid: null,
        process_name: null,
        protocol: "UDP",
        local_addr: "0.0.0.0",
        local_port: 5353,
        remote_addr: null,
        remote_port: null,
        state: "NONE",
        service_label: null,
        is_lan: true,
      },
    ],
  };

  it("renders process ledger by default when processes are provided", async () => {
    const wrapper = mount(NetworkView, {
      props: {
        network: sampleNetwork,
        networkRxSamples: [1024, 2048, 4096],
        networkTxSamples: [512, 1024, 2048],
      },
    });

    expect(wrapper.text()).toContain("Network Telemetry & Topology");
    expect(wrapper.text()).toContain("PROCESS LEDGER (2)");
    expect(wrapper.text()).toContain("brave");
    expect(wrapper.text()).toContain("1234");
    expect(wrapper.text()).toContain("5 sockets");
    expect(wrapper.text()).toContain("2 remotes");
    expect(wrapper.text()).toContain("local only");
    expect(wrapper.text()).toContain("ACTIVE SOCKET CONNECTIONS");
    expect(wrapper.text()).toContain("HTTPS");
    expect(wrapper.text()).toContain("WAN");
    expect(wrapper.text()).toContain("LAN");
  });

  it("navigates to interfaces subtab and displays interface metrics", async () => {
    const wrapper = mount(NetworkView, {
      props: {
        network: sampleNetwork,
        networkRxSamples: [1024],
        networkTxSamples: [512],
      },
    });

    const ifaceBtn = wrapper.find('[data-testid="subtab-interfaces"]');
    await ifaceBtn.trigger("click");

    expect(wrapper.text()).toContain("PHYSICAL & VIRTUAL INTERFACES");
    expect(wrapper.text()).toContain("en0");
    expect(wrapper.text()).toContain("4300 pkts · 1 err");
  });

  it("navigates to astrolabe subtab and displays celestial socket topology", async () => {
    const wrapper = mount(NetworkView, {
      props: {
        network: sampleNetwork,
        networkRxSamples: [],
        networkTxSamples: [],
      },
    });

    const astrolabeBtn = wrapper.find('[data-testid="subtab-astrolabe"]');
    await astrolabeBtn.trigger("click");

    expect(wrapper.text()).toContain("STEAMPUNK CONNECTION ASTROLABE");
    expect(wrapper.text()).toContain("LAN PERIMETER (2)");
    expect(wrapper.text()).toContain("WAN CELESTIAL ORBIT (1)");
    expect(wrapper.text()).toContain("N · 000°");
    expect(wrapper.text()).toContain("LOCAL HOST");

    // Metric cards
    expect(wrapper.find(".astrolabe-metrics").text()).toContain("3"); // active sockets
    expect(wrapper.find(".astrolabe-metrics").text()).toContain("2"); // lan neighbors
    expect(wrapper.find(".astrolabe-metrics").text()).toContain("1"); // wan destinations
    expect(wrapper.find(".astrolabe-metrics").text()).toContain("2"); // established
  });

  it("defaults to interfaces subtab when processes are not present", () => {
    const networkWithoutProcs: NetworkInfo = {
      rx_bytes_per_sec: 1024,
      tx_bytes_per_sec: 512,
      total_rx_bytes: 2048,
      total_tx_bytes: 1024,
      interfaces: [
        {
          name: "lo0",
          rx_bytes_per_sec: null,
          tx_bytes_per_sec: null,
          total_rx_bytes: 0,
          total_tx_bytes: 0,
          packets_rx: 0,
          packets_tx: 0,
          errors_rx: 0,
          errors_tx: 0,
        },
      ],
    };

    const wrapper = mount(NetworkView, {
      props: {
        network: networkWithoutProcs,
        networkRxSamples: [],
        networkTxSamples: [],
      },
    });

    expect(wrapper.find(".network-interface-console").exists()).toBe(true);
    expect(wrapper.text()).toContain("lo0");
  });

  it("handles completely empty network data gracefully", async () => {
    const emptyNetwork: NetworkInfo = {
      rx_bytes_per_sec: null,
      tx_bytes_per_sec: null,
      total_rx_bytes: 0,
      total_tx_bytes: 0,
      interfaces: [],
      processes: [],
      connections: [],
    };

    const wrapper = mount(NetworkView, {
      props: {
        network: emptyNetwork,
        networkRxSamples: [],
        networkTxSamples: [],
      },
    });

    expect(wrapper.text()).toContain("NO NETWORK INTERFACES DETECTED");

    await wrapper.find('[data-testid="subtab-apps"]').trigger("click");
    expect(wrapper.text()).toContain("NO ACTIVE NETWORK PROCESSES DETECTED");

    await wrapper.find('[data-testid="subtab-astrolabe"]').trigger("click");
    expect(wrapper.find(".astrolabe-svg").exists()).toBe(true);
    expect(wrapper.find(".astrolabe-node").exists()).toBe(false);

    await wrapper.find('[data-testid="subtab-interfaces"]').trigger("click");
    expect(wrapper.text()).toContain("NO NETWORK INTERFACES DETECTED");
  });
});
