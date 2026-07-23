<script setup lang="ts">
import type { RegimenOption } from '#/types/dose.ts';

defineProps<{
  options: RegimenOption[];
  selectedIndex: number | null;
  loading?: boolean;
}>();

const _emit = defineEmits<(e: 'select', index: number) => void>();
</script>

<template>
  <div class="dose-options-panel">
    <div v-if="loading" class="loading-state">
      <span>กำลังคำนวณตัวเลือก...</span>
    </div>

    <div v-else-if="options.length === 0" class="empty-state">
      <span class="body-sm">ไม่พบตัวเลือกที่เหมาะสม</span>
    </div>

    <div v-else class="options-list">
      <div
        v-for="(option, index) in options"
        :key="index"
      >
        <RegimenOptionCard
          :option="option"
          :label="`ตัวเลือก ${index + 1}`"
          :selected="selectedIndex === index"
          interactive
          @select="emit('select', index)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.dose-options-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

.loading-state,
.empty-state {
  padding: var(--spacing-xl);
  text-align: center;
  color: var(--color-stone);
}

.options-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}
</style>
