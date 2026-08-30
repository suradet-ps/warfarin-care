<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { CalendarDays, CalendarRange, Clock3, Download, Users } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import AppointmentDayReport, {
  type AppointmentDayRow,
} from '#/components/appointments/AppointmentDayReport.vue';
import SearchBox from '#/components/shared/SearchBox.vue';
import { usePdfExport } from '#/composables/usePdfExport.ts';
import type { WfAppointment } from '#/types/appointment.ts';
import type { ActivePatientSummary } from '#/types/patient.ts';
import { daysUntil, formatThaiDate, patientFullName } from '#/utils/clinic.ts';

const loading = ref(false);
const error = ref<string | null>(null);
const searchQuery = ref('');
const selectedDate = ref('');
const appointments = ref<WfAppointment[]>([]);
const summaries = ref<ActivePatientSummary[]>([]);

const patientMap = computed(
  () => new Map(summaries.value.map((summary) => [summary.patient.hn, summary])),
);

const today = () => new Date().toISOString().slice(0, 10);

const dateBuckets = computed(() => {
  const buckets = new Map<
    string,
    { date: string; count: number; overdueCount: number; urgentCount: number }
  >();
  const cutoff = today();
  const countedHns = new Map<string, Set<string>>();

  for (const appointment of appointments.value) {
    // Only today/future dates are shown.
    if (appointment.apptDate < cutoff) {
      continue;
    }

    const existing = buckets.get(appointment.apptDate) ?? {
      date: appointment.apptDate,
      count: 0,
      overdueCount: 0,
      urgentCount: 0,
    };

    // Count distinct patients per day - the same HN may have multiple
    // appointment rows (e.g. visit-generated + manual) on one date.
    const seen = countedHns.get(appointment.apptDate) ?? new Set<string>();
    countedHns.set(appointment.apptDate, seen);
    if (!seen.has(appointment.hn)) {
      seen.add(appointment.hn);
      existing.count += 1;
    }
    if (appointment.isOverdue === true) {
      existing.overdueCount += 1;
    }
    if (appointment.apptType === 'urgent') {
      existing.urgentCount += 1;
    }
    buckets.set(appointment.apptDate, existing);
  }

  return [...buckets.values()].sort((a, b) => a.date.localeCompare(b.date));
});

const selectedBucket = computed(() => {
  if (selectedDate.value) {
    return (
      dateBuckets.value.find((bucket) => bucket.date === selectedDate.value) ??
      dateBuckets.value[0] ??
      null
    );
  }

  return dateBuckets.value[0] ?? null;
});

const filteredAppointments = computed(() => {
  const activeDate = selectedBucket.value?.date ?? '';
  const query = searchQuery.value.trim().toLowerCase();
  const seenHns = new Set<string>();

  return appointments.value
    .filter((appointment) => (activeDate ? appointment.apptDate === activeDate : true))
    .filter((appointment) => {
      if (!query) {
        return true;
      }
      const summary = patientMap.value.get(appointment.hn);
      const fullName = patientFullName(summary?.hosxpInfo).toLowerCase();
      return appointment.hn.toLowerCase().includes(query) || fullName.includes(query);
    })
    .filter((appointment) => {
      // Deduplicate patients on the same day.
      if (seenHns.has(appointment.hn)) {
        return false;
      }
      seenHns.add(appointment.hn);
      return true;
    })
    .sort((a, b) => `${a.apptDate}-${a.hn}`.localeCompare(`${b.apptDate}-${b.hn}`));
});

const stats = computed(() => ({
  total: appointments.value.length,
  today: appointments.value.filter(
    (appointment) => (daysUntil(appointment.apptDate) ?? Number.MAX_SAFE_INTEGER) === 0,
  ).length,
  nextSevenDays: appointments.value.filter((appointment) => {
    const delta = daysUntil(appointment.apptDate);
    return delta !== null && delta >= 0 && delta <= 7;
  }).length,
  // "Actually overdue" = past + clinic ran that day + patient didn't attend.
  // The Rust backend pre-computes this in get_pending_appointments. Count
  // distinct patients so duplicate rows for the same HN are not double-counted.
  overdue: new Set(
    appointments.value.filter((appointment) => appointment.isOverdue === true).map((a) => a.hn),
  ).size,
}));

const reportRows = computed<AppointmentDayRow[]>(() =>
  filteredAppointments.value.map((appointment) => {
    const summary = patientMap.value.get(appointment.hn);
    return {
      patientName: summary ? patientFullName(summary.hosxpInfo) : appointment.hn,
      hn: appointment.hn,
      statusText: appointmentTimingText(appointment.apptDate),
      lastVisitDate: summary?.lastVisitDate ? formatThaiDate(summary.lastVisitDate) : '-',
      phone: summary?.hosxpInfo.phone?.trim() || '-',
      notes: appointment.notes?.trim() || '-',
    };
  }),
);

const reportCapture = ref<HTMLElement | null>(null);
const { exporting: exportingPdf, error: pdfError, exportPdf } = usePdfExport(reportCapture);

const canExportPdf = computed(() => reportRows.value.length > 0 && Boolean(reportCapture.value));

const defaultPdfFileName = computed(() => {
  const date = selectedBucket.value?.date ?? today();
  return `appointments-${date}.pdf`;
});

async function loadAppointments() {
  loading.value = true;
  error.value = null;
  try {
    const [pendingAppointments, activeSummaries] = await Promise.all([
      invoke<WfAppointment[]>('get_pending_appointments'),
      invoke<ActivePatientSummary[]>('get_active_patient_summaries'),
    ]);

    appointments.value = pendingAppointments;
    summaries.value = activeSummaries;
    if (!selectedDate.value) {
      selectedDate.value =
        pendingAppointments.find((appointment) => appointment.apptDate >= today())?.apptDate ?? '';
    }
  } catch (invokeError) {
    error.value = String(invokeError);
  } finally {
    loading.value = false;
  }
}

function appointmentTimingText(apptDate: string) {
  const delta = daysUntil(apptDate);
  if (delta === null) {
    return '-';
  }
  if (delta < 0) {
    return `เกินนัด ${Math.abs(delta)} วัน`;
  }
  if (delta === 0) {
    return 'วันนี้';
  }
  return `อีก ${delta} วัน`;
}

onMounted(() => {
  void loadAppointments();
});
</script>

<template>
  <div class="appointments-view">
    <section class="stats-grid">
      <article class="card stat-card">
        <div class="stat-head"><Users :size="18" /><span class="caption">นัดทั้งหมด</span></div>
        <strong class="stat-value">{{ stats.total }}</strong>
        <span class="body-sm stat-copy">นัดหมายผ่าน Warfarin Care</span>
      </article>
      <article class="card stat-card">
        <div class="stat-head"><CalendarDays :size="18" /><span class="caption">วันนี้</span></div>
        <strong class="stat-value">{{ stats.today }}</strong>
        <span class="body-sm stat-copy">นัดหมายวันนี้</span>
      </article>
      <article class="card stat-card">
        <div class="stat-head"><Clock3 :size="18" /><span class="caption">7 วันข้างหน้า</span></div>
        <strong class="stat-value">{{ stats.nextSevenDays }}</strong>
        <span class="body-sm stat-copy">สำหรับวางแผนล่วงหน้า</span>
      </article>
      <article class="card stat-card overdue-card">
        <div class="stat-head"><CalendarRange :size="18" /><span class="caption">เกินนัด</span></div>
        <strong class="stat-value">{{ stats.overdue }}</strong>
        <span class="body-sm stat-copy">ควรเร่งติดตาม</span>
      </article>
    </section>

    <div class="page-grid">
      <section class="card queue-panel">
        <div class="panel-header">
          <div>
            <h3 class="h5">ข้อมูลการนัดหมายจำแนกตามวันที่</h3>
            <p class="caption section-meta">เลือกวันที่เพื่อแสดงรายละเอียด</p>
          </div>
        </div>

        <div v-if="loading" class="empty-state body-sm">กำลังโหลด...</div>
        <div v-else-if="error" class="badge badge-danger error-box">{{ error }}</div>
        <div v-else-if="!dateBuckets.length" class="empty-state body-sm">ยังไม่มีรายการนัดหมายที่รอดำเนินการ</div>
        <div v-else class="date-bucket-list">
          <button
            v-for="bucket in dateBuckets"
            :key="bucket.date"
            type="button"
            class="date-bucket"
            :class="{ active: selectedBucket?.date === bucket.date }"
            @click="selectedDate = bucket.date"
          >
            <div>
              <p class="body-sm-medium">{{ formatThaiDate(bucket.date) }}</p>
              <p class="caption section-meta">{{ bucket.overdueCount > 0 ? `เกินนัด ${bucket.overdueCount} คน` : 'ยังอยู่ในกำหนด' }}</p>
            </div>
            <div class="bucket-metrics">
              <span class="badge badge-info">{{ bucket.count }} คน</span>
              <span v-if="bucket.urgentCount > 0" class="badge badge-danger">เร่งด่วน {{ bucket.urgentCount }}</span>
            </div>
          </button>
        </div>
      </section>

      <section class="card schedule-panel">
        <div class="panel-header panel-header-inline">
          <div>
            <h3 class="h5">รายละเอียดการนัดในวันที่เลือก</h3>
            <p class="caption section-meta">
              {{ selectedBucket ? `${formatThaiDate(selectedBucket.date)} · ${selectedBucket.count} คน` : 'ยังไม่ได้เลือกวัน' }}
            </p>
          </div>
          <div class="panel-actions">
            <button
              type="button"
              class="btn btn-secondary"
              :disabled="!canExportPdf || exportingPdf"
              @click="exportPdf(defaultPdfFileName)"
            >
              <Download :size="16" />
              {{ exportingPdf ? 'กำลังสร้าง PDF...' : 'ส่งออก PDF' }}
            </button>
            <SearchBox v-model="searchQuery" placeholder="ค้นหา HN หรือชื่อ" aria-label="ค้นหาผู้ป่วย" />
          </div>
        </div>

        <div v-if="pdfError" class="badge badge-danger error-box">{{ pdfError }}</div>

        <div v-if="loading" class="empty-state body-sm">กำลังโหลด...</div>
        <div v-else-if="error" class="empty-state body-sm">โหลดข้อมูลไม่สำเร็จ</div>
        <div v-else-if="!filteredAppointments.length" class="empty-state body-sm">ไม่พบรายการในวันที่เลือก</div>
        <div v-else class="table-wrap">
          <table class="table">
            <thead>
              <tr>
                <th>ผู้ป่วย</th>
                <th>มาโรงพยาบาลล่าสุด</th>
                <th>สถานะวันนัด</th>
                <th>เบอร์โทรศัพท์</th>
                <th>หมายเหตุ</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="appointment in filteredAppointments" :key="appointment.id">
                <td>
                  <div class="patient-cell">
                    <span class="body-sm-medium">{{ patientMap.get(appointment.hn) ? patientFullName(patientMap.get(appointment.hn)?.hosxpInfo) : appointment.hn }}</span>
                    <span class="caption section-meta">HN {{ appointment.hn }}</span>
                  </div>
                </td>
                <td>
                  <span class="body-sm">{{
                    patientMap.get(appointment.hn)?.lastVisitDate
                      ? formatThaiDate(patientMap.get(appointment.hn)?.lastVisitDate)
                      : '-'
                  }}</span>
                </td>
                <td><span class="body-sm">{{ appointmentTimingText(appointment.apptDate) }}</span></td>
                <td><span class="body-sm">{{ patientMap.get(appointment.hn)?.hosxpInfo.phone?.trim() || '-' }}</span></td>
                <td><span class="caption section-meta">{{ appointment.notes || '-' }}</span></td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>

    <div ref="reportCapture" class="report-capture" aria-hidden="true">
      <AppointmentDayReport
        v-if="selectedBucket && reportRows.length"
        :date="selectedBucket.date"
        :rows="reportRows"
      />
    </div>
  </div>
</template>

<style scoped>
.appointments-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xl);
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--spacing-md);
}

.stat-card {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.stat-head {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  color: var(--color-slate);
}

.stat-value {
  font-size: var(--typography-heading-3-size);
  font-weight: var(--typography-heading-3-weight);
  line-height: var(--typography-heading-3-line-height);
}

.overdue-card {
  background: var(--color-coral-100);
}

.page-grid {
  display: grid;
  grid-template-columns: minmax(18rem, 24rem) minmax(0, 1fr);
  gap: var(--spacing-lg);
  align-items: start;
}

.queue-panel,
.schedule-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--spacing-md);
}

.panel-header-inline {
  align-items: center;
}

.panel-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

/* The printable report is rendered off-screen and captured to PDF via
   html-to-image; it must stay out of the visual layout but keep layout. */
.report-capture {
  position: fixed;
  left: -9999px;
  top: 0;
  pointer-events: none;
  width: 210mm;
}

.date-bucket-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.date-bucket {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-md);
  width: 100%;
  padding: var(--spacing-md);
  border: 1px solid var(--color-hairline-soft);
  border-radius: var(--rounded-xl);
  background: var(--color-canvas);
  cursor: pointer;
  text-align: left;
}

.date-bucket.active {
  border-color: var(--color-pink-600);
  background: var(--color-pink-50);
  box-shadow: var(--elevation-1);
}

.bucket-metrics {
  display: flex;
  gap: var(--spacing-xs);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.table-wrap {
  overflow-x: auto;
}

.patient-cell {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xxs);
}

.empty-state {
  display: grid;
  place-items: center;
  min-height: 10rem;
  color: var(--color-slate);
}

.error-box {
  width: fit-content;
}

@media (max-width: 1100px) {
  .stats-grid,
  .page-grid {
    grid-template-columns: 1fr;
  }

  .panel-header-inline {
    flex-direction: column;
    align-items: stretch;
  }

  .panel-actions {
    flex-wrap: wrap;
  }
}
</style>