<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import {
  Activity,
  ActivitySquare,
  CalendarDays,
  FileClock,
  FilePenLine,
  Pill,
  ShieldAlert,
} from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import TtrBadge from '#/components/active/TtrBadge.vue';
import AdverseEventList from '#/components/patient/AdverseEventList.vue';
import AppointmentTimeline from '#/components/patient/AppointmentTimeline.vue';
import DispensingTable from '#/components/patient/DispensingTable.vue';
import DrugInteractionTable from '#/components/patient/DrugInteractionTable.vue';
import InrTrendChart from '#/components/patient/InrTrendChart.vue';
import StatusChangeModal from '#/components/patient/StatusChangeModal.vue';
import VisitList from '#/components/patient/VisitList.vue';
import ErrorState from '#/components/shared/ErrorState.vue';
import LoadingState from '#/components/shared/LoadingState.vue';
import StatusBadge from '#/components/shared/StatusBadge.vue';
import VisitFormPanel from '#/components/visit/VisitFormPanel.vue';
import { useAlertStore } from '#/stores/alerts.ts';
import { useReviewStore } from '#/stores/review.ts';
import { useSettingsStore } from '#/stores/settings.ts';
import type { PatientDetail } from '#/types/patient.ts';
import type { WfVisit } from '#/types/visit.ts';
import { calculateAge, formatThaiDate, patientFullName, sexLabel } from '#/utils/clinic.ts';

const route = useRoute();
const router = useRouter();
const hn = route.params.hn as string;
const settingsStore = useSettingsStore();
const alertStore = useAlertStore();
const reviewStore = useReviewStore();

type TabKey = 'inr' | 'visits' | 'dispensing' | 'interactions' | 'appointments' | 'adverse';
const activeTab = ref<TabKey>('inr');
const tabs: { key: TabKey; label: string; icon: unknown }[] = [
  { key: 'inr', label: 'INR', icon: Activity },
  { key: 'visits', label: 'ประวัติการทำคลินิก', icon: FileClock },
  { key: 'dispensing', label: 'ประวัติยา', icon: Pill },
  { key: 'interactions', label: 'Drug interaction', icon: ActivitySquare },
  { key: 'appointments', label: 'นัดหมาย', icon: CalendarDays },
  { key: 'adverse', label: 'เหตุการณ์', icon: ShieldAlert },
];

const patientDetail = ref<PatientDetail | null>(null);
const visits = ref<WfVisit[]>([]);
const ttr = ref<number | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const visitPanelOpen = ref(false);
const editingVisit = ref<WfVisit | null>(null);
const statusModalOpen = ref(false);
const appointmentTimelineKey = ref(0);

async function loadPatient() {
  loading.value = true;
  error.value = null;
  try {
    const [detail, visitList, ttrValue] = await Promise.all([
      invoke<PatientDetail>('get_patient_detail', { hn }),
      invoke<WfVisit[]>('get_visit_history', { hn }),
      invoke<number | null>('calculate_ttr', { hn, windowDays: 180 }),
    ]);
    patientDetail.value = detail;
    visits.value = visitList;
    ttr.value = ttrValue;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

const age = computed(() => calculateAge(patientDetail.value?.hosxpInfo?.birthday));
const fullName = computed(() => patientFullName(patientDetail.value?.hosxpInfo));

async function onVisitSaved(visitId: number) {
  visitPanelOpen.value = false;
  editingVisit.value = null;
  void alertStore.fetchAlerts();
  void reviewStore.fetchPendingCount();
  await router.push(`/slip/${visitId}`);
}

function handleEditVisit(visit: WfVisit) {
  editingVisit.value = visit;
  visitPanelOpen.value = true;
}

function handleVisitUpdated() {
  visitPanelOpen.value = false;
  editingVisit.value = null;
  void alertStore.fetchAlerts();
  void reviewStore.fetchPendingCount();
  void refreshVisits();
  void loadPatient();
}

async function refreshVisits() {
  const visitList = await invoke<WfVisit[]>('get_visit_history', { hn });
  visits.value = visitList;
  appointmentTimelineKey.value += 1;
}

function handleVisitDeleted() {
  void alertStore.fetchAlerts();
  void reviewStore.fetchPendingCount();
  void refreshVisits();
}

onMounted(() => {
  void loadPatient();
});
</script>

<template>
  <div class="patient-detail">
    <LoadingState v-if="loading" message="กำลังโหลดข้อมูลผู้ป่วย..." />
    <ErrorState v-else-if="error" :message="error" @retry="loadPatient" />

    <template v-else-if="patientDetail">
      <section class="card header-card">
        <div class="header-main">
          <div>
            <h2 class="h3">{{ fullName }}</h2>
            <p class="body-sm header-meta">
              HN: {{ hn }} &nbsp;|&nbsp;
              อายุ: {{ age ?? '-' }} ปี &nbsp;|&nbsp;
              {{ sexLabel(patientDetail.hosxpInfo?.sex) }} &nbsp;|&nbsp;
              {{ patientDetail.hosxpInfo?.phone || '-' }}
            </p>
          </div>
          <div class="header-badges">
            <StatusBadge :status="patientDetail.patient.status" />
            <TtrBadge :ttr="ttr" />
          </div>
        </div>

        <div class="header-grid">
          <div class="header-item">
            <span class="caption header-label">ข้อบ่งชี้</span>
            <span class="body-sm-medium">{{ patientDetail.patient.indication || '-' }}</span>
          </div>
          <div class="header-item">
            <span class="caption header-label">เป้าหมาย INR</span>
            <span class="body-sm-medium">
              {{ patientDetail.patient.targetInrLow.toFixed(1) }}–{{ patientDetail.patient.targetInrHigh.toFixed(1) }}
            </span>
          </div>
          <div class="header-item">
            <span class="caption header-label">ลงทะเบียน</span>
            <span class="body-sm-medium">{{ formatThaiDate(patientDetail.patient.enrolledAt) }}</span>
          </div>
          <div class="header-item">
            <span class="caption header-label">ที่อยู่</span>
            <span class="body-sm-medium">{{ patientDetail.hosxpInfo?.addrpart || '-' }}</span>
          </div>
        </div>

        <div class="header-actions">
          <button type="button" class="btn btn-secondary" @click="statusModalOpen = true">
            เปลี่ยนสถานะ
          </button>
          <button type="button" class="btn btn-primary" @click="visitPanelOpen = true">
            <FilePenLine :size="16" /> + บันทึกการทำคลินิก
          </button>
        </div>
      </section>

      <div class="tab-bar" role="tablist" aria-label="แท็บข้อมูลผู้ป่วย">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          :class="['tab-pill', { active: activeTab === tab.key }]"
          role="tab"
          :aria-selected="activeTab === tab.key"
          :aria-controls="`tab-panel-${tab.key}`"
          @click="activeTab = tab.key"
        >
          <component :is="tab.icon" :size="14" aria-hidden="true" />
          {{ tab.label }}
        </button>
      </div>

      <div class="tab-content">
        <div v-if="activeTab === 'inr'" id="tab-panel-inr" role="tabpanel" aria-label="กราฟแนวโน้ม INR">
          <InrTrendChart
            :inr-records="patientDetail.inrHistory ?? []"
            :target-low="patientDetail.patient.targetInrLow"
            :target-high="patientDetail.patient.targetInrHigh"
          />
        </div>

        <div v-else-if="activeTab === 'visits'" id="tab-panel-visits" role="tabpanel" aria-label="ประวัติการทำคลินิก">
          <VisitList :visits="visits" :hn="hn" @deleted="handleVisitDeleted" @edit="handleEditVisit" />
        </div>

        <div v-else-if="activeTab === 'dispensing'" id="tab-panel-dispensing" role="tabpanel" aria-label="ประวัติยา">
          <DispensingTable
            :records="patientDetail.dispensingHistory ?? []"
          />
        </div>

        <div v-else-if="activeTab === 'interactions'" id="tab-panel-interactions" role="tabpanel" aria-label="Drug interaction">
          <DrugInteractionTable
            :hn="hn"
            :mysql-config="settingsStore.mysqlConfig"
          />
        </div>

        <div v-else-if="activeTab === 'appointments'" id="tab-panel-appointments" role="tabpanel" aria-label="นัดหมาย">
          <AppointmentTimeline :key="appointmentTimelineKey" :hn="hn" />
        </div>

        <div v-else-if="activeTab === 'adverse'" id="tab-panel-adverse" role="tabpanel" aria-label="เหตุการณ์ไม่พึงประสงค์">
          <AdverseEventList :hn="hn" />
        </div>
      </div>
    </template>

    <div v-else class="card">
      <p class="body-sm">ไม่พบข้อมูลผู้ป่วย HN: {{ hn }}</p>
    </div>

    <VisitFormPanel v-model="visitPanelOpen" :hn="hn" :edit-visit="editingVisit" @saved="onVisitSaved" @updated="handleVisitUpdated" />

    <StatusChangeModal
      v-if="statusModalOpen && patientDetail"
      v-model="statusModalOpen"
      :hn="hn"
      :current-status="patientDetail.patient.status"
      @saved="loadPatient"
    />
  </div>
</template>

<style scoped>
.patient-detail { display: flex; flex-direction: column; gap: var(--spacing-xl); }
.header-card { display: flex; flex-direction: column; gap: var(--spacing-lg); }
.header-main { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--spacing-md); }
.header-meta { color: var(--color-slate); margin-top: var(--spacing-xs); }
.header-badges { display: flex; align-items: center; gap: var(--spacing-sm); flex-shrink: 0; }
.header-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: var(--spacing-md); }
.header-item { display: flex; flex-direction: column; gap: 2px; }
.header-label { color: var(--color-slate); }
.header-actions { display: flex; gap: var(--spacing-md); justify-content: flex-end; }
.tab-bar { display: flex; gap: var(--spacing-xs); flex-wrap: wrap; }
.tab-pill {
  display: flex; align-items: center; gap: var(--spacing-xs);
  padding: var(--spacing-xs) var(--spacing-lg);
  border-radius: var(--rounded-full);
  border: 1px solid var(--color-hairline);
  background: transparent;
  cursor: pointer; font-size: var(--typography-body-sm-size); color: var(--color-slate);
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}
.tab-pill:hover { background: var(--color-surface-raised); }
.tab-pill.active { background: var(--color-primary); color: var(--color-on-primary); border-color: var(--color-primary); }
.tab-content { display: flex; flex-direction: column; gap: var(--spacing-xl); }
</style>