<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed, onMounted, ref } from 'vue';
import { useSettingsStore } from '#/stores/settings.ts';

interface PatientDrugInteractionRecord {
  date: string;
  drugName: string;
  strength: string;
  icode: string;
  interactionType: string;
}

interface PatientDrugInteractionSummary {
  increaseCount: number;
  decreaseCount: number;
  trend: string;
}

const props = defineProps<{
  hn: string;
  mysqlConfig: {
    host: string;
    port: number;
    database: string;
    username: string;
    password: string;
  };
}>();

const _store = useSettingsStore();
const loading = ref(false);
const error = ref<string | null>(null);
const records = ref<PatientDrugInteractionRecord[]>([]);
const summary = ref<PatientDrugInteractionSummary | null>(null);

async function loadInteractions() {
  loading.value = true;
  error.value = null;
  try {
    const [recs, sum] = await invoke<
      [PatientDrugInteractionRecord[], PatientDrugInteractionSummary]
    >('get_patient_drug_interactions', {
      hn: props.hn,
      mysqlConfig: props.mysqlConfig,
    });
    records.value = recs;
    summary.value = sum;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

const _trendLabel = computed(() => {
  if (!summary.value) {
    return '';
  }
  switch (summary.value.trend) {
    case 'increase':
      return 'มีแนวโน้มเพิ่มฤทธิ์ยา Warfarin';
    case 'decrease':
      return 'มีแนวโน้มลดฤทธิ์ยา Warfarin';
    case 'none':
      return 'ไม่มีปฏิกิริยากับยา Warfarin';
    default:
      return 'ไม่มีผลต่อยา Warfarin ชัดเจน';
  }
});

onMounted(() => {
  void loadInteractions();
});
</script>

<template>
  <div class="drug-interaction-panel">
    <div v-if="loading" class="body-sm" style="color: var(--color-stone); padding: var(--spacing-lg)">
      กำลังโหลด...
    </div>

    <div v-else-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">
      {{ error }}
    </div>

    <template v-else-if="records.length === 0">
      <div class="card" style="padding: var(--spacing-lg); text-align: center;">
        <p class="body-sm" style="color: var(--color-slate)">
          ไม่พบประวัติการใช้ยาที่มีปฏิกิริยากับ Warfarin ในช่วง 1 ปีย้อนหลัง
        </p>
      </div>
    </template>

    <template v-else>
      <div class="table-card">
        <table class="comparison-table">
          <thead>
            <tr class="comparison-row">
              <th>วันที่ล่าสุด</th>
              <th>ชื่อยา</th>
              <th>ผลต่อ Warfarin</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(rec, idx) in records" :key="idx" class="comparison-row">
              <td>{{ formatThaiDate(rec.date) }}</td>
              <td>
                <div class="drug-info">
                  <span class="drug-name">{{ rec.drugName }}</span>
                  <span v-if="rec.strength" class="drug-strength">{{ rec.strength }}</span>
                </div>
              </td>
              <td>
                <span :class="['badge', rec.interactionType === 'increase' ? 'badge-danger' : 'badge-warning']">
                  <component :is="rec.interactionType === 'increase' ? ArrowUp : ArrowDown" :size="12" />
                  {{ rec.interactionType === 'increase' ? 'เพิ่มฤทธิ์' : 'ลดฤทธิ์' }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div :class="['summary-card', summary?.trend === 'increase' ? 'card-feature-coral' : summary?.trend === 'decrease' ? 'card-feature-yellow' : 'card-feature-pink-dark']">
        <div class="summary-header">
          <AlertTriangle :size="20" />
          <span class="h5">สรุปแนวโน้ม Drug interaction</span>
        </div>
        <div class="summary-stats">
          <div class="stat-item">
            <ArrowUp :size="16" class="stat-icon increase" />
            <span class="stat-value">{{ summary?.increaseCount || 0 }}</span>
            <span class="stat-label">ยาที่เพิ่มฤทธิ์</span>
          </div>
          <div class="stat-item">
            <ArrowDown :size="16" class="stat-icon decrease" />
            <span class="stat-value">{{ summary?.decreaseCount || 0 }}</span>
            <span class="stat-label">ยาที่ลดฤทธิ์</span>
          </div>
          <div class="stat-item stat-trend">
            <span class="h4">{{ trendLabel }}</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.drug-interaction-panel { display: flex; flex-direction: column; gap: var(--spacing-lg); }
.table-card { overflow-x: auto; }
.comparison-table { width: 100%; border-collapse: collapse; }
.comparison-row { border-bottom: 1px solid var(--color-hairline-soft); }
.comparison-row th { 
  padding: var(--spacing-sm) var(--spacing-md);
  text-align: left;
  font-weight: 600;
  font-size: var(--typography-micro-uppercase-size);
  color: var(--color-slate);
  background: var(--color-surface);
}
.comparison-row td { padding: var(--spacing-sm) var(--spacing-md); }
.text-right { text-align: right; }
.drug-info { display: flex; flex-direction: column; gap: 2px; }
.drug-name { font-weight: 500; }
.drug-strength { font-size: var(--typography-micro-size); color: var(--color-slate); }
.summary-card { padding: var(--spacing-lg); display: flex; flex-direction: column; gap: var(--spacing-md); }
.summary-header { display: flex; align-items: center; gap: var(--spacing-sm); }
.summary-stats { display: flex; gap: var(--spacing-xl); align-items: center; }
.stat-item { display: flex; align-items: center; gap: var(--spacing-xs); }
.stat-value { font-size: var(--typography-heading-4-size); font-weight: 700; }
.stat-label { font-size: var(--typography-body-sm-size); color: var(--color-slate); }
.stat-icon.increase { color: var(--color-inr-high-bg); }
.stat-icon.decrease { color: var(--color-coral-500); }
.stat-trend { margin-left: auto; }
</style>