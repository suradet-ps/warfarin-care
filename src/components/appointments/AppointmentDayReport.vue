<script lang="ts">
export interface AppointmentDayRow {
  patientName: string;
  hn: string;
  apptTypeLabel: string;
  statusText: string;
  lastVisitDate: string;
  phone: string;
  notes: string;
}
</script>

<script setup lang="ts">
import { ref } from 'vue';
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
</script>

<template>
  <div class="report-sheet">
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
          <th>ประเภท</th>
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
          <td>{{ row.apptTypeLabel }}</td>
          <td>{{ row.statusText }}</td>
          <td>{{ row.phone }}</td>
          <td>{{ row.notes }}</td>
        </tr>
      </tbody>
    </table>

    <div class="report-footer">
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

<style scoped>
.report-sheet {
  width: 210mm;
  max-width: 210mm;
  margin: 0 auto;
  padding: 8mm;
  box-sizing: border-box;
  background: var(--color-canvas);
  font-family: var(--font-family-primary);
  color: var(--color-ink);
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
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
  margin-bottom: var(--spacing-md);
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
  .report-sheet {
    width: auto;
    max-width: none;
    padding: 0;
    background: white;
  }
}
</style>