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

function extractWorkspacePackageNames(path) {
  const cargoToml = readFileSync(path, "utf8");
  const members = cargoToml.match(/^members\s*=\s*\[([\s\S]*?)^\]/m)?.[1];
  if (!members) {
    throw new Error(`Could not find workspace members in ${path}`);
  }

  return members.match(/"([^"]+)"/g).map((member) => {
    const manifestPath = resolve(rootDir, member.slice(1, -1), "Cargo.toml");
    const packageName = readFileSync(manifestPath, "utf8").match(/^name\s*=\s*"([^"]+)"/m)?.[1];
    if (!packageName) {
      throw new Error(`Could not find package name in ${manifestPath}`);
    }
    return packageName;
  });
}

function extractWorkspaceLockVersions(path, workspacePackages) {
  const expectedPackages = new Set(workspacePackages);

  return readFileSync(path, "utf8")
    .split(/^\[\[package\]\]$/m)
    .slice(1)
    .map((entry) => ({
      name: entry.match(/^name\s*=\s*"([^"]+)"/m)?.[1],
      version: entry.match(/^version\s*=\s*"([^"]+)"/m)?.[1],
    }))
    .filter(({ name }) => expectedPackages.has(name));
}

const expectedTag = process.argv[2];

const packageVersion = extractJsonVersion(resolve(rootDir, "package.json"));
const workspaceCargo = extractCargoVersion(resolve(rootDir, "Cargo.toml"));
const desktopCargo = extractCargoVersion(resolve(rootDir, "apps/desktop/src-tauri/Cargo.toml"));
const desktopPkg = extractJsonVersion(resolve(rootDir, "apps/desktop/package.json"));
const tauriConf = extractJsonVersion(resolve(rootDir, "apps/desktop/src-tauri/tauri.conf.json"));
const workspacePackageNames = extractWorkspacePackageNames(resolve(rootDir, "Cargo.toml"));
const lockVersions = extractWorkspaceLockVersions(resolve(rootDir, "Cargo.lock"), workspacePackageNames);

console.log(`Checking version alignment:`);
console.log(`  package.json (workspace):     ${packageVersion}`);
console.log(`  Cargo.toml (workspace):       ${workspaceCargo}`);
console.log(`  apps/desktop/src-tauri/Cargo: ${desktopCargo}`);
console.log(`  apps/desktop/package.json:    ${desktopPkg}`);
console.log(`  tauri.conf.json:              ${tauriConf}`);
for (const { name, version } of lockVersions) {
  console.log(`  Cargo.lock (${name}): ${version}`);
}

const versions = new Set([
  packageVersion,
  workspaceCargo,
  desktopCargo,
  desktopPkg,
  tauriConf,
  ...lockVersions.map(({ version }) => version),
]);
if (versions.size > 1) {
  console.error(`\nError: Version mismatch detected across manifests: ${Array.from(versions).join(", ")}`);
  process.exit(1);
}

if (lockVersions.length !== workspacePackageNames.length) {
  const lockedNames = new Set(lockVersions.map(({ name }) => name));
  const missing = workspacePackageNames.filter((name) => !lockedNames.has(name));
  console.error(`\nError: Missing local workspace packages in Cargo.lock: ${missing.join(", ") || "unknown"}.`);
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
