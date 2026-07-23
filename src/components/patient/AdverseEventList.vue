<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { AlertTriangle } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import type { OutcomeInput, OutcomeType, WfOutcome } from '#/types/outcome.ts';
import { dateInputToday, formatThaiDate, sortOutcomes } from '#/utils/clinic.ts';

const props = defineProps<{ hn: string }>();
const outcomes = ref<WfOutcome[]>([]);
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const showForm = ref(false);

const form = ref<OutcomeInput>({
  hn: props.hn,
  eventDate: dateInputToday(),
  eventType: 'minor_bleeding',
  description: '',
  actionTaken: '',
});

const eventLabels: Record<OutcomeType, string> = {
  major_bleeding: 'เลือดออกรุนแรง',
  minor_bleeding: 'เลือดออกเล็กน้อย',
  thromboembolism: 'ลิ่มเลือดอุดตัน',
  hospitalization: 'นอนโรงพยาบาล',
  death: 'เสียชีวิต',
  other: 'อื่นๆ',
};

const orderedOutcomes = computed(() => sortOutcomes(outcomes.value));

async function fetchOutcomes() {
  loading.value = true;
  error.value = null;
  try {
    outcomes.value = await invoke<WfOutcome[]>('get_outcomes', { hn: props.hn });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function submitOutcome() {
  saving.value = true;
  error.value = null;
  try {
    await invoke<number>('record_adverse_event', { event: { ...form.value, hn: props.hn } });
    showForm.value = false;
    form.value = {
      hn: props.hn,
      eventDate: dateInputToday(),
      eventType: 'minor_bleeding',
      description: '',
      actionTaken: '',
    };
    await fetchOutcomes();
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

onMounted(() => {
  void fetchOutcomes();
});
</script>

<template>
  <div class="adverse-section">
    <div class="section-header">
      <h3 class="h5"><AlertTriangle :size="16" /> &#x0E40;&#x0E2B;&#x0E15;&#x0E38;&#x0E01;&#x0E32;&#x0E23;&#x0E13;&#x0E4C;&#x0E44;&#x0E21;&#x0E48;&#x0E1E;&#x0E36;&#x0E07;&#x0E1B;&#x0E23;&#x0E30;&#x0E2A;&#x0E07;&#x0E04;&#x0E4C;</h3>
      <button class="btn btn-secondary" @click="showForm = !showForm">+ &#x0E1A;&#x0E31;&#x0E19;&#x0E17;&#x0E36;&#x0E01;&#x0E40;&#x0E2B;&#x0E15;&#x0E38;&#x0E01;&#x0E32;&#x0E23;&#x0E13;&#x0E4C;</button>
    </div>

    <div v-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">{{ error }}</div>

    <div v-if="showForm" class="card outcome-form">
      <div class="form-grid">
        <label class="form-field">
          <span class="caption" style="color: var(--color-slate)">&#x0E27;&#x0E31;&#x0E19;&#x0E17;&#x0E35;&#x0E48;</span>
          <input class="input" type="date" v-model="form.eventDate" />
        </label>
        <label class="form-field">
          <span class="caption" style="color: var(--color-slate)">&#x0E1B;&#x0E23;&#x0E30;&#x0E40;&#x0E20;&#x0E17;&#x0E40;&#x0E2B;&#x0E15;&#x0E38;&#x0E01;&#x0E32;&#x0E23;&#x0E13;&#x0E4C;</span>
          <select class="input" v-model="form.eventType">
            <option v-for="(label, key) in eventLabels" :key="key" :value="key">{{ label }}</option>
          </select>
        </label>
      </div>
      <label class="form-field">
          <span class="caption" style="color: var(--color-slate)">INR &#x0E02;&#x0E13;&#x0E30;&#x0E40;&#x0E01;&#x0E34;&#x0E14;&#x0E40;&#x0E2B;&#x0E15;&#x0E38;&#x0E01;&#x0E32;&#x0E23;&#x0E13;&#x0E4C;</span>
        <input class="input" type="number" step="0.1" v-model.number="form.inrAtEvent" />
      </label>
      <label class="form-field">
          <span class="caption" style="color: var(--color-slate)">&#x0E23;&#x0E32;&#x0E22;&#x0E25;&#x0E30;&#x0E40;&#x0E2D;&#x0E35;&#x0E22;&#x0E14;</span>
        <textarea class="input" rows="2" v-model="form.description" />
      </label>
      <label class="form-field">
          <span class="caption" style="color: var(--color-slate)">&#x0E01;&#x0E32;&#x0E23;&#x0E14;&#x0E33;&#x0E40;&#x0E19;&#x0E34;&#x0E19;&#x0E01;&#x0E32;&#x0E23;</span>
        <textarea class="input" rows="2" v-model="form.actionTaken" />
      </label>
      <div class="form-actions">
        <button class="btn btn-ghost" @click="showForm = false">&#x0E22;&#x0E01;&#x0E40;&#x0E25;&#x0E34;&#x0E01;</button>
        <button class="btn btn-primary" @click="submitOutcome" :disabled="saving">
          {{ saving ? '&#x0E01;&#x0E33;&#x0E25;&#x0E31;&#x0E07;&#x0E40;&#x0E01;&#x0E47;&#x0E1A;...' : '&#x0E1A;&#x0E31;&#x0E19;&#x0E17;&#x0E36;&#x0E01;' }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="body-sm" style="color: var(--color-stone)">&#x0E01;&#x0E33;&#x0E25;&#x0E31;&#x0E07;&#x0E42;&#x0E2B;&#x0E25;&#x0E14;...</div>
    <div v-else-if="orderedOutcomes.length === 0 && !showForm" class="body-sm" style="color: var(--color-stone)">&#x0E44;&#x0E21;&#x0E48;&#x0E21;&#x0E35;&#x0E1c;&#x0E25;&#x0E25;&#x0E31;&#x0E1E;&#x0E18;&#x0E4C;&#x0E17;&#x0E35;&#x0E48;&#x0E1A;&#x0E31;&#x0E19;&#x0E17;&#x0E36;&#x0E01;</div>
    <div v-else class="outcome-list">
      <div v-for="o in orderedOutcomes" :key="o.id" class="card outcome-row">
        <div class="outcome-meta">
          <span class="badge badge-tag-coral">{{ eventLabels[o.eventType] ?? o.eventType }}</span>
          <span class="caption" style="color: var(--color-slate)">{{ formatThaiDate(o.eventDate) }}</span>
          <span v-if="o.inrAtEvent" class="caption" style="color: var(--color-slate)">INR: {{ o.inrAtEvent.toFixed(1) }}</span>
        </div>
        <p v-if="o.description" class="body-sm">{{ o.description }}</p>
        <p v-if="o.actionTaken" class="caption" style="color: var(--color-slate)">&#x0E01;&#x0E32;&#x0E23;&#x0E14;&#x0E33;&#x0E40;&#x0E19;&#x0E34;&#x0E19;&#x0E01;&#x0E32;&#x0E23;: {{ o.actionTaken }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.adverse-section { display: flex; flex-direction: column; gap: var(--spacing-lg); }
.section-header { display: flex; justify-content: space-between; align-items: center; }
.outcome-form { display: flex; flex-direction: column; gap: var(--spacing-md); padding: var(--spacing-xl); }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-md); }
.form-field { display: flex; flex-direction: column; gap: var(--spacing-xs); }
.form-actions { display: flex; gap: var(--spacing-md); justify-content: flex-end; }
.outcome-list { display: flex; flex-direction: column; gap: var(--spacing-md); }
.outcome-row { display: flex; flex-direction: column; gap: var(--spacing-xs); padding: var(--spacing-md); }
.outcome-meta { display: flex; align-items: center; gap: var(--spacing-sm); flex-wrap: wrap; }
</style>
