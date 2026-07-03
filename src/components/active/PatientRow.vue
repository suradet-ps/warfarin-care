<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import { FilePenLine, FileText } from 'lucide-vue-next'
import AlertBadge from '#/components/active/AlertBadge.vue'
import InrStatusBadge from '#/components/active/InrStatusBadge.vue'
import TtrBadge from '#/components/active/TtrBadge.vue'
import type { PatientAlert } from '#/types/alert'
import type { ActivePatientSummary } from '#/types/patient'
import { daysUntil, formatThaiDate, patientFullName } from '#/utils/clinic'

const props = defineProps<{
  summary: ActivePatientSummary
  alerts?: PatientAlert[]
}>()

const emit = defineEmits<{
  openVisit: [hn: string]
}>()

const inrDays = computed(() => {
  const delta = daysUntil(props.summary.latestInr?.date ?? undefined)
  return delta === null ? null : Math.abs(delta)
})

const nextApptDelta = computed(() => daysUntil(props.summary.nextAppointment ?? undefined))
const appointmentText = computed(() => {
  if (!props.summary.nextAppointment) return 'ยังไม่มีนัด'
  const delta = nextApptDelta.value
  if (delta === null) return formatThaiDate(props.summary.nextAppointment)
  if (delta < 0) return `${formatThaiDate(props.summary.nextAppointment)} · เกินนัด ${Math.abs(delta)} วัน`
  if (delta === 0) return `${formatThaiDate(props.summary.nextAppointment)} · วันนี้`
  return `${formatThaiDate(props.summary.nextAppointment)} · อีก ${delta} วัน`
})
</script>

<template>
  <tr>
    <td>
      <div class="cell-stack">
        <RouterLink :to="`/patient/${summary.patient.hn}`" class="hn-link body-sm-medium" :aria-label="`ดูข้อมูลผู้ป่วย HN ${summary.patient.hn}`">{{ summary.patient.hn }}</RouterLink>
        <span class="body-sm">{{ patientFullName(summary.hosxpInfo) }}</span>
      </div>
    </td>
    <td>
      <InrStatusBadge :inr-value="summary.latestInr?.value" :target-low="summary.patient.targetInrLow" :target-high="summary.patient.targetInrHigh" :days-since-last-inr="inrDays" />
    </td>
    <td>{{ summary.currentDoseMgday !== null && summary.currentDoseMgday !== undefined ? `${(summary.currentDoseMgday * 7).toFixed(1)} mg/สัปดาห์ (เฉลี่ย ${summary.currentDoseMgday.toFixed(1)} mg/วัน)` : '-' }}</td>
    <td><TtrBadge :ttr="summary.ttr6months" /></td>
    <td>{{ appointmentText }}</td>
    <td><AlertBadge :alerts="alerts ?? []" /></td>
    <td>
      <div class="row-actions">
        <RouterLink :to="`/patient/${summary.patient.hn}`" class="btn btn-ghost action-button" :aria-label="`ดูข้อมูลผู้ป่วย ${patientFullName(summary.hosxpInfo)}`"><FileText :size="16" aria-hidden="true" />ดูข้อมูล</RouterLink>
        <button type="button" class="btn btn-secondary action-button" @click="emit('openVisit', summary.patient.hn)" :aria-label="`บันทึกการทำคลินิก ${patientFullName(summary.hosxpInfo)}`"><FilePenLine :size="16" aria-hidden="true" />บันทึก</button>
      </div>
    </td>
  </tr>
</template>

<style scoped>
.cell-stack { display: flex; flex-direction: column; gap: var(--spacing-xxs); }
.hn-link { color: var(--color-ink); text-decoration: none; }
.row-actions { display: flex; justify-content: flex-end; gap: var(--spacing-xs); flex-wrap: wrap; }
.action-button { min-width: max-content; }
</style>
