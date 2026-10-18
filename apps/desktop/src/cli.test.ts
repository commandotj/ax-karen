import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CliEvent } from "./cli";

const shell = vi.hoisted(() => ({
  program: "",
  args: [] as string[],
  stdout: undefined as ((chunk: string) => void) | undefined,
  stderr: undefined as ((chunk: string) => void) | undefined,
  written: [] as string[],
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  Command: {
    sidecar(program: string, args: string[]) {
      shell.program = program;
      shell.args = args;
      return {
        stdout: { on: (_: string, handler: (chunk: string) => void) => { shell.stdout = handler; } },
        stderr: { on: (_: string, handler: (chunk: string) => void) => { shell.stderr = handler; } },
        spawn: async () => ({ write: async (line: string) => { shell.written.push(line); } }),
      };
    },
  },
}));

const { KarenCli } = await import("./cli");

describe("KarenCli", () => {
  let events: CliEvent[];
  let cli: InstanceType<typeof KarenCli>;

  beforeEach(() => {
    events = [];
    shell.written.length = 0;
    cli = new KarenCli((event) => events.push(event));
  });

  it("refuses to send before the sidecar starts and stop is a no-op", async () => {
    await expect(cli.send({ command: "volumes" })).rejects.toThrow("Karen CLI is not running");
    await cli.stop();
    expect(shell.written).toEqual([]);
  });

  it("spawns the stream-json sidecar and writes versioned JSON lines", async () => {
    await cli.start();
    expect(shell.program).toBe("../../../target/tauri-sidecars/karen");
    expect(shell.args).toEqual(["--stream-json"]);
    await cli.send({ id: "a", command: "volumes" });
    expect(shell.written).toEqual([`${JSON.stringify({ version: 1, id: "a", command: "volumes" })}\n`]);
  });

  it("frames events across split chunks, skips blank lines and logs invalid JSON", async () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => undefined);
    await cli.start();
    shell.stdout?.('{"event":"rea');
    expect(events).toEqual([]);
    shell.stdout?.('dy"}\n\n  \n{"event":"volumes"}\nnot json\n{"event":"tail"');
    expect(events).toEqual([{ event: "ready" }, { event: "volumes" }]);
    expect(log).toHaveBeenCalledWith("Invalid Karen JSON event:", expect.any(SyntaxError));
    shell.stderr?.("boom");
    expect(log).toHaveBeenCalledWith("karen:", "boom");
    log.mockRestore();
  });

  it("sends shutdown on stop and then refuses further requests", async () => {
    await cli.start();
    await cli.stop();
    expect(JSON.parse(shell.written[0])).toEqual({ version: 1, id: "desktop-shutdown", command: "shutdown" });
    await expect(cli.send({ command: "volumes" })).rejects.toThrow();
  });
});
