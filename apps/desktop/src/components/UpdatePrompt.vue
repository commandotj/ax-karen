<script setup lang="ts">
import { onMounted, ref } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useI18n } from "../i18n";

const { t } = useI18n();
const found = ref<Update | null>(null);
const installing = ref(false);
const failed = ref(false);

// A failed or offline check must never bother the user; the next launch tries again.
onMounted(async () => {
  if (!isTauri()) return;
  try {
    found.value = await check();
  } catch (cause) {
    console.warn("Update check failed", cause);
  }
});

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
</template>
