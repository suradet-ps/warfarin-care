<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  inrValue?: number | null;
  targetLow: number;
  targetHigh: number;
  lastDate?: string;
  daysSinceLastInr?: number | null;
}>();

type InrStatus = 'in_range' | 'above' | 'below' | 'critical_high' | 'critical_low' | 'no_data';

const status = computed<InrStatus>(() => {
  if (props.inrValue === null || props.inrValue === undefined) {
    return 'no_data';
  }
  if (
    props.daysSinceLastInr !== null &&
    props.daysSinceLastInr !== undefined &&
    props.daysSinceLastInr > 90
  ) {
    return 'no_data';
  }
  if (props.inrValue > 4) {
    return 'critical_high';
  }
  if (props.inrValue < 1.5) {
    return 'critical_low';
  }
  if (props.inrValue > props.targetHigh) {
    return 'above';
  }
  if (props.inrValue < props.targetLow) {
    return 'below';
  }
  if (props.inrValue >= props.targetLow && props.inrValue <= props.targetHigh) {
    return 'in_range';
  }
  return 'in_range';
});

const config = computed(() => {
  const map: Record<InrStatus, { label: string; className: string; dotClass: string }> = {
    in_range: { label: 'อยู่ในเป้าหมาย', className: 'badge-success', dotClass: 'dot-success' },
    above: { label: 'สูงกว่าเป้าหมาย', className: 'badge-warning', dotClass: 'dot-warning' },
    below: { label: 'ต่ำกว่าเป้าหมาย', className: 'badge-warning', dotClass: 'dot-warning' },
    critical_high: { label: 'สูงวิกฤต', className: 'badge-danger', dotClass: 'dot-danger' },
    critical_low: { label: 'ต่ำวิกฤต', className: 'badge-danger', dotClass: 'dot-danger' },
    no_data: { label: 'ไม่มีข้อมูล', className: 'badge-muted', dotClass: 'dot-muted' },
  };
  return map[status.value];
});
</script>

<template>
  <div class="inr-status">
    <span :class="['badge', config.className]">
      {{ inrValue !== null && inrValue !== undefined ? inrValue.toFixed(2) : '-' }}
    </span>
    <span class="caption inr-label">{{ config.label }}</span>
  </div>
</template>

<style scoped>
.inr-status { display: flex; flex-direction: column; gap: var(--spacing-xxs); align-items: center; }
.inr-label { color: var(--color-slate); text-align: center; }
.badge { display: flex; align-items: center; justify-content: center; }
</style>
