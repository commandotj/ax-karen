<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "../i18n";
import { stepTab, wheelDeltaStep, wheelOffset, wheelStyle, type TabId, type TabItem } from "./tabWheel";

const { text } = useI18n();

const props = defineProps<{
  tabs: readonly TabItem[];
  active: TabId;
}>();

const emit = defineEmits<{
  select: [id: TabId];
}>();

const ids = computed(() => props.tabs.map((item) => item.id));
const activeIndex = computed(() => Math.max(0, ids.value.indexOf(props.active)));

function onWheel(event: WheelEvent): void {
  const step = wheelDeltaStep(event.deltaX, event.deltaY);
  if (step !== 0) emit("select", stepTab(ids.value, props.active, step));
}
</script>

<template>
  <nav class="tab-rail tab-wheel" :aria-label="text('Monitor views')" @wheel.prevent="onWheel">
    <button
      type="button"
      class="wheel-step wheel-prev"
      :aria-label="text('Previous view')"
      @click="emit('select', stepTab(ids, active, -1))"
    >
      <svg viewBox="0 0 12 12" aria-hidden="true">
        <path d="M7.5 2.5L4 6l3.5 3.5" />
      </svg>
    </button>
    <div class="wheel-drum">
      <span class="wheel-reticle" aria-hidden="true"></span>
      <button
        v-for="(item, idx) in tabs"
        :key="item.id"
        type="button"
        class="wheel-tab"
        :class="{ active: item.id === active }"
        :style="wheelStyle(wheelOffset(activeIndex, idx, tabs.length))"
        @click="emit('select', item.id)"
      >
        <span class="tab-index">{{ item.index }}</span><span>{{ item.label }}</span><i>{{ item.badge }}</i>
      </button>
    </div>
    <button
      type="button"
      class="wheel-step wheel-next"
      :aria-label="text('Next view')"
      @click="emit('select', stepTab(ids, active, 1))"
    >
      <svg viewBox="0 0 12 12" aria-hidden="true">
        <path d="M4.5 2.5L8 6l-3.5 3.5" />
      </svg>
    </button>
  </nav>
</template>
