<script setup lang="ts">
import { computed } from 'vue';
import type { DispensingRecord } from '#/types/dispensing.ts';
import type { DoseSchedule } from '#/types/visit.ts';
import {
  aggregateDispensingByVisit,
  doseDayLabels,
  formatThaiDate,
  normalizeDoseSchedule,
} from '#/utils/clinic.ts';

const props = defineProps<{ records: DispensingRecord[] }>();
const orderedVisits = computed(() => aggregateDispensingByVisit(props.records));

function activeDays(scheduleInput?: Partial<DoseSchedule> | string | null) {
  const schedule = normalizeDoseSchedule(scheduleInput);
  const labels = Object.entries(schedule)
    .filter(([, value]) => Number(value) > 0)
    .map(([day]) => doseDayLabels[day as keyof typeof doseDayLabels]);
  return labels.join(' ') || '-';
}

function regimenSummary(items: DispensingRecord[]): string {
  return items
    .map((item) => `${item.strength} x ${item.parsedDose?.tabletsPerDose?.toFixed(2) ?? '?'} เม็ด`)
    .join(' + ');
}
</script>

<template>
  <section class="card dispensing-section">
    <div>
      <h3 class="h5">&#x0E1B;&#x0E23;&#x0E30;&#x0E27;&#x0E31;&#x0E15;&#x0E34;&#x0E01;&#x0E32;&#x0E23;&#x0E08;&#x0E48;&#x0E32;&#x0E22;&#x0E22;&#x0E32;</h3>
      <p class="body-sm section-meta">&#x0E02;&#x0E49;&#x0E2D;&#x0E21;&#x0E39;&#x0E25;&#x0E08;&#x0E32;&#x0E01; HOSxP &#x0E1E;&#x0E23;&#x0E49;&#x0E2D;&#x0E21;&#x0E01;&#x0E32;&#x0E23;&#x0E15;&#x0E35;&#x0E04;&#x0E27;&#x0E32;&#x0E21;&#x0E27;&#x0E34;&#x0E18;&#x0E35;&#x0E43;&#x0E0A;&#x0E49;&#x0E22;&#x0E32;&#x0E40;&#x0E1B;&#x0E47;&#x0E19; mg/week</p>
    </div>
    <table class="table">
      <thead>
        <tr>
          <th>&#x0E27;&#x0E31;&#x0E19;&#x0E17;&#x0E35;&#x0E48;</th>
          <th>VN/AN</th>
          <th>&#x0E23;&#x0E32;&#x0E22;&#x0E01;&#x0E32;&#x0E23;&#x0E22;&#x0E32;</th>
          <th>&#x0E27;&#x0E34;&#x0E18;&#x0E35;&#x0E43;&#x0E0A;&#x0E49;&#x0E22;&#x0E32;</th>
          <th>&#x0E1C;&#x0E25;&#x0E23;&#x0E27;&#x0E21;&#x0E15;&#x0E48;&#x0E2D; visit</th>
          <th>&#x0E08;&#x0E33;&#x0E19;&#x0E27;&#x0E19;</th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="orderedVisits.length === 0">
          <td colspan="6" class="empty-cell">&#x0E44;&#x0E21;&#x0E48;&#x0E21;&#x0E35;&#x0E1B;&#x0E23;&#x0E30;&#x0E27;&#x0E31;&#x0E15;&#x0E34;&#x0E01;&#x0E32;&#x0E23;&#x0E08;&#x0E48;&#x0E32;&#x0E22;&#x0E22;&#x0E32;</td>
        </tr>
        <tr v-for="visit in orderedVisits" :key="visit.visitKey" class="comparison-row">
          <td>{{ formatThaiDate(visit.vstdate) }}</td>
          <td>{{ visit.vn || visit.an || '-' }}</td>
          <td>
            <div class="drug-stack">
              <span class="body-sm-medium">{{ regimenSummary(visit.items) }}</span>
              <span class="caption section-meta">{{ visit.items.length }} รายการ warfarin</span>
            </div>
          </td>
          <td>
            <div class="usage-stack">
              <span class="body-sm">{{ visit.usageTextSummary }}</span>
              <span class="caption section-meta">
                มาตรฐาน: {{ visit.items.map((item) => item.drugusageCode).filter(Boolean).join(', ') || '-' }}
                | พิเศษ: {{ visit.items.map((item) => item.spUseCode).filter(Boolean).join(', ') || '-' }}
              </span>
            </div>
          </td>
          <td>
            <div v-if="visit.mgPerWeek > 0" class="calc-stack">
              <span class="body-sm-medium">{{ visit.mgPerWeek.toFixed(1) }} mg/week</span>
              <span class="caption section-meta">{{ visit.mgPerDayAverage.toFixed(2) }} mg/day เฉลี่ย</span>
              <span class="caption section-meta">{{ activeDays(visit.combinedSchedule) }}</span>
              <span v-if="visit.parseNotes.length" class="caption calc-note">{{ visit.parseNotes.join(' | ') }}</span>
            </div>
            <div v-else class="usage-stack">
              <span class="body-sm calc-missing">คำนวณอัตโนมัติไม่ได้</span>
              <span v-if="visit.parseNotes.length" class="caption calc-note">{{ visit.parseNotes.join(' | ') }}</span>
            </div>
          </td>
          <td>{{ visit.items.reduce((sum, item) => sum + item.qty, 0).toFixed(0) }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.dispensing-section { display: flex; flex-direction: column; gap: var(--spacing-lg); }
.section-meta { color: var(--color-slate); }
.empty-cell { text-align: center; color: var(--color-stone); }
.comparison-row td { border-bottom-color: var(--color-hairline-soft); }
.drug-stack, .usage-stack, .calc-stack { display: flex; flex-direction: column; gap: 2px; }
.calc-note { color: var(--color-coral-500); }
.calc-missing { color: var(--color-inr-high); }
</style>
