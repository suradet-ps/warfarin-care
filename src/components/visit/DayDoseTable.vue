<script setup lang="ts">
import { computed } from 'vue';
import type { DoseSchedule } from '#/types/visit.ts';
import {
  doseDayKeys,
  doseDayLabels,
  emptyDoseSchedule,
  scheduleAverageDose,
} from '#/utils/clinic.ts';

const model = defineModel<DoseSchedule>({ default: emptyDoseSchedule() });
const props = defineProps<{ readonly?: boolean }>();

const totalMgDay = computed(() => scheduleAverageDose(model.value));
</script>

<template>
  <div class="dose-table-wrap">
    <div class="dose-grid">
      <div v-for="day in doseDayKeys" :key="day" class="dose-cell">
        <span class="caption day-label">{{ doseDayLabels[day] }}</span>
        <span v-if="props.readonly" class="dose-value">{{ Number(model?.[day] ?? 0).toFixed(1) }}</span>
        <input v-else v-model.number="model[day]" class="input dose-input" type="number" min="0" step="0.5" />
      </div>
    </div>
    <div class="dose-total">
      <span class="caption" style="color: var(--color-slate)">&#x0E40;&#x0E09;&#x0E25;&#x0E35;&#x0E48;/&#x0E27;&#x0E31;&#x0E19;</span>
      <span class="body-sm-medium">{{ totalMgDay.toFixed(1) }} mg/&#x0E27;&#x0E31;&#x0E19;</span>
    </div>
  </div>
</template>

<style scoped>
.dose-table-wrap {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.dose-grid {
  display: grid;
  grid-template-columns: repeat(7, minmax(88px, 1fr));
  gap: var(--spacing-xs);
}

.dose-cell {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  padding: var(--spacing-md) var(--spacing-sm);
  border: 1px solid var(--color-hairline-soft);
  border-radius: var(--rounded-md);
  background: var(--color-canvas);
  min-width: 0;
}

.day-label {
  color: var(--color-slate);
  text-align: center;
}

.dose-input,
.dose-value {
  text-align: center;
  color: var(--color-ink);
  font-weight: var(--typography-body-sm-medium-weight);
  font-size: var(--typography-body-md-size);
}

.dose-input {
  padding-inline: var(--spacing-xs);
}

.dose-value {
  padding: var(--spacing-sm) 0;
  min-height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.dose-total {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

@media (max-width: 640px) {
  .dose-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}
</style>
