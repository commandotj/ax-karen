import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const rootDir = resolve(import.meta.dirname, "..");

function extractCargoVersion(path) {
  const content = readFileSync(path, "utf8");
  const match = content.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`Could not find version in ${path}`);
  }
  return match[1];
}

function extractJsonVersion(path) {
  const content = JSON.parse(readFileSync(path, "utf8"));
  if (!content.version) {
    throw new Error(`Could not find version field in ${path}`);
  }
  return content.version;
}

const expectedTag = process.argv[2];

const releasePleaseVersion = readFileSync(resolve(rootDir, "VERSION"), "utf8").trim();
const workspaceCargo = extractCargoVersion(resolve(rootDir, "Cargo.toml"));
const desktopCargo = extractCargoVersion(resolve(rootDir, "apps/desktop/src-tauri/Cargo.toml"));
const desktopPkg = extractJsonVersion(resolve(rootDir, "apps/desktop/package.json"));
const tauriConf = extractJsonVersion(resolve(rootDir, "apps/desktop/src-tauri/tauri.conf.json"));

console.log(`Checking version alignment:`);
console.log(`  VERSION:                      ${releasePleaseVersion}`);
console.log(`  Cargo.toml (workspace):       ${workspaceCargo}`);
console.log(`  apps/desktop/src-tauri/Cargo: ${desktopCargo}`);
console.log(`  apps/desktop/package.json:    ${desktopPkg}`);
console.log(`  tauri.conf.json:              ${tauriConf}`);

const versions = new Set([releasePleaseVersion, workspaceCargo, desktopCargo, desktopPkg, tauriConf]);
if (versions.size > 1) {
  console.error(`\nError: Version mismatch detected across manifests: ${Array.from(versions).join(", ")}`);
  process.exit(1);
}

if (expectedTag) {
  const cleanTag = expectedTag.startsWith("refs/tags/")
    ? expectedTag.replace("refs/tags/", "")
    : expectedTag;
  if (cleanTag !== `v${workspaceCargo}`) {
    console.error(`\nError: Tag '${cleanTag}' does not match manifests version 'v${workspaceCargo}'`);
    process.exit(1);
  }
  console.log(`  Verified against release tag: ${cleanTag}`);
}

console.log(`\nAll versions aligned at ${workspaceCargo}.`);
