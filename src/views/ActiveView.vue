<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useAlertStore } from '#/stores/alerts.ts';
import { useReviewStore } from '#/stores/review.ts';
import type { ActivePatientSummary } from '#/types/patient.ts';

const alertStore = useAlertStore();
const reviewStore = useReviewStore();
const router = useRouter();
const visitPanelOpen = ref(false);
const selectedHn = ref<string>('');
const summaries = ref<ActivePatientSummary[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const searchQuery = ref('');
const searchInputRef = ref<HTMLInputElement | null>(null);

const criticalAlerts = computed(() => alertStore.alerts.filter((a) => a.severity === 'critical'));

const filteredSummaries = computed(() => {
  if (!searchQuery.value.trim()) {
    return summaries.value;
  }
  const query = searchQuery.value.toLowerCase();
  return summaries.value.filter((s) => {
    const hn = s.patient.hn.toLowerCase();
    const fname = s.hosxpInfo?.fname?.toLowerCase() ?? '';
    const lname = s.hosxpInfo?.lname?.toLowerCase() ?? '';
    return hn.includes(query) || fname.includes(query) || lname.includes(query);
  });
});

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault();
    searchInputRef.value?.focus();
  }
}

async function loadRows() {
  loading.value = true;
  error.value = null;
  try {
    summaries.value = await invoke<ActivePatientSummary[]>('get_active_patient_summaries');
    if (!selectedHn.value) {
      selectedHn.value = summaries.value[0]?.patient.hn ?? '';
    }
    void alertStore.fetchAlerts();
  } catch (invokeError) {
    error.value = String(invokeError);
  } finally {
    loading.value = false;
  }
}

function openVisit(hn: string) {
  selectedHn.value = hn;
  visitPanelOpen.value = true;
}

async function handleSaved(visitId: number) {
  visitPanelOpen.value = false;
  void reviewStore.fetchPendingCount();
  await loadRows();
  await router.push(`/slip/${visitId}`);
}

onMounted(() => {
  void loadRows();
  document.addEventListener('keydown', handleKeydown);
});
onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div class="active-view">
    <div v-if="criticalAlerts.length" class="critical-alert-banner" role="alert" aria-live="assertive">
      <div class="critical-alert-inner">
        <AlertTriangle :size="20" class="critical-alert-icon" />
        <div class="critical-alert-content">
          <h2 class="body-sm-medium">แจ้งเตือนวิกฤต {{ criticalAlerts.length }} รายการ</h2>
          <p class="caption" v-for="alert in criticalAlerts.slice(0, 3)" :key="alert.hn">
            HN {{ alert.hn }}: {{ alert.message }}
          </p>
          <p v-if="criticalAlerts.length > 3" class="caption">และอีก {{ criticalAlerts.length - 3 }} รายการ...</p>
        </div>
      </div>
    </div>

    <div class="page-toolbar">
      <div class="stat-row">
        <div class="stat-chip card" aria-label="จำนวนผู้ป่วยทั้งหมด">
          <Users :size="16" class="stat-icon" aria-hidden="true" />
          <span class="body-sm">ผู้ป่วย <strong>{{ filteredSummaries.length }}</strong> ราย</span>
        </div>
      </div>
      <div class="search-box">
        <Search :size="16" class="search-icon" aria-hidden="true" />
        <input
          ref="searchInputRef"
          v-model="searchQuery"
          type="text"
          placeholder="ค้นหา HN, ชื่อ, สกุล (Ctrl+F)"
          class="search-input"
          aria-label="ค้นหาผู้ป่วย"
        />
      </div>
    </div>

    <LoadingState v-if="loading" message="กำลังโหลดรายชื่อผู้ป่วย..." />
    <ErrorState v-else-if="error" :message="error" @retry="loadRows" />
    <div v-else class="table-wrap card">
      <table class="table" aria-label="รายชื่อผู้ป่วยคลินิกวาร์ฟาริน">
        <thead>
          <tr>
            <th scope="col">HN / ชื่อ-นามสกุล</th>
            <th scope="col">INR ล่าสุด</th>
            <th scope="col">ขนาดยา (mg/สัปดาห์)</th>
            <th scope="col">TTR</th>
            <th scope="col">นัดต่อไป</th>
            <th scope="col">การแจ้งเตือน</th>
            <th scope="col"><span class="sr-only">การกระทำ</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="filteredSummaries.length === 0">
            <td colspan="7" class="empty-cell">{{ searchQuery ? 'ไม่พบผู้ป่วยที่ค้นหา' : 'ยังไม่มีผู้ป่วยในคลินิก' }}</td>
          </tr>
          <PatientRow
            v-for="summary in filteredSummaries"
            :key="summary.patient.hn"
            :summary="summary"
            :alerts="alertStore.getAlertsForPatient(summary.patient.hn)"
            @open-visit="openVisit"
          />
        </tbody>
      </table>
    </div>

    <VisitFormPanel v-if="selectedHn" v-model="visitPanelOpen" :hn="selectedHn" @saved="handleSaved" />
  </div>
</template>

<style scoped>
.active-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xl);
}
.critical-alert-banner {
  background: var(--color-inr-high-bg);
  border: 2px solid var(--color-inr-high);
  border-radius: var(--rounded-xl);
  padding: var(--spacing-md) var(--spacing-lg);
  animation: pulse-border 2s ease-in-out infinite;
}
@keyframes pulse-border {
  0%, 100% { border-color: var(--color-inr-high); }
  50% { border-color: var(--color-inr-critical); }
}
.critical-alert-inner {
  display: flex;
  align-items: flex-start;
  gap: var(--spacing-md);
}
.critical-alert-icon {
  color: var(--color-inr-high);
  flex-shrink: 0;
  margin-top: 2px;
}
.critical-alert-content {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xxs);
  color: var(--color-inr-high);
}
.page-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-md);
}
.stat-row { display: flex; gap: var(--spacing-md); }
.stat-chip { display: flex; align-items: center; gap: var(--spacing-xs); }
.stat-icon { color: var(--color-slate); }
.search-box { display: flex; align-items: center; gap: var(--spacing-xs); background: var(--color-canvas); border: 1px solid var(--color-hairline-soft); border-radius: var(--rounded-md); padding: var(--spacing-sm) var(--spacing-md); }
.search-icon { color: var(--color-stone); flex-shrink: 0; }
.search-input { border: none; outline: none; background: transparent; font-size: var(--typography-body-sm-size); color: var(--color-ink); width: 240px; }
.search-input::placeholder { color: var(--color-stone); }
.search-input:focus { outline: 2px solid var(--color-primary); outline-offset: -2px; border-radius: var(--rounded-md); }
.empty-cell {
  padding: var(--spacing-xxl);
  text-align: center;
  color: var(--color-slate);
}
.table-wrap { padding: 0; overflow-x: auto; }
.table-wrap .table { min-width: 980px; }
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border-width: 0;
}
</style>
