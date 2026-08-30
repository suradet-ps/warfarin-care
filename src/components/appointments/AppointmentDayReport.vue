<script lang="ts">
export interface AppointmentDayRow {
  patientName: string;
  hn: string;
  statusText: string;
  lastVisitDate: string;
  phone: string;
  notes: string;
}
</script>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useSettingsStore } from '#/stores/settings.ts';
import { formatThaiDate } from '#/utils/clinic.ts';

const props = defineProps<{ date: string; rows: AppointmentDayRow[] }>();

const settingsStore = useSettingsStore();
const hospitalName = ref('Warfarin Care');

void settingsStore.loadSettings().then(() => {
  hospitalName.value = settingsStore.hospitalName || hospitalName.value;
});

const generatedAt = new Date().toLocaleString('th-TH', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
});

const PAGE_HEIGHT_MM = 297;
const PAGE_PADDING_MM = 8;
const CONTENT_HEIGHT_MM = PAGE_HEIGHT_MM - PAGE_PADDING_MM * 2;

function mmToPx(mm: number): number {
  return (mm * 96) / 25.4;
}

interface ReportPage {
  rows: AppointmentDayRow[];
  startIndex: number;
  pageNumber: number;
  isLast: boolean;
}

interface ReportMetrics {
  firstPageHeaderPx: number;
  continuationHeaderPx: number;
  footerPx: number;
  rowHeights: number[];
}

const measured = ref(false);
const measureRoot = ref<HTMLElement | null>(null);
const measureContinuationHeader = ref<HTMLElement | null>(null);
const measureFooter = ref<HTMLElement | null>(null);
const metrics = ref<ReportMetrics | null>(null);

const pages = computed<ReportPage[]>(() => {
  if (!metrics.value || props.rows.length === 0) {
    return [];
  }
  const { firstPageHeaderPx, continuationHeaderPx, footerPx, rowHeights } = metrics.value;
  const total = props.rows.length;
  const contentPx = Math.floor(mmToPx(CONTENT_HEIGHT_MM));

  const rowSum = (start: number, end: number): number => {
    let sum = 0;
    for (let index = start; index < end; index += 1) {
      sum += rowHeights[index] ?? 0;
    }
    return sum;
  };

  const rowsThatFit = (start: number, available: number): number => {
    let used = 0;
    let count = 0;
    while (start + count < total) {
      const height = rowHeights[start + count] ?? 0;
      if (used + height > available) {
        break;
      }
      used += height;
      count += 1;
    }
    return count;
  };

  const result: ReportPage[] = [];
  let start = 0;
  let pageNumber = 1;
  while (start < total) {
    const headerPx = start === 0 ? firstPageHeaderPx : continuationHeaderPx;
    const available = contentPx - headerPx;
    if (rowSum(start, total) + footerPx <= available) {
      result.push({ rows: props.rows.slice(start), startIndex: start, pageNumber, isLast: true });
      break;
    }
    const count = Math.max(rowsThatFit(start, available), 1);
    result.push({
      rows: props.rows.slice(start, start + count),
      startIndex: start,
      pageNumber,
      isLast: false,
    });
    start += count;
    pageNumber += 1;
  }
  return result;
});

async function runMeasurement() {
  measured.value = false;
  await nextTick();
  await document.fonts.ready;
  const root = measureRoot.value;
  const continuationHeader = measureContinuationHeader.value;
  const footer = measureFooter.value;
  if (!(root && continuationHeader && footer) || props.rows.length === 0) {
    metrics.value = null;
    return;
  }
  const table = root.querySelector('table');
  const rows = Array.from(root.querySelectorAll<HTMLElement>('tbody tr'));
  const firstRow = rows[0];
  if (!(table && firstRow)) {
    metrics.value = null;
    return;
  }
  const contentTop = root.getBoundingClientRect().top + parseFloat(getComputedStyle(root).paddingTop);
  const firstRowTop = firstRow.getBoundingClientRect().top;
  const theadPx = firstRowTop - table.getBoundingClientRect().top;
  const continuationStyle = getComputedStyle(continuationHeader);
  metrics.value = {
    firstPageHeaderPx: firstRowTop - contentTop,
    continuationHeaderPx:
      continuationHeader.offsetHeight + parseFloat(continuationStyle.marginBottom) + theadPx,
    footerPx: footer.offsetHeight,
    rowHeights: rows.map((row) => row.offsetHeight),
  };
  measured.value = true;
}

watch(
  [() => props.rows, hospitalName],
  () => {
    void runMeasurement();
  },
  { immediate: true },
);
</script>

<template>
  <div class="report-pages">
    <div v-if="!measured || !pages.length" ref="measureRoot" class="measure-pass">
      <div class="report-header">
        <div class="report-title">
          <strong>Warfarin Care</strong>
          <span class="report-subtitle">คลินิกวาร์ฟาริน {{ hospitalName }}</span>
        </div>
        <div class="report-kind">รายงานการนัดหมาย</div>
      </div>

      <div class="report-meta">
        <div class="meta-block">
          <span class="label">วันที่นัดหมาย</span>
          <strong class="meta-value">{{ formatThaiDate(props.date) }}</strong>
        </div>
        <div class="meta-block">
          <span class="label">จำนวนผู้ป่วย</span>
          <strong class="meta-value">{{ props.rows.length }} คน</strong>
        </div>
        <div class="meta-block meta-block-end">
          <span class="label">พิมพ์เมื่อ</span>
          <span class="meta-value meta-value-muted">{{ generatedAt }}</span>
        </div>
      </div>

      <table class="report-table">
        <thead>
          <tr>
            <th class="col-index">ลำดับ</th>
            <th>ผู้ป่วย</th>
            <th>มาโรงพยาบาลล่าสุด</th>
            <th>สถานะวันนัด</th>
            <th>เบอร์โทรศัพท์</th>
            <th>หมายเหตุ</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, index) in props.rows" :key="row.hn">
            <td class="col-index">{{ index + 1 }}</td>
            <td>
              <div class="patient-cell">
                <span class="patient-name">{{ row.patientName }}</span>
                <span class="patient-hn">HN {{ row.hn }}</span>
              </div>
            </td>
            <td>{{ row.lastVisitDate }}</td>
            <td>{{ row.statusText }}</td>
            <td>{{ row.phone }}</td>
            <td>{{ row.notes }}</td>
          </tr>
        </tbody>
      </table>

      <div ref="measureContinuationHeader" class="continuation-header">
        รายงานการนัดหมาย · {{ formatThaiDate(props.date) }}
      </div>

      <div ref="measureFooter" class="report-footer">
        <div class="signature-box">
          <span class="label">ผู้จัดทำรายงาน</span>
          <div class="signature-line">
            <span>ลงชื่อ</span>
            <span class="line-space"></span>
          </div>
        </div>
        <div class="signature-box">
          <span class="label">ผู้ตรวจสอบ</span>
          <div class="signature-line">
            <span>ลงชื่อ</span>
            <span class="line-space"></span>
          </div>
        </div>
      </div>
    </div>

    <template v-else>
      <div v-for="page in pages" :key="page.pageNumber" class="report-page">
        <div v-if="page.pageNumber === 1" class="report-header">
          <div class="report-title">
            <strong>Warfarin Care</strong>
            <span class="report-subtitle">คลินิกวาร์ฟาริน {{ hospitalName }}</span>
          </div>
          <div class="report-kind">รายงานการนัดหมาย</div>
        </div>
        <div v-else class="continuation-header">
          <span>รายงานการนัดหมาย · {{ formatThaiDate(props.date) }}</span>
          <span>หน้า {{ page.pageNumber }}/{{ pages.length }}</span>
        </div>

        <div v-if="page.pageNumber === 1" class="report-meta">
          <div class="meta-block">
            <span class="label">วันที่นัดหมาย</span>
            <strong class="meta-value">{{ formatThaiDate(props.date) }}</strong>
          </div>
          <div class="meta-block">
            <span class="label">จำนวนผู้ป่วย</span>
            <strong class="meta-value">{{ props.rows.length }} คน</strong>
          </div>
          <div class="meta-block meta-block-end">
            <span class="label">พิมพ์เมื่อ</span>
            <span class="meta-value meta-value-muted">{{ generatedAt }}</span>
          </div>
        </div>

        <table class="report-table">
          <thead>
            <tr>
              <th class="col-index">ลำดับ</th>
              <th>ผู้ป่วย</th>
              <th>มาโรงพยาบาลล่าสุด</th>
              <th>สถานะวันนัด</th>
              <th>เบอร์โทรศัพท์</th>
              <th>หมายเหตุ</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, index) in page.rows" :key="row.hn">
              <td class="col-index">{{ page.startIndex + index + 1 }}</td>
              <td>
                <div class="patient-cell">
                  <span class="patient-name">{{ row.patientName }}</span>
                  <span class="patient-hn">HN {{ row.hn }}</span>
                </div>
              </td>
              <td>{{ row.lastVisitDate }}</td>
              <td>{{ row.statusText }}</td>
              <td>{{ row.phone }}</td>
              <td>{{ row.notes }}</td>
            </tr>
          </tbody>
        </table>

        <div v-if="page.isLast" class="report-footer">
          <div class="signature-box">
            <span class="label">ผู้จัดทำรายงาน</span>
            <div class="signature-line">
              <span>ลงชื่อ</span>
              <span class="line-space"></span>
            </div>
          </div>
          <div class="signature-box">
            <span class="label">ผู้ตรวจสอบ</span>
            <div class="signature-line">
              <span>ลงชื่อ</span>
              <span class="line-space"></span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.measure-pass,
.report-page {
  width: 210mm;
  max-width: 210mm;
  padding: 8mm;
  box-sizing: border-box;
  background: var(--color-canvas);
  font-family: var(--font-family-primary);
  color: var(--color-ink);
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}

.report-page {
  height: 297mm;
  overflow: hidden;
  position: relative;
}

.report-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: var(--spacing-md);
  padding-bottom: var(--spacing-sm);
  border-bottom: 3px solid var(--color-primary);
  margin-bottom: var(--spacing-md);
}

.report-title {
  display: flex;
  flex-direction: column;
}

.report-title strong {
  font-size: var(--typography-heading-4-size);
  font-weight: 700;
  color: var(--color-primary);
  line-height: 1.2;
}

.report-subtitle {
  font-size: var(--typography-body-md-size);
  color: var(--color-slate);
}

.report-kind {
  font-size: var(--typography-subtitle-size);
  font-weight: 600;
  color: var(--color-ink);
  white-space: nowrap;
}

.continuation-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--spacing-md);
  padding-bottom: var(--spacing-xs);
  border-bottom: 2px solid var(--color-primary);
  margin-bottom: var(--spacing-sm);
  font-size: var(--typography-body-md-medium-size);
  font-weight: var(--typography-body-md-medium-weight);
  color: var(--color-primary);
}

.report-meta {
  display: flex;
  align-items: stretch;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.meta-block {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 9rem;
  padding: var(--spacing-sm) var(--spacing-md);
  border: 1px solid var(--color-hairline);
  border-radius: var(--rounded-md);
  background: var(--color-surface);
}

.meta-block-end {
  margin-left: auto;
  min-width: 12rem;
}

.meta-block .label {
  font-size: var(--typography-caption-size);
  color: var(--color-slate);
  font-weight: var(--typography-caption-bold-weight);
}

.meta-value {
  font-size: var(--typography-body-md-medium-size);
  font-weight: var(--typography-body-md-medium-weight);
}

.meta-value-muted {
  font-weight: var(--typography-body-md-weight);
  color: var(--color-slate);
}

.report-table {
  width: 100%;
  border-collapse: collapse;
}

.report-table th,
.report-table td {
  padding: 6px var(--spacing-xs);
  border: 1px solid var(--color-hairline);
  font-size: var(--typography-body-sm-size);
  line-height: var(--typography-body-sm-line-height);
  text-align: left;
  vertical-align: top;
}

.report-table th {
  background: var(--color-pink-100);
  color: var(--color-pink-900);
  font-weight: var(--typography-caption-bold-weight);
  font-size: var(--typography-caption-size);
  white-space: nowrap;
}

.report-table .col-index {
  text-align: center;
  width: 2.5rem;
}

.patient-cell {
  display: flex;
  flex-direction: column;
}

.patient-name {
  font-weight: var(--typography-body-sm-medium-weight);
}

.patient-hn {
  font-size: var(--typography-caption-size);
  color: var(--color-slate);
}

.report-footer {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--spacing-xl);
  padding-top: var(--spacing-md);
  margin-top: var(--spacing-lg);
}

.signature-box {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.signature-box .label {
  font-size: var(--typography-caption-size);
  color: var(--color-slate);
  font-weight: var(--typography-caption-bold-weight);
}

.signature-line {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: var(--typography-body-sm-size);
  color: var(--color-slate);
}

.line-space {
  flex: 1;
  border-bottom: 1px solid var(--color-ink);
}

@media print {
  .measure-pass,
  .report-page {
    width: auto;
    max-width: none;
    padding: 0;
    background: white;
  }
}
</style>