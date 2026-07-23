<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed, onMounted, ref } from 'vue';

interface CensusReport {
  active: number;
  inactive: number;
  transferred: number;
  discharged: number;
  deceased: number;
  total: number;
}
interface TtrReport {
  meanTtr: number;
}
interface AdverseReport {
  totalEvents: number;
}
interface InrDistributionReport {
  lt_1_5: number;
  '1_5_to_2_0': number;
  '2_0_to_3_0': number;
  '3_0_to_4_0': number;
  gt_4_0: number;
  total: number;
}
interface MissedAppointmentsReport {
  total: number;
  items: Array<{ hn: string; apptDate: string }>;
}
interface DoseAdjReport {
  totalChanges: number;
  totalVisits: number;
  patientsWithChanges: number;
  changeRatio: number;
}
interface MonthlyCohortReport {
  items: Array<{ month: string; count: number }>;
}

const census = ref<CensusReport | null>(null);
const ttr = ref<TtrReport | null>(null);
const adverse = ref<AdverseReport | null>(null);
const inrDist = ref<InrDistributionReport | null>(null);
const missedAppts = ref<MissedAppointmentsReport | null>(null);
const doseAdj = ref<DoseAdjReport | null>(null);
const monthlyCohort = ref<MonthlyCohortReport | null>(null);
const loading = ref(false);
const dateFrom = ref('');
const dateTo = ref('');

const reportCards = computed(() => [
  {
    key: 'census',
    title: 'สถิติผู้ป่วย',
    value: census.value ? `${census.value.total}` : '-',
    description: census.value
      ? `Active ${census.value.active} · Inactive ${census.value.inactive}`
      : 'กำลังโหลดข้อมูล',
    tone: 'card-feature-yellow',
    rows: census.value
      ? [
          ['กำลังติดตาม', `${census.value.active}`],
          ['หยุดติดตาม', `${census.value.inactive}`],
          ['ส่งต่อ', `${census.value.transferred}`],
          ['จำหน่าย', `${census.value.discharged}`],
          ['เสียชีวิต', `${census.value.deceased}`],
        ]
      : [],
  },
  {
    key: 'ttr',
    title: 'TTR เฉลี่ย',
    value: ttr.value ? `${ttr.value.meanTtr.toFixed(0)}%` : '-',
    description: 'Rosendaal method · 6 เดือนล่าสุด',
    tone: 'card-feature-pink-dark',
    rows: ttr.value ? [['ค่าเฉลี่ยทั้งคลินิก', `${ttr.value.meanTtr.toFixed(2)}%`]] : [],
  },
  {
    key: 'adverse',
    title: 'เหตุการณ์ไม่พึงประสงค์',
    value: adverse.value ? `${adverse.value.totalEvents}` : '-',
    description: 'จำนวนเหตุการณ์ที่บันทึกทั้งหมด',
    tone: 'card-feature-coral',
    rows: adverse.value ? [['รวมเหตุการณ์', `${adverse.value.totalEvents}`]] : [],
  },
  {
    key: 'inr_distribution',
    title: 'การกระจาย INR',
    value: inrDist.value ? `${inrDist.value.total} ค่า` : '-',
    description: 'ฮิสโตแกรมของค่า INR ทั้งหมด',
    tone: 'card-feature-teal',
    rows: inrDist.value
      ? [
          ['< 1.5', `${inrDist.value.lt_1_5}`],
          ['1.5 – 2.0', `${inrDist.value['1_5_to_2_0']}`],
          ['2.0 – 3.0', `${inrDist.value['2_0_to_3_0']}`],
          ['3.0 – 4.0', `${inrDist.value['3_0_to_4_0']}`],
          ['> 4.0', `${inrDist.value.gt_4_0}`],
        ]
      : [],
  },
  {
    key: 'missed_appointments',
    title: 'ขาดนัด',
    value: missedAppts.value ? `${missedAppts.value.total} รายการ` : '-',
    description: 'นัดที่เลยวันแล้วและยังไม่ได้อัพเดตสถานะ',
    tone: 'card-feature-coral',
    rows: missedAppts.value
      ? missedAppts.value.items.slice(0, 5).map((it) => [it.hn, it.apptDate])
      : [],
  },
  {
    key: 'dose_adjustment_frequency',
    title: 'ความถี่ในการปรับยา',
    value: doseAdj.value ? `${(doseAdj.value.changeRatio * 100).toFixed(0)}%` : '-',
    description: 'สัดส่วนการเปลี่ยนขนาดยาต่อการมาคลินิกทั้งหมด',
    tone: 'card-feature-yellow',
    rows: doseAdj.value
      ? [
          ['ครั้งที่ปรับยา', `${doseAdj.value.totalChanges}`],
          ['การมาคลินิกทั้งหมด', `${doseAdj.value.totalVisits}`],
          ['ผู้ป่วยที่มีการปรับยา', `${doseAdj.value.patientsWithChanges}`],
        ]
      : [],
  },
  {
    key: 'monthly_cohort',
    title: 'การลงทะเบียนรายเดือน',
    value: monthlyCohort.value ? `${monthlyCohort.value.items.length} เดือน` : '-',
    description: 'จำนวนผู้ป่วยใหม่ที่ลงทะเบียนในคลินิก (12 เดือนล่าสุด)',
    tone: 'card-feature-pink-dark',
    rows: monthlyCohort.value
      ? monthlyCohort.value.items.map((it) => [it.month, `${it.count}`])
      : [],
  },
]);

async function loadReports() {
  loading.value = true;
  try {
    const [censusData, ttrData, adverseData, inrDistData, missedData, doseAdjData, cohortData] =
      await Promise.all([
        invoke<CensusReport>('get_report_data', { reportType: 'census' }),
        invoke<TtrReport>('get_report_data', { reportType: 'ttr' }),
        invoke<AdverseReport>('get_report_data', { reportType: 'adverse' }),
        invoke<InrDistributionReport>('get_report_data', { reportType: 'inr_distribution' }),
        invoke<MissedAppointmentsReport>('get_report_data', { reportType: 'missed_appointments' }),
        invoke<DoseAdjReport>('get_report_data', { reportType: 'dose_adjustment_frequency' }),
        invoke<MonthlyCohortReport>('get_report_data', { reportType: 'monthly_cohort' }),
      ]);
    census.value = censusData;
    ttr.value = ttrData;
    adverse.value = adverseData;
    inrDist.value = inrDistData;
    missedAppts.value = missedData;
    doseAdj.value = doseAdjData;
    monthlyCohort.value = cohortData;
  } finally {
    loading.value = false;
  }
}

function exportCsv(title: string, rows: string[][]) {
  const csv = [['หัวข้อ', 'ค่า'], ...rows]
    .map((row) => row.map((cell) => `"${cell.replaceAll('"', '""')}"`).join(','))
    .join('\n');
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `${title}.csv`;
  link.click();
  URL.revokeObjectURL(url);
}

onMounted(() => {
  void loadReports();
});
</script>

<template>
  <div class="reports-view">
    <div class="reports-toolbar">
      <div class="date-filter">
        <Calendar :size="16" class="filter-icon" aria-hidden="true" />
        <label class="date-field">
          <span class="caption">จาก</span>
          <input v-model="dateFrom" type="date" class="input input-sm" aria-label="วันที่เริ่มต้น" />
        </label>
        <label class="date-field">
          <span class="caption">ถึง</span>
          <input v-model="dateTo" type="date" class="input input-sm" aria-label="วันที่สิ้นสุด" />
        </label>
        <button type="button" class="btn btn-secondary btn-sm" @click="loadReports" aria-label="รีเฟรชข้อมูลรายงาน">
          โหลดใหม่
        </button>
      </div>
    </div>

    <LoadingState v-if="loading" message="กำลังโหลดรายงาน..." />
    <div v-else class="reports-grid">
      <section v-for="report in reportCards" :key="report.key" :class="['report-card', 'card', report.tone]" :aria-label="report.title">
        <div class="card-head">
          <div>
            <h3 class="h5">{{ report.title }}</h3>
            <p class="body-sm report-description">{{ report.description }}</p>
          </div>
          <button type="button" class="btn btn-secondary" @click="exportCsv(report.key, report.rows)" :aria-label="`ส่งออก ${report.title} เป็น CSV`"><Download :size="16" aria-hidden="true" />CSV</button>
        </div>
        <p class="report-value">{{ report.value }}</p>
        <div class="report-table" role="list">
          <div v-for="row in report.rows" :key="row[0]" class="report-row" role="listitem">
            <span class="body-sm">{{ row[0] }}</span>
            <strong class="body-sm-medium">{{ row[1] }}</strong>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.reports-view { display: flex; flex-direction: column; gap: var(--spacing-xl); }
.reports-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-md);
}
.date-filter {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}
.filter-icon { color: var(--color-stone); }
.date-field {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}
.input-sm {
  min-height: 36px;
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: var(--typography-body-sm-size);
}
.btn-sm {
  padding: var(--spacing-xs) var(--spacing-md);
  font-size: var(--typography-body-sm-size);
}
.reports-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr)); gap: var(--spacing-xl); }
.report-card { display: flex; flex-direction: column; gap: var(--spacing-lg); }
.card-head,.report-row { display: flex; justify-content: space-between; gap: var(--spacing-md); }
.report-description { color: var(--color-slate); }
.report-value { font-size: var(--typography-stat-display-size); font-weight: var(--typography-stat-display-weight); line-height: var(--typography-stat-display-line-height); letter-spacing: var(--typography-stat-display-letter-spacing); }
.report-table { display: flex; flex-direction: column; gap: var(--spacing-sm); }
.report-row { padding-top: var(--spacing-sm); border-top: 1px solid var(--color-hairline-soft); }
</style>
