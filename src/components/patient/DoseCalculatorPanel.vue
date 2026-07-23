<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { Calculator } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import type { InrRecord } from '#/types/inr.ts';
import type { DoseSuggestion, WfVisit } from '#/types/visit.ts';
import { formatThaiDate } from '#/utils/clinic.ts';

const props = defineProps<{ hn: string; targetLow: number; targetHigh: number }>();

const loading = ref(false);
const error = ref<string | null>(null);
const currentDose = ref<number | null>(null);
const currentInr = ref<number | null>(null);
const latestInrDate = ref<string | null>(null);
const suggestion = ref<DoseSuggestion | null>(null);

const urgencyInfo = computed(() => {
  const map: Record<string, { text: string; className: string }> = {
    normal: { text: 'ปกติ', className: 'badge-success' },
    caution: { text: 'ระวัง', className: 'badge-warning' },
    urgent: { text: 'ด่วน', className: 'badge-danger' },
    hold: { text: 'หยุดยา', className: 'badge-danger' },
  };
  return suggestion.value ? (map[suggestion.value.urgency] ?? map.normal) : null;
});

async function loadDefaults() {
  try {
    const [latestInr, visits] = await Promise.all([
      invoke<InrRecord | null>('get_latest_inr', { hn: props.hn }),
      invoke<WfVisit[]>('get_visit_history', { hn: props.hn }),
    ]);
    currentInr.value = latestInr?.value ?? null;
    latestInrDate.value = latestInr?.date ?? null;
    const lastVisit = visits[0];
    currentDose.value = lastVisit?.newDoseMgday ?? lastVisit?.currentDoseMgday ?? null;
  } catch (e) {
    error.value = String(e);
  }
}

async function calculateSuggestion() {
  if (currentDose.value === null || currentInr.value === null) {
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    suggestion.value = await invoke<DoseSuggestion>('suggest_dose', {
      currentDoseMgday: currentDose.value,
      currentInr: currentInr.value,
      targetLow: props.targetLow,
      targetHigh: props.targetHigh,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void loadDefaults();
});
</script>

<template>
  <div class="calc-card card">
    <div class="calc-header">
      <Calculator :size="18" />
      <h3 class="h5">คำนวณขนาดยา</h3>
    </div>

    <div v-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">{{ error }}</div>

    <div class="calc-inputs">
      <label class="form-field">
        <span class="caption" style="color: var(--color-slate)">ขนาดยาปัจจุบัน (mg/วัน)</span>
        <input class="input" type="number" step="0.5" min="0" v-model.number="currentDose" />
      </label>
      <label class="form-field">
        <span class="caption" style="color: var(--color-slate)">
          ค่า INR ล่าสุด
          <span v-if="latestInrDate" style="color: var(--color-stone)"> ({{ formatThaiDate(latestInrDate) }})</span>
        </span>
        <input class="input" type="number" step="0.1" min="0" v-model.number="currentInr" />
      </label>
    </div>

    <div class="calc-range body-sm">
      เป้าหมาย INR: <strong>{{ targetLow.toFixed(1) }}–{{ targetHigh.toFixed(1) }}</strong>
    </div>

    <button class="btn btn-primary" @click="calculateSuggestion" :disabled="loading || currentDose === null || currentInr === null">
      <Calculator :size="14" />
      {{ loading ? 'กำลังคำนวณ...' : 'คำนวณ' }}
    </button>

    <div v-if="suggestion" class="suggestion-box card">
      <div class="suggestion-header">
        <span class="body-sm-medium">ขนาดยาที่แนะนำ: <strong>{{ suggestion.suggestedDoseMgweek.toFixed(1) }} mg/สัปดาห์</strong></span>
        <span v-if="urgencyInfo" :class="['badge', urgencyInfo.className]">{{ urgencyInfo.text }}</span>
      </div>
      <p class="body-sm" style="color: var(--color-slate)">{{ suggestion.recommendation }}</p>
      <p class="caption" style="color: var(--color-stone)">
        ปรับ {{ suggestion.adjustmentPercent >= 0 ? '+' : '' }}{{ suggestion.adjustmentPercent.toFixed(0) }}% •
        ตรวจ INR ครั้งต่อไปใน {{ suggestion.recheckDays }} วัน
      </p>
    </div>
  </div>
</template>

<style scoped>
.calc-card { display: flex; flex-direction: column; gap: var(--spacing-lg); }
.calc-header { display: flex; align-items: center; gap: var(--spacing-sm); }
.calc-inputs { display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-md); }
.calc-range { color: var(--color-slate); }
.form-field { display: flex; flex-direction: column; gap: var(--spacing-xs); }
.suggestion-box { background: var(--color-surface-raised); display: flex; flex-direction: column; gap: var(--spacing-xs); padding: var(--spacing-md); }
.suggestion-header { display: flex; justify-content: space-between; align-items: center; gap: var(--spacing-md); }
</style>
