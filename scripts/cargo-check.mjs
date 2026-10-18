import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const rootDir = fileURLToPath(new URL("../", import.meta.url));
const cargo = process.env.CARGO || "cargo";

const USAGE = `usage: node scripts/cargo-check.mjs <task>

tasks:
  lint      fmt --check, then clippy over the workspace with -D warnings
  fmt       rustfmt --check over the workspace
  fmt-fix   rustfmt in place
  clippy    clippy --workspace --all-targets --all-features -- -D warnings
  test      cargo test --workspace
  ci        lint, then test
`;

function runCargo(args) {
  const display = `${cargo} ${args.join(" ")}`;
  const result = spawnSync(cargo, args, {
    cwd: rootDir,
    stdio: ["ignore", "inherit", "inherit"],
  });
  if (result.error) {
    console.error(`cargo-check: could not start \`${display}\`: ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(`cargo-check: \`${display}\` failed`);
    process.exit(result.status ?? 1);
  }
}

function fmt(fix) {
  runCargo(fix ? ["fmt", "--all"] : ["fmt", "--all", "--check"]);
}

function clippy() {
  runCargo([
    "clippy",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--",
    "-D",
    "warnings",
  ]);
}

function test() {
  runCargo(["test", "--workspace"]);
}

const task = process.argv[2] ?? "lint";

switch (task) {
  case "lint":
    fmt(false);
    clippy();
    break;
  case "fmt":
    fmt(false);
    break;
  case "fmt-fix":
    fmt(true);
    break;
  case "clippy":
    clippy();
    break;
  case "test":
    test();
    break;
  case "ci":
    fmt(false);
    clippy();
    test();
    break;
  case "help":
  case "-h":
  case "--help":
    process.stdout.write(USAGE);
    break;
  default:
    console.error(`unknown task: ${task}\n\n${USAGE}`);
    process.exit(1);
}
