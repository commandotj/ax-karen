<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { relaunch } from "@tauri-apps/plugin-process";
import { error as logError, info as logInfo } from "@tauri-apps/plugin-log";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { useI18n } from "../i18n";

type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "restarting" | "restart-required" | "failed";
type FailurePhase = "check" | "download" | "install" | "restart" | null;

const { t } = useI18n();
const found = shallowRef<Update | null>(null);
const phase = ref<UpdatePhase>("idle");
const failurePhase = ref<FailurePhase>(null);
const notice = ref("");
const progress = ref<number | null>(null);
const downloadedBytes = ref(0);
const contentLength = ref<number | null>(null);
const busy = computed(() => ["checking", "downloading", "installing", "restarting"].includes(phase.value));
const phaseMessage = computed(() => {
  if (phase.value === "checking") return t("update.checking");
  if (phase.value === "downloading") return progress.value === null
    ? t("update.downloading")
    : t("update.progress", { progress: progress.value });
  if (phase.value === "installing") return t("update.installingPhase");
  if (phase.value === "restarting") return t("update.restarting");
  if (phase.value === "restart-required") return t("update.restartRequired");
  if (failurePhase.value === "download") return t("update.downloadFailed");
  if (failurePhase.value === "install") return t("update.installFailed");
  if (failurePhase.value === "restart") return t("update.restartRequired");
  return "";
});
let unlisten: UnlistenFn | undefined;
let appVersion: Promise<string> | undefined;

function errorText(cause: unknown): string {
  let message: string;
  if (cause instanceof Error) {
    message = `${cause.name}: ${cause.message}`;
  } else if (typeof cause === "string") {
    message = cause;
  } else {
    try {
      message = JSON.stringify(cause) ?? String(cause);
    } catch {
      message = String(cause);
    }
  }
  return (message || String(cause))
    .replace(/(https?:\/\/[^\s"'<>?]+)\?[^\s"'<>]*/gi, "$1?<redacted>")
    .replace(/((?:authorization|proxy-authorization)\s*[:=]\s*)(?:bearer\s+)?[^\s,;]+/gi, "$1<redacted>")
    .replace(/\bbearer\s+[A-Za-z0-9._~+/-]+=*/gi, "Bearer <redacted>");
}

async function logContext(): Promise<string> {
  appVersion ??= getVersion().catch(() => found.value?.currentVersion ?? "unknown");
  const version = await appVersion;
  return `appVersion=${version} platform=${navigator.platform}`;
}

async function recordInfo(message: string): Promise<void> {
  try {
    await logInfo(`updater ${message} ${await logContext()}`);
  } catch {
    return;
  }
}

async function recordError(message: string, cause: unknown): Promise<void> {
  try {
    await logError(`updater ${message} ${await logContext()} error=${errorText(cause)}`);
  } catch {
    return;
  }
}

function setPhase(next: UpdatePhase): void {
  phase.value = next;
  void recordInfo(`phase=${next}${found.value ? ` updateVersion=${found.value.version}` : ""}`);
}

async function runCheck(manual: boolean): Promise<void> {
  if (busy.value || phase.value === "checking") return;
  failurePhase.value = null;
  notice.value = manual ? t("update.checking") : "";
  setPhase("checking");
  try {
    found.value = await check();
    progress.value = null;
    if (found.value) {
      notice.value = "";
      setPhase("available");
      return;
    }
    setPhase("idle");
    if (manual) notice.value = t("update.upToDate", { version: await getVersion() });
  } catch (cause) {
    failurePhase.value = "check";
    phase.value = "failed";
    if (manual) notice.value = t("update.checkFailed");
    void recordError("phase=check-failed", cause);
  }
}

onMounted(async () => {
  if (!isTauri()) return;
  unlisten = await listen("tray-check-update", () => void runCheck(true));
  await runCheck(false);
});

onBeforeUnmount(() => unlisten?.());

function receiveDownloadEvent(event: DownloadEvent): void {
  if (event.event === "Started") {
    downloadedBytes.value = 0;
    contentLength.value = event.data.contentLength ?? null;
    progress.value = contentLength.value === 0 ? null : 0;
    return;
  }
  if (event.event === "Progress") {
    downloadedBytes.value += event.data.chunkLength;
    progress.value = contentLength.value
      ? Math.min(100, Math.floor((downloadedBytes.value / contentLength.value) * 100))
      : null;
    return;
  }
  setPhase("installing");
}

async function install(): Promise<void> {
  const update = found.value;
  if (!update || busy.value || phase.value === "restart-required") return;
  failurePhase.value = null;
  progress.value = null;
  setPhase("downloading");
  try {
    await update.downloadAndInstall(receiveDownloadEvent);
  } catch (cause) {
    failurePhase.value = phase.value === "installing" ? "install" : "download";
    phase.value = "failed";
    void recordError(`phase=${failurePhase.value}-failed`, cause);
    return;
  }
  failurePhase.value = null;
  setPhase("restarting");
  try {
    await relaunch();
    setPhase("restart-required");
  } catch (cause) {
    failurePhase.value = "restart";
    void recordError("phase=restart-failed installed=true", cause);
    setPhase("restart-required");
  }
}

async function restart(): Promise<void> {
  if (phase.value !== "restart-required") return;
  setPhase("restarting");
  try {
    await relaunch();
    setPhase("restart-required");
  } catch (cause) {
    failurePhase.value = "restart";
    void recordError("phase=restart-failed installed=true", cause);
    setPhase("restart-required");
  }
}

function dismiss(): void {
  found.value = null;
  notice.value = "";
  failurePhase.value = null;
  progress.value = null;
  phase.value = "idle";
}
</script>

<template>
  <aside v-if="found" class="update-prompt" role="status" aria-live="polite">
    <span>{{ t("update.available", { version: found.version }) }}</span>
    <span v-if="phaseMessage" class="update-state" :class="{ 'update-error': phase === 'failed' }">{{ phaseMessage }}</span>
    <div v-if="phase === 'downloading' || phase === 'installing' || phase === 'restarting'" class="update-progress" role="progressbar" :aria-valuenow="phase === 'downloading' ? progress ?? undefined : undefined" aria-valuemin="0" aria-valuemax="100" :aria-label="phaseMessage">
      <span :class="{ 'is-indeterminate': phase !== 'downloading' || progress === null }" :style="progress === null || phase !== 'downloading' ? undefined : { width: `${progress}%` }" />
    </div>
    <div class="update-actions">
      <button class="update-install" :disabled="busy" @click="phase === 'restart-required' ? restart() : install()">{{ phase === 'restart-required' ? t("update.restartNow") : phase === 'failed' ? t("update.retryInstall") : t("update.install") }}</button>
      <button class="update-later" :disabled="busy" @click="dismiss">{{ t("update.later") }}</button>
    </div>
  </aside>
  <aside v-else-if="notice" class="update-prompt update-notice" role="status" aria-live="polite">
    <span :class="{ 'update-error': failurePhase === 'check' && phase === 'failed' }">{{ notice }}</span>
    <div class="update-actions">
      <button v-if="failurePhase === 'check'" class="update-retry" @click="runCheck(true)">{{ t("update.retryCheck") }}</button>
      <button class="update-dismiss" @click="dismiss">{{ t("update.dismiss") }}</button>
    </div>
  </aside>
</template>
