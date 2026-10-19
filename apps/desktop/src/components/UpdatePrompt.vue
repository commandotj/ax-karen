<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useI18n } from "../i18n";

const { t } = useI18n();
const found = ref<Update | null>(null);
const installing = ref(false);
const failed = ref(false);
const checking = ref(false);
const notice = ref("");
let unlisten: UnlistenFn | undefined;

// The startup check is silent so a failed or offline check never bothers the user; a manual check
// from the tray menu always answers, and a second request while one is running is dropped.
async function runCheck(manual: boolean): Promise<void> {
  if (checking.value || installing.value) return;
  checking.value = true;
  notice.value = "";
  try {
    found.value = await check();
    if (!found.value && manual) notice.value = t("update.upToDate", { version: await getVersion() });
  } catch (cause) {
    console.warn("Update check failed", cause);
    if (manual) notice.value = t("update.checkFailed");
  } finally {
    checking.value = false;
  }
}

onMounted(async () => {
  if (!isTauri()) return;
  unlisten = await listen("tray-check-update", () => void runCheck(true));
  await runCheck(false);
});

onBeforeUnmount(() => unlisten?.());

async function install(): Promise<void> {
  installing.value = true;
  failed.value = false;
  try {
    await found.value!.downloadAndInstall();
    await relaunch();
  } catch (cause) {
    console.warn("Update install failed", cause);
    installing.value = false;
    failed.value = true;
  }
}
</script>

<template>
  <aside v-if="found" class="update-prompt" role="status">
    <span>{{ t("update.available", { version: found.version }) }}</span>
    <span v-if="failed" class="update-error">{{ t("update.failed") }}</span>
    <div class="update-actions">
      <button class="update-install" :disabled="installing" @click="install">{{ installing ? t("update.installing") : t("update.install") }}</button>
      <button class="update-later" :disabled="installing" @click="found = null">{{ t("update.later") }}</button>
    </div>
  </aside>
  <aside v-else-if="notice" class="update-prompt update-notice" role="status">
    <span>{{ notice }}</span>
    <div class="update-actions">
      <button class="update-dismiss" @click="notice = ''">{{ t("update.dismiss") }}</button>
    </div>
  </aside>
</template>
