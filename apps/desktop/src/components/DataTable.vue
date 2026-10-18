<script setup lang="ts" generic="T extends Record<string, any>">
defineProps<{
  columns: readonly string[];
  items: readonly T[];
  empty?: boolean;
  emptyMessage?: string;
  keyProp?: (item: T, index: number) => string | number;
  headingClass?: string;
  rowClass?: string;
  sectionTitle?: string;
  sectionSubtitle?: string;
}>();
</script>

<template>
  <div class="detail-process-console">
    <div v-if="sectionTitle" class="instrument-section-heading">
      <span>{{ sectionTitle }}</span>
      <span v-if="sectionSubtitle">{{ sectionSubtitle }}</span>
    </div>
    <div class="detail-table-heading" :class="headingClass">
      <span v-for="col in columns" :key="col">{{ col }}</span>
    </div>
    <div class="detail-process-rows">
      <slot v-if="empty || !items.length" name="empty">
        <p class="process-empty">{{ emptyMessage || "NO READINGS AVAILABLE" }}</p>
      </slot>
      <template v-else>
        <div
          v-for="(item, index) in items"
          :key="keyProp ? keyProp(item, index) : index"
          class="detail-process-row"
          :class="rowClass"
        >
          <slot name="row" :item="item" :index="index" />
        </div>
      </template>
    </div>
  </div>
</template>
