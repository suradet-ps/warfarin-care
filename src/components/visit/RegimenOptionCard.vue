<script setup lang="ts">
import { Check } from 'lucide-vue-next';
import { computed } from 'vue';
import PillVisual from '#/components/shared/PillVisual.vue';
import type { RegimenOption } from '#/types/dose.ts';

const props = withDefaults(
  defineProps<{
    option: RegimenOption;
    selected?: boolean;
    interactive?: boolean;
    label?: string;
  }>(),
  {
    selected: false,
    interactive: false,
    label: '',
  },
);

const emit = defineEmits<(e: 'select') => void>();

const dayNames = ['จ.', 'อ.', 'พ.', 'พฤ.', 'ศ.', 'ส.', 'อา.'];

const dayHeaderColors = [
  'bg-day-mon',
  'bg-day-tue',
  'bg-day-wed',
  'bg-day-thu',
  'bg-day-fri',
  'bg-day-sat',
  'bg-day-sun',
];

const headerLabel = computed(() => props.label || 'วิธีกินยา');

function expandPills(pills: { mg: number; count: number; is_half: boolean }[]) {
  const expanded: { mg: number; isHalf: boolean; key: string }[] = [];
  for (const pill of pills) {
    for (let i = 0; i < pill.count; i += 1) {
      expanded.push({
        mg: pill.mg,
        isHalf: pill.is_half,
        key: `${pill.mg}-${pill.is_half}-${i}-${expanded.length}`,
      });
    }
  }
  return expanded;
}

function getDayName(dayIndex: number): string {
  return dayNames[dayIndex] ?? '';
}

function getDayHeaderColor(dayIndex: number): string {
  return dayHeaderColors[dayIndex] ?? '';
}
</script>

<template>
  <article
    class="option-card"
    :class="{
      'option-selected': selected,
      'option-interactive': interactive,
    }"
    @click="interactive ? emit('select') : undefined"
  >
    <div class="option-header">
      <div class="option-info">
        <span v-if="selected" class="check-icon">
          <Check :size="16" />
        </span>
        <span class="option-label">{{ headerLabel }}</span>
      </div>
      <div class="option-description">{{ option.description }}</div>
      <div class="option-weekly-dose">
        {{ option.weekly_dose_actual.toFixed(1) }} <span class="text-muted">mg/สัปดาห์</span>
      </div>
    </div>

    <div class="schedule-grid">
      <div
        v-for="day in option.weekly_schedule"
        :key="day.day_index"
        class="schedule-day"
        :class="{
          'day-stop': day.is_stop_day,
          'day-special': day.is_special_day,
        }"
      >
        <div class="day-header" :class="getDayHeaderColor(day.day_index)">
          {{ getDayName(day.day_index) }}
        </div>
        <div class="day-content">
          <template v-if="day.is_stop_day">
            <span class="stop-text">หยุดยา</span>
          </template>
          <template v-else>
            <div class="pills-row">
              <PillVisual
                v-for="pill in expandPills(day.pills)"
                :key="pill.key"
                :mg="pill.mg"
                :is-half="pill.isHalf"
              />
            </div>
            <div class="dose-text">({{ day.total_dose.toFixed(1) }} mg)</div>
          </template>
        </div>
      </div>
    </div>

    <div class="pills-summary">
      <span class="summary-header">{{ option.total_pills_summary.header }}</span>
      <div v-if="option.total_pills_summary.pill_lines.length > 0" class="summary-lines">
        <div v-for="line in option.total_pills_summary.pill_lines" :key="line.mg" class="summary-line">
          <PillVisual :mg="line.mg" />
          <span>{{ line.mg }}mg: {{ line.dispensed_count }} เม็ด</span>
          <span v-if="line.usage_note" class="usage-note">— {{ line.usage_note }}</span>
        </div>
      </div>
      <span v-else class="no-pills">ไม่ต้องจ่ายยา</span>
    </div>
  </article>
</template>

<style scoped>
.option-card {
  border: 1px solid var(--color-hairline-soft);
  border-radius: var(--rounded-xl);
  padding: var(--spacing-lg);
  background: var(--color-canvas);
  box-shadow: var(--elevation-2);
}

.option-interactive {
  cursor: pointer;
  transition: border-color 0.2s ease, box-shadow 0.2s ease, transform 0.2s ease;
}

.option-interactive:hover {
  border-color: var(--color-hairline);
  transform: translateY(-1px);
}

.option-selected {
  border-color: var(--color-pink-600);
  box-shadow: 0 0 0 2px var(--color-pink-600);
}

.option-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
  flex-wrap: wrap;
}

.option-info {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.check-icon {
  color: var(--color-inr-safe);
}

.option-label {
  font-size: var(--typography-caption-bold-size);
  font-weight: var(--typography-caption-bold-weight);
  color: var(--color-pink-600);
}

.option-description {
  flex: 1;
  font-size: var(--typography-body-sm-size);
  color: var(--color-slate);
}

.option-weekly-dose {
  font-size: var(--typography-body-md-size);
  font-weight: var(--typography-body-sm-medium-weight);
  color: var(--color-ink);
}

.schedule-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: var(--spacing-xs);
  margin-bottom: var(--spacing-md);
}

.schedule-day {
  border: 1px solid var(--color-hairline-soft);
  border-radius: var(--rounded-md);
  overflow: hidden;
  background: var(--color-canvas);
}

.day-stop {
  background: var(--color-surface-soft);
}

.day-special {
  border-color: var(--color-coral-500);
  border-width: 2px;
}

.day-header {
  padding: var(--spacing-xs);
  text-align: center;
  font-size: var(--typography-caption-size);
  font-weight: var(--typography-caption-bold-weight);
  color: var(--color-slate);
}

.day-content {
  min-height: 64px;
  padding: var(--spacing-sm);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-xs);
}

.stop-text {
  font-size: var(--typography-micro-size);
  color: var(--color-stone);
}

.pills-row {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 2px;
}

.dose-text {
  font-size: var(--typography-micro-size);
  color: var(--color-slate);
}

.pills-summary {
  padding: var(--spacing-md);
  background: var(--color-surface-soft);
  border-radius: var(--rounded-md);
}

.summary-header {
  display: block;
  margin-bottom: var(--spacing-xs);
  font-size: var(--typography-body-sm-size);
  font-weight: var(--typography-body-sm-medium-weight);
  color: var(--color-ink);
}

.summary-lines {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.summary-line {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  flex-wrap: wrap;
  font-size: var(--typography-body-sm-size);
  color: var(--color-slate);
}

.usage-note {
  color: var(--color-stone);
  font-size: var(--typography-caption-size);
}

.no-pills,
.text-muted {
  color: var(--color-stone);
}

@media (max-width: 900px) {
  .schedule-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
