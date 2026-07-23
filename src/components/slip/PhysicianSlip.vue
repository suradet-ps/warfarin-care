<script setup lang="ts">
import { computed, ref } from 'vue';
import RegimenOptionCard from '#/components/visit/RegimenOptionCard.vue';
import { useSettingsStore } from '#/stores/settings.ts';
import type { PatientDetail } from '#/types/patient.ts';
import type { WfVisit } from '#/types/visit.ts';
import {
  calculateAge,
  doseDayKeys,
  doseDayLabels,
  formatThaiDate,
  getCssVar,
  normalizeDoseSchedule,
  scheduleWeeklyTotal,
} from '#/utils/clinic.ts';
import { createRegimenOptionSnapshot } from '#/utils/regimen.ts';

const props = defineProps<{ visit: WfVisit; patient: PatientDetail; ttr: number | null }>();

const settingsStore = useSettingsStore();
const hospitalName = ref('Warfarin Care');

void settingsStore.loadSettings().then(() => {
  hospitalName.value = settingsStore.hospitalName || hospitalName.value;
});

const CHART_WIDTH = 400;
const CHART_HEIGHT = 160;
const PADDING = { top: 12, right: 32, bottom: 24, left: 12 };

interface ChartPoint {
  date: string;
  value: number;
  x: number;
  y: number;
}

const inrRecords = computed(() =>
  [...(props.patient.inrHistory ?? [])].sort((a, b) => a.date.localeCompare(b.date)).slice(-12),
);

const valueRange = computed(() => {
  const values = inrRecords.value.map((r) => r.value);
  if (values.length === 0) {
    return null;
  }
  const targetLow = props.patient.patient.targetInrLow;
  const targetHigh = props.patient.patient.targetInrHigh;
  const rawMin = Math.min(...values, targetLow, targetHigh);
  const rawMax = Math.max(...values, targetLow, targetHigh);
  const min = Math.max(0, Math.floor((rawMin - 0.4) * 2) / 2);
  const max = Math.ceil((rawMax + 0.4) * 2) / 2;
  return { min, max: max === min ? max + 1 : max };
});

const plotWidth = CHART_WIDTH - PADDING.left - PADDING.right;
const plotHeight = CHART_HEIGHT - PADDING.top - PADDING.bottom;

function xForDate(date: string): number {
  const first = inrRecords.value[0];
  const last = inrRecords.value.at(-1);
  if (!(first && last)) {
    return PADDING.left;
  }
  const firstTime = new Date(first.date).getTime();
  const lastTime = new Date(last.date).getTime();
  const currentTime = new Date(date).getTime();
  const span = Math.max(lastTime - firstTime, 1);
  return PADDING.left + ((currentTime - firstTime) / span) * plotWidth;
}

function yForValue(value: number): number {
  const range = valueRange.value;
  if (!range) {
    return PADDING.top;
  }
  return PADDING.top + ((range.max - value) / (range.max - range.min)) * plotHeight;
}

const points = computed<ChartPoint[]>(() =>
  inrRecords.value.map((record) => ({
    ...record,
    x: xForDate(record.date),
    y: yForValue(record.value),
  })),
);

function buildSmoothPath(series: ChartPoint[]): string {
  if (series.length === 0) {
    return '';
  }
  if (series.length === 1) {
    return `M ${series[0].x} ${series[0].y}`;
  }
  if (series.length === 2) {
    return `M ${series[0].x} ${series[0].y} L ${series[1].x} ${series[1].y}`;
  }
  let path = `M ${series[0].x} ${series[0].y}`;
  for (let index = 0; index < series.length - 1; index += 1) {
    const p0 = series[index - 1] ?? series[index];
    const p1 = series[index];
    const p2 = series[index + 1];
    const p3 = series[index + 2] ?? p2;
    const cp1x = p1.x + (p2.x - p0.x) / 6;
    const cp1y = p1.y + (p2.y - p0.y) / 6;
    const cp2x = p2.x - (p3.x - p1.x) / 6;
    const cp2y = p2.y - (p3.y - p1.y) / 6;
    path += ` C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${p2.x} ${p2.y}`;
  }
  return path;
}

const linePath = computed(() => buildSmoothPath(points.value));

const targetBand = computed(() => {
  const range = valueRange.value;
  if (!range) {
    return null;
  }
  const yTop = yForValue(props.patient.patient.targetInrHigh);
  const yBottom = yForValue(props.patient.patient.targetInrLow);
  return {
    y: Math.min(yTop, yBottom),
    height: Math.abs(yBottom - yTop),
  };
});

const yTicks = computed(() => {
  const range = valueRange.value;
  if (!range) {
    return [];
  }
  const step = Math.max(0.5, Math.ceil(((range.max - range.min) / 3) * 2) / 2);
  const ticks: number[] = [];
  for (let value = range.min; value <= range.max + 0.001; value += step) {
    ticks.push(Number(value.toFixed(1)));
  }
  return ticks;
});

const xTicks = computed(() => {
  const records = inrRecords.value;
  if (records.length === 0) {
    return [];
  }
  return records
    .filter((_, i) => i === 0 || i === records.length - 1 || i === Math.floor(records.length / 2))
    .map((record) => ({
      label: new Date(record.date).toLocaleDateString('th-TH', { month: 'short', day: 'numeric' }),
      x: xForDate(record.date),
    }));
});

const chartPalette = computed(() => ({
  line: getCssVar('--color-primary') || '#111111',
  canvas: getCssVar('--color-canvas') || '#ffffff',
  grid: getCssVar('--color-hairline-soft') || '#e5e5e6',
  target: getCssVar('--color-inr-safe') || '#16a34a',
  text: getCssVar('--color-slate') || '#6b7280',
}));

const info = computed(() => props.patient.hosxpInfo);
const p = computed(() => props.patient.patient);
const age = computed(() => (info.value ? calculateAge(info.value.birthday) : null));
const currentDoseSchedule = computed(() => normalizeDoseSchedule(props.visit.doseDetail));
const newDoseSchedule = computed(() => normalizeDoseSchedule(props.visit.newDoseDetail));
const selectedDoseOption = computed(
  () =>
    props.visit.selectedDoseOption ??
    createRegimenOptionSnapshot({
      schedule: props.visit.newDoseDetail,
      visitDate: props.visit.visitDate,
      nextAppointment: props.visit.nextAppointment,
    }),
);

const adherenceLabel: Record<string, string> = {
  good: 'ดี',
  fair: 'พอใช้',
  poor: 'ไม่ดี',
};

function ttrClass(v: number | null): string {
  if (v === null) {
    return '';
  }
  if (v >= 65) {
    return 'ttr-good';
  }
  if (v >= 50) {
    return 'ttr-warn';
  }
  return 'ttr-bad';
}

const sideEffectOptionsHigh: Record<string, string> = {
  body_bleeding: 'เลือดออกตามร่างกาย',
  blood_urine: 'เลือดปนออกมาในปัสสาวะ',
  bleeding_gums: 'เลือดออกตามไรฟัน',
  hemoptysis: 'ไอเป็นเลือด',
  hematoma: 'ห้อเลือด',
};
const sideEffectOptionsLow: Record<string, string> = {
  headache: 'ปวดหัว',
  dizziness_fatigue: 'เวียนศีรษะหรืออ่อนเพลีย',
  faint_breath: 'รู้สึกหวิวหรือหายใจติดขัด',
  numbness_weakness: 'มีอาการชา หรือกล้ามเนื้ออ่อนแรง',
  other: 'อื่นๆ',
};

const adrHighLabels = computed(() => {
  const selected = props.visit.sideEffects ?? [];
  return selected.map((k: string) => sideEffectOptionsHigh[k]).filter(Boolean);
});

const adrLowLabels = computed(() => {
  const selected = props.visit.sideEffects ?? [];
  return selected.map((k: string) => sideEffectOptionsLow[k]).filter(Boolean);
});

function daysFromNow(dateStr: string | null): string {
  if (!dateStr) {
    return '';
  }
  const targetDate = new Date(dateStr);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  targetDate.setHours(0, 0, 0, 0);
  const diffMs = targetDate.getTime() - today.getTime();
  const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));
  if (diffDays < 0) {
    return `(${Math.abs(diffDays)} วันผ่านไปแล้ว)`;
  }
  return `(+${diffDays} วัน)`;
}
</script>

<template>
  <div class="slip-sheet">
    <div class="slip-header">
      <div class="slip-title">
        <strong>Warfarin Assessment & Recommendation</strong>
        <span class="slip-subtitle">คลินิกวาร์ฟาริน {{ hospitalName }}</span>
      </div>
      <div class="slip-date">{{ formatThaiDate(props.visit.visitDate) }}</div>
    </div>

    <table class="patient-table">
      <tbody>
        <tr>
          <td><span class="label">ชื่อ-สกุล</span>{{ [info?.pname, info?.fname, info?.lname].filter(Boolean).join(' ') || '-' }}</td>
          <td><span class="label">HN</span>{{ patient.patient.hn }}</td>
          <td><span class="label">อายุ</span>{{ age ?? '-' }} ปี</td>
          <td><span class="label">เพศ</span>{{ info?.sex === 'M' ? 'ชาย' : info?.sex === 'F' ? 'หญิง' : '-' }}</td>
        </tr>
        <tr>
          <td><span class="label">ข้อบ่งชี้</span>{{ p.indication || '-' }}</td>
          <td><span class="label">เป้าหมาย INR</span>{{ p.targetInrLow.toFixed(1) }}–{{ p.targetInrHigh.toFixed(1) }}</td>
          <td><span class="label">TTR 6 เดือน</span><span :class="ttrClass(ttr)">{{ ttr != null ? ttr.toFixed(0) + '%' : '-' }}</span></td>
          <td><span class="label">การรับประทานยา</span>{{ adherenceLabel[props.visit.adherence ?? ''] ?? '-' }}</td>
        </tr>
      </tbody>
    </table>

    <div class="inr-section">
      <div class="inr-today">
        <span class="label">ค่า INR วันนี้</span>
        <span class="inr-value">{{ props.visit.inrValue?.toFixed(2) ?? '-' }}</span>
      </div>
      <div class="inr-chart">
        <svg v-if="inrRecords.length" :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`" preserveAspectRatio="xMidYMid meet">
          <rect
            v-if="targetBand"
            :x="PADDING.left"
            :y="targetBand.y"
            :width="CHART_WIDTH - PADDING.left - PADDING.right"
            :height="targetBand.height"
            :fill="chartPalette.target"
            fill-opacity="0.15"
          />
          <g>
            <line
              v-for="tick in yTicks"
              :key="`y-${tick}`"
              :x1="PADDING.left"
              :y1="yForValue(tick)"
              :x2="CHART_WIDTH - PADDING.right"
              :y2="yForValue(tick)"
              :stroke="chartPalette.grid"
              stroke-width="1"
            />
          </g>
          <g>
            <line
              :x1="PADDING.left"
              :y1="yForValue(patient.patient.targetInrHigh)"
              :x2="CHART_WIDTH - PADDING.right"
              :y2="yForValue(patient.patient.targetInrHigh)"
              :stroke="chartPalette.target"
              stroke-width="1"
              stroke-dasharray="4 4"
            />
            <line
              :x1="PADDING.left"
              :y1="yForValue(patient.patient.targetInrLow)"
              :x2="CHART_WIDTH - PADDING.right"
              :y2="yForValue(patient.patient.targetInrLow)"
              :stroke="chartPalette.target"
              stroke-width="1"
              stroke-dasharray="4 4"
            />
          </g>
          <path
            :d="linePath"
            fill="none"
            :stroke="chartPalette.line"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <g>
            <circle
              v-for="point in points"
              :key="`${point.date}-${point.value}`"
              :cx="point.x"
              :cy="point.y"
              r="3"
              :fill="chartPalette.line"
              :stroke="chartPalette.canvas"
              stroke-width="1.5"
            />
          </g>
          <g>
            <template v-for="tick in xTicks" :key="`x-${tick.x}`">
              <line
                :x1="tick.x"
                :y1="PADDING.top"
                :x2="tick.x"
                :y2="CHART_HEIGHT - PADDING.bottom"
                :stroke="chartPalette.grid"
                stroke-width="1"
              />
              <text :x="tick.x" :y="CHART_HEIGHT - 4" text-anchor="middle" :fill="chartPalette.text" font-size="10">{{ tick.label }}</text>
            </template>
          </g>
          <g>
            <text
              v-for="tick in yTicks"
              :key="`label-${tick}`"
              :x="CHART_WIDTH - 4"
              :y="yForValue(tick) + 3"
              text-anchor="end"
              :fill="chartPalette.text"
              font-size="10"
            >
              {{ tick.toFixed(1) }}
            </text>
          </g>
        </svg>
        <span v-else class="no-data">ไม่มีข้อมูล</span>
      </div>
    </div>

    <div class="dose-section">
      <table class="dose-table">
        <thead>
          <tr>
            <th></th>
            <th v-for="k in doseDayKeys" :key="k">{{ doseDayLabels[k] }}</th>
            <th>รวม/สัปดาห์</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td class="dose-label">ยาเดิม</td>
            <td v-for="k in doseDayKeys" :key="k">{{ currentDoseSchedule[k] || '-' }}</td>
            <td>{{ scheduleWeeklyTotal(currentDoseSchedule).toFixed(1) }} mg</td>
          </tr>
          <tr class="new-dose-row">
            <td class="dose-label">ยาใหม่</td>
            <td v-for="k in doseDayKeys" :key="k">{{ newDoseSchedule[k] || '-' }}</td>
            <td>{{ scheduleWeeklyTotal(newDoseSchedule).toFixed(1) }} mg</td>
          </tr>
        </tbody>
      </table>

      <div class="dose-instructions">
        <span class="label">วิธีกินยาที่เลือกจากหน้าบันทึกการทำคลินิก</span>
        <RegimenOptionCard :option="selectedDoseOption" label="การ์ดวิธีกินยา" selected />
      </div>
    </div>

    <div class="footer-section">
      <div class="appointment-info">
        <span class="label">นัดครั้งต่อไป:</span>
        <strong>{{ formatThaiDate(props.visit.nextAppointment) }}</strong>
        <span class="days-from-now">{{ daysFromNow(props.visit.nextAppointment ?? null) }}</span>
      </div>
    </div>

    <div class="adr-section">
      <span class="label">อาการไม่พึงประสงค์:</span>
      <div v-if="adrHighLabels.length || adrLowLabels.length">
        <div v-if="adrHighLabels.length" class="adr-group">
          <span class="adr-category">ระดับยาสูง:</span>
          <span class="adr-items">{{ adrHighLabels.join(', ') }}</span>
        </div>
        <div v-if="adrLowLabels.length" class="adr-group">
          <span class="adr-category">ระดับยาต่ำ:</span>
          <span class="adr-items">{{ adrLowLabels.join(', ') }}</span>
        </div>
      </div>
      <span v-else class="adr-none">-</span>
    </div>

    <div class="footer-section">
      <div class="signature-area">
        <div class="signature-box pharmacist-box">
          <div class="box-header">
            <span class="label">Pharmacist Notes</span>
          </div>
          <div class="box-body">
            <div class="notes-space"></div>
            <div class="signature-line">
              <span class="line-label">ลงชื่อ</span>
              <span class="line-space"></span>
            </div>
          </div>
        </div>
        <div class="signature-box doctor-box">
          <div class="box-header">
            <span class="label">Doctor Notes</span>
          </div>
          <div class="box-body">
            <div class="notes-space"></div>
            <div class="signature-line">
              <span class="line-label">ลงชื่อ</span>
              <span class="line-space"></span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.slip-sheet {
  width: 210mm;
  min-height: 297mm;
  max-width: 210mm;
  margin: 0 auto;
  padding: 8mm;
  background: var(--color-canvas);
  font-family: var(--font-family-primary);
  color: var(--color-ink);
  box-sizing: border-box;
  overflow: hidden;
  border: 1px solid var(--color-hairline);
  border-radius: var(--rounded-xl);
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}

.slip-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-md);
  padding-bottom: var(--spacing-sm);
  border-bottom: 2px solid var(--color-primary);
}

.slip-title {
  display: flex;
  flex-direction: column;
}

.slip-title strong {
  font-size: var(--typography-subtitle-size);
  color: var(--color-primary);
}

.slip-subtitle {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
}

.slip-date {
  font-size: var(--typography-body-md-size);
  color: var(--color-ink);
}

.patient-table {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: var(--spacing-md);
}

.patient-table td {
  padding: var(--spacing-xs) var(--spacing-sm);
  border: 1px solid var(--color-hairline);
  font-size: var(--typography-body-md-size);
}

.patient-table .label {
  display: block;
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
  font-weight: 600;
  margin-bottom: 2px;
}

.inr-section {
  display: flex;
  gap: var(--spacing-md);
  align-items: center;
  margin-bottom: var(--spacing-md);
  padding: var(--spacing-sm);
  background: var(--color-surface);
  border-radius: var(--rounded-lg);
}

.inr-today {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 110px;
}

.inr-today .label {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
  font-weight: 600;
}

.inr-value {
  font-size: 2.2rem;
  font-weight: 700;
  color: var(--color-primary);
  line-height: 1;
}

.inr-chart {
  flex: 1;
  height: 148px;
}

.inr-chart svg {
  width: 100%;
  height: 100%;
}

.inr-chart .no-data {
  display: grid;
  place-items: center;
  height: 100%;
  color: var(--color-stone);
  font-size: var(--typography-body-sm-size);
}

.dose-section {
  margin-bottom: var(--spacing-md);
}

.dose-table {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: var(--spacing-sm);
}

.dose-table th,
.dose-table td {
  padding: 6px;
  text-align: center;
  border: 1px solid var(--color-hairline);
  font-size: var(--typography-body-md-size);
}

.dose-table th {
  background: var(--color-surface);
  font-weight: 600;
}

.dose-label {
  text-align: left !important;
  font-weight: 500;
  background: var(--color-surface);
}

.new-dose-row td {
  background: var(--color-pink-100);
  font-weight: 600;
}

.dose-instructions {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.dose-instructions .label {
  display: block;
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
  font-weight: 600;
}

.dose-instructions :deep(.option-card) {
  padding: var(--spacing-md);
  box-shadow: none;
}

.dose-instructions :deep(.option-header) {
  margin-bottom: var(--spacing-sm);
  gap: var(--spacing-sm);
}

.dose-instructions :deep(.schedule-grid) {
  gap: 4px;
  margin-bottom: var(--spacing-sm);
}

.dose-instructions :deep(.day-content) {
  min-height: 56px;
  padding: 6px;
}

.dose-instructions :deep(.pills-summary) {
  padding: var(--spacing-sm);
}

.dose-instructions :deep(.summary-line) {
  font-size: var(--typography-caption-size);
}

.adr-section {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm) var(--spacing-md);
  align-items: baseline;
  padding: var(--spacing-sm);
  background: var(--color-surface);
  border-radius: var(--rounded-md);
  margin-bottom: var(--spacing-md);
}

.adr-section .label {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
  font-weight: 600;
}

.adr-group {
  display: flex;
  gap: var(--spacing-xs);
}

.adr-category {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
  font-weight: 600;
}

.adr-items {
  font-size: var(--typography-body-md-size);
color: var(--color-coral-500);
  font-weight: 600;
}

.adr-none {
  font-size: var(--typography-body-md-size);
  color: var(--color-stone);
}

.footer-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  padding-top: var(--spacing-sm);
  border-top: 1px solid var(--color-hairline);
}

.appointment-info {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm);
  background: var(--color-surface);
  border-radius: var(--rounded-md);
}

.appointment-info .label {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
  font-weight: 600;
}

.appointment-info strong {
  font-size: var(--typography-body-md-size);
}

.days-from-now {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
}

.signature-area {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--spacing-md);
}

.signature-box {
  border: 2px solid var(--color-hairline);
  border-radius: var(--rounded-lg);
  overflow: hidden;
}

.box-header {
  padding: var(--spacing-sm) var(--spacing-md);
  border-bottom: 1px solid var(--color-hairline);
}

.box-header .label {
  font-size: var(--typography-body-md-size);
  font-weight: 700;
}

.pharmacist-box .box-header {
  background: var(--color-pink-100);
  color: var(--color-pink-600);
}

.doctor-box .box-header {
  background: var(--color-pink-100);
  color: var(--color-pink-600);
}

.box-body {
  padding: var(--spacing-md);
}

.notes-space {
  height: 40px;
}

.signature-line {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.line-label {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
}

.line-space {
  flex: 1;
  border-bottom: 1px solid var(--color-ink);
}

.doctor-name {
  font-size: var(--typography-body-md-size);
  color: var(--color-ink);
}

.ttr-good { color: var(--color-ttr-good); }
.ttr-warn { color: var(--color-ttr-warn); }
.ttr-bad { color: var(--color-ttr-bad); }

@media print {
  .slip-sheet {
    width: auto;
    max-width: none;
    min-height: auto;
    padding: 5mm;
    background: white;
    box-shadow: none;
    border: none;
    border-radius: 0;
    page-break-inside: avoid;
    break-inside: avoid-page;
  }

  .inr-section {
    background: white;
    border: 1px solid var(--color-hairline);
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-sm);
    padding: var(--spacing-xs);
  }

  .inr-chart {
    height: 128px;
  }

  .patient-table,
  .dose-section,
  .adr-section {
    margin-bottom: var(--spacing-sm);
  }

  .patient-table td {
    padding: 4px 6px;
  }

  .dose-table th,
  .dose-table td {
    padding: 4px;
  }

  .new-dose-row td {
    background: #e6faf9 !important;
  }

  .dose-instructions :deep(.option-card) {
    break-inside: avoid;
    padding: var(--spacing-sm);
  }

  .dose-instructions :deep(.option-header) {
    margin-bottom: var(--spacing-xs);
    gap: var(--spacing-xs);
  }

  .dose-instructions :deep(.option-description),
  .dose-instructions :deep(.summary-line) {
    font-size: var(--typography-caption-size);
  }

  .dose-instructions :deep(.schedule-grid) {
    gap: 2px;
    margin-bottom: var(--spacing-xs);
  }

  .dose-instructions :deep(.day-content) {
    min-height: 46px;
    padding: 4px;
  }

  .dose-instructions :deep(.pills-summary) {
    padding: var(--spacing-xs);
  }

  .adr-section,
  .appointment-info {
    padding: var(--spacing-xs);
  }

.footer-section {
    gap: var(--spacing-xs);
    padding-top: var(--spacing-xs);
  }

  .signature-area {
    gap: var(--spacing-sm);
  }

  .signature-box {
    border-width: 1px;
  }

  .box-header {
    padding: var(--spacing-xs) var(--spacing-sm);
  }

  .box-body {
    padding: var(--spacing-sm);
  }

  .notes-space {
    height: 24px;
  }
}
</style>
