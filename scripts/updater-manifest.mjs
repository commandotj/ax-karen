import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const REQUIRED = ["darwin-aarch64", "linux-x86_64"];

export function buildManifest({ tag, version, repo, notes, pubDate, platforms }) {
  if (tag !== `v${version}`) throw new Error(`tag ${tag} does not match version ${version}`);
  const entries = {};
  for (const key of REQUIRED) {
    const platform = platforms[key];
    if (!platform) throw new Error(`missing platform ${key}`);
    if (!platform.signature.trim()) throw new Error(`empty signature for ${key}`);
    entries[key] = {
      signature: platform.signature,
      url: `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(platform.file)}`,
    };
  }
  return { version, notes, pub_date: pubDate, platforms: entries };
}

function signedArchive(dir, files, key, suffix) {
  const matches = files.filter((name) => name.endsWith(suffix));
  if (matches.length !== 1) throw new Error(`${key}: expected one ${suffix}, found ${matches.length}`);
  const file = matches[0];
  if (!files.includes(`${file}.sig`)) throw new Error(`${key}: missing ${file}.sig`);
  return { file, signature: readFileSync(join(dir, `${file}.sig`), "utf8").trim() };
}

export function findPlatforms(dir) {
  const files = readdirSync(dir);
  return {
    "darwin-aarch64": signedArchive(dir, files, "darwin-aarch64", ".app.tar.gz"),
    "linux-x86_64": signedArchive(dir, files, "linux-x86_64", ".AppImage"),
  };
}

export function writeManifest({ dir, tag, version, repo, notes, now = new Date() }) {
  const manifest = buildManifest({ tag, version, repo, notes, pubDate: now.toISOString(), platforms: findPlatforms(dir) });
  writeFileSync(join(dir, "latest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

// Usage: node scripts/updater-manifest.mjs <dir> <tag> <owner/repo> [notes]
if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const [dir, tag, repo, notes = ""] = process.argv.slice(2);
  const conf = JSON.parse(readFileSync(resolve(import.meta.dirname, "../apps/desktop/src-tauri/tauri.conf.json"), "utf8"));
  console.log(JSON.stringify(writeManifest({ dir, tag, version: conf.version, repo, notes }), null, 2));
}
