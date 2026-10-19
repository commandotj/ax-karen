import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { buildManifest, findPlatforms, writeManifest } from "./updater-manifest.mjs";

const repo = "commandotj/ax-karen";
const platforms = {
  "darwin-aarch64": { file: "Karen.app.tar.gz", signature: "SIG-MAC" },
  "linux-x86_64": { file: "Karen_0.13.0_amd64.AppImage", signature: "SIG-LINUX" },
};

function stage(files) {
  const dir = mkdtempSync(join(tmpdir(), "updater-"));
  for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
  return dir;
}

const goodFiles = {
  "Karen.app.tar.gz": "tar",
  "Karen.app.tar.gz.sig": "SIG-MAC\n",
  "Karen_0.13.0_amd64.AppImage": "appimage",
  "Karen_0.13.0_amd64.AppImage.sig": " SIG-LINUX ",
  "Karen_0.13.0_aarch64.dmg": "dmg",
};

test("builds the manifest with release asset URLs", () => {
  const manifest = buildManifest({ tag: "v0.13.0", version: "0.13.0", repo, notes: "Fixes", pubDate: "2026-10-19T00:00:00.000Z", platforms });
  assert.deepEqual(manifest, {
    version: "0.13.0",
    notes: "Fixes",
    pub_date: "2026-10-19T00:00:00.000Z",
    platforms: {
      "darwin-aarch64": { signature: "SIG-MAC", url: "https://github.com/commandotj/ax-karen/releases/download/v0.13.0/Karen.app.tar.gz" },
      "linux-x86_64": { signature: "SIG-LINUX", url: "https://github.com/commandotj/ax-karen/releases/download/v0.13.0/Karen_0.13.0_amd64.AppImage" },
    },
  });
});

test("encodes file names in URLs", () => {
  const manifest = buildManifest({ tag: "v0.13.0", version: "0.13.0", repo, notes: "", pubDate: "d", platforms: { ...platforms, "linux-x86_64": { file: "Karen 0.13.0.AppImage", signature: "S" } } });
  assert.ok(manifest.platforms["linux-x86_64"].url.endsWith("/Karen%200.13.0.AppImage"));
});

test("rejects a tag that does not match the version", () => {
  assert.throws(() => buildManifest({ tag: "v0.13.1", version: "0.13.0", repo, notes: "", pubDate: "d", platforms }), /tag v0\.13\.1 does not match version 0\.13\.0/);
});

test("rejects a missing platform or an empty signature", () => {
  const onlyMac = { "darwin-aarch64": platforms["darwin-aarch64"] };
  assert.throws(() => buildManifest({ tag: "v0.13.0", version: "0.13.0", repo, notes: "", pubDate: "d", platforms: onlyMac }), /missing platform linux-x86_64/);
  assert.throws(() => buildManifest({ tag: "v0.13.0", version: "0.13.0", repo, notes: "", pubDate: "d", platforms: { ...platforms, "darwin-aarch64": { file: "Karen.app.tar.gz", signature: "  " } } }), /empty signature for darwin-aarch64/);
});

test("finds the updater archives and trims their signatures", () => {
  assert.deepEqual(findPlatforms(stage(goodFiles)), {
    "darwin-aarch64": { file: "Karen.app.tar.gz", signature: "SIG-MAC" },
    "linux-x86_64": { file: "Karen_0.13.0_amd64.AppImage", signature: "SIG-LINUX" },
  });
});

test("fails when an archive or its .sig is absent or ambiguous", () => {
  const noMacSig = { ...goodFiles };
  delete noMacSig["Karen.app.tar.gz.sig"];
  assert.throws(() => findPlatforms(stage(noMacSig)), /darwin-aarch64: missing Karen\.app\.tar\.gz\.sig/);

  const noMac = { ...goodFiles };
  delete noMac["Karen.app.tar.gz"];
  delete noMac["Karen.app.tar.gz.sig"];
  assert.throws(() => findPlatforms(stage(noMac)), /darwin-aarch64: expected one \.app\.tar\.gz, found 0/);

  assert.throws(() => findPlatforms(stage({ ...goodFiles, "Other.AppImage": "x" })), /linux-x86_64: expected one \.AppImage, found 2/);
});

test("writes latest.json next to the assets", () => {
  const dir = stage(goodFiles);
  const manifest = writeManifest({ dir, tag: "v0.13.0", version: "0.13.0", repo, notes: "n", now: new Date("2026-10-19T01:02:03.000Z") });
  const onDisk = JSON.parse(readFileSync(join(dir, "latest.json"), "utf8"));
  assert.deepEqual(onDisk, manifest);
  assert.equal(onDisk.pub_date, "2026-10-19T01:02:03.000Z");
});
