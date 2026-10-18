import { Command, type Child } from "@tauri-apps/plugin-shell";

export type CliEvent = Record<string, unknown> & { event: string };

export class KarenCli {
  private child?: Child;
  private output = "";
  private readonly onEvent: (event: CliEvent) => void;

  constructor(onEvent: (event: CliEvent) => void) {
    this.onEvent = onEvent;
  }

  async start(): Promise<void> {
    const command = Command.sidecar("../../../target/tauri-sidecars/karen", ["--stream-json"]);
    command.stdout.on("data", (chunk) => this.consume(chunk));
    command.stderr.on("data", (chunk) => console.error("karen:", chunk));
    this.child = await command.spawn();
  }

  async send(request: Record<string, unknown>): Promise<void> {
    if (!this.child) throw new Error("Karen CLI is not running");
    await this.child.write(`${JSON.stringify({ version: 1, ...request })}\n`);
  }

  async stop(): Promise<void> {
    if (!this.child) return;
    await this.send({ id: "desktop-shutdown", command: "shutdown" });
    this.child = undefined;
  }

  private consume(chunk: string): void {
    this.output += chunk;
    const lines = this.output.split("\n");
    this.output = lines.pop() ?? "";
    for (const line of lines) {
      if (!line.trim()) continue;
      try {
        this.onEvent(JSON.parse(line) as CliEvent);
      } catch (error) {
        console.error("Invalid Karen JSON event:", error);
      }
    }
  }
}
