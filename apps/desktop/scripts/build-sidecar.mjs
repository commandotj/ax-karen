import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktopDir = fileURLToPath(new URL("..", import.meta.url));
const rootDir = resolve(desktopDir, "../..");
const release = process.argv.includes("--release");
const target = process.env.TAURI_ENV_TARGET_TRIPLE ?? execFileSync("rustc", ["--print", "host-tuple"], {
  cwd: rootDir,
  encoding: "utf8",
}).trim();
const extension = target.includes("windows") ? ".exe" : "";
const profile = release ? "release" : "debug";

execFileSync("cargo", ["build", "--locked", "-p", "karen-cli", ...(release ? ["--release"] : [])], {
  cwd: rootDir,
  stdio: "inherit",
});

const source = resolve(rootDir, "target", profile, `karen${extension}`);
const destinationDir = resolve(desktopDir, "src-tauri/binaries");
mkdirSync(destinationDir, { recursive: true });
copyFileSync(source, resolve(destinationDir, `karen-${target}${extension}`));
