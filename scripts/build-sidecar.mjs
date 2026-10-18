import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = fileURLToPath(new URL("../", import.meta.url));
const release = process.argv.includes("--release");
const targetDirectory = JSON.parse(execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
  cwd: rootDir,
  encoding: "utf8",
})).target_directory;
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

const source = resolve(targetDirectory, profile, `karen${extension}`);
const destinationDir = resolve(rootDir, "target/tauri-sidecars");
mkdirSync(destinationDir, { recursive: true });
copyFileSync(source, resolve(destinationDir, `karen-${target}${extension}`));
