<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { CalendarPlus } from 'lucide-vue-next';
import { computed, onMounted, ref, watch } from 'vue';
import StatusBadge from '#/components/shared/StatusBadge.vue';
import type { AppointmentDayLoad, AppointmentInput, WfAppointment } from '#/types/appointment.ts';
import { dateInputToday, formatThaiDate, sortAppointments } from '#/utils/clinic.ts';

const props = defineProps<{ hn: string }>();
const appointments = ref<WfAppointment[]>([]);
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const showForm = ref(false);
const form = ref<AppointmentInput>({
  hn: props.hn,
  apptDate: dateInputToday(),
  apptType: 'clinic_visit',
  notes: '',
});
const orderedAppointments = computed(() => sortAppointments(appointments.value));
const loadingDayLoad = ref(false);
const appointmentDayLoad = ref<AppointmentDayLoad | null>(null);

async function fetchAppointments() {
  loading.value = true;
  error.value = null;
  try {
    appointments.value = await invoke<WfAppointment[]>('get_appointments', { hn: props.hn });
  } catch (invokeError) {
    error.value = String(invokeError);
  } finally {
    loading.value = false;
  }
}

async function submitAppointment() {
  saving.value = true;
  error.value = null;
  try {
    await invoke<number>('schedule_appointment', { appt: { ...form.value, hn: props.hn } });
    showForm.value = false;
    form.value = { hn: props.hn, apptDate: dateInputToday(), apptType: 'clinic_visit', notes: '' };
    await fetchAppointments();
  } catch (invokeError) {
    error.value = String(invokeError);
  } finally {
    saving.value = false;
  }
}

async function refreshDayLoad() {
  if (!form.value.apptDate) {
    appointmentDayLoad.value = null;
    return;
  }

  loadingDayLoad.value = true;
  try {
    appointmentDayLoad.value = await invoke<AppointmentDayLoad>('get_appointment_day_load', {
      apptDate: form.value.apptDate,
    });
  } catch {
    appointmentDayLoad.value = null;
  } finally {
    loadingDayLoad.value = false;
  }
}

watch(
  () => form.value.apptDate,
  () => {
    void refreshDayLoad();
  },
);

onMounted(() => {
  void fetchAppointments();
  void refreshDayLoad();
});
</script>

<template>
  <section class="card appointment-section">
    <div class="section-header">
      <div>
        <h3 class="h5">&#x0E15;&#x0E32;&#x0E23;&#x0E32;&#x0E07;&#x0E19;&#x0E31;&#x0E14;&#x0E2B;&#x0E21;&#x0E32;&#x0E22;</h3>
        <p class="caption section-meta">&#x0E19;&#x0E31;&#x0E14;&#x0E17;&#x0E35;&#x0E48;&#x0E1C;&#x0E48;&#x0E32;&#x0E19;&#x0E21;&#x0E32;&#x0E41;&#x0E25;&#x0E30;&#x0E17;&#x0E35;&#x0E48;&#x0E08;&#x0E30;&#x0E16;&#x0E36;&#x0E07;</p>
      </div>
      <button type="button" class="btn btn-primary" @click="showForm = !showForm"><CalendarPlus :size="16" /> + &#x0E19;&#x0E31;&#x0E14;&#x0E2B;&#x0E21;&#x0E32;&#x0E22;&#x0E43;&#x0E2B;&#x0E21;&#x0E48;</button>
    </div>

    <form v-if="showForm" class="appointment-form card-feature-yellow" @submit.prevent="submitAppointment">
      <div class="form-grid">
        <label class="form-field"><span class="caption section-meta">&#x0E27;&#x0E31;&#x0E19;&#x0E17;&#x0E35;&#x0E48;&#x0E19;&#x0E31;&#x0E14;</span><input v-model="form.apptDate" class="input" type="date" required /><span v-if="loadingDayLoad" class="caption section-meta">กำลังโหลดคิวนัด...</span><span v-else-if="appointmentDayLoad" class="caption section-meta">วันดังกล่าวมีนัดแล้ว {{ appointmentDayLoad.scheduledCount }} คน</span></label>
        <label class="form-field"><span class="caption section-meta">&#x0E1B;&#x0E23;&#x0E30;&#x0E40;&#x0E20;&#x0E17;&#x0E01;&#x0E32;&#x0E23;&#x0E19;&#x0E31;&#x0E14;</span><select v-model="form.apptType" class="input"><option value="clinic_visit">&#x0E15;&#x0E23;&#x0E27;&#x0E08;&#x0E04;&#x0E25;&#x0E34;&#x0E19;&#x0E34;&#x0E01;</option><option value="inr_check">&#x0E15;&#x0E23;&#x0E27;&#x0E08; INR</option><option value="urgent">&#x0E40;&#x0E23;&#x0E48;&#x0E07;&#x0E14;&#x0E48;&#x0E27;&#x0E19;</option></select></label>
      </div>
      <label class="form-field"><span class="caption section-meta">&#x0E2B;&#x0E21;&#x0E32;&#x0E22;&#x0E40;&#x0E2B;&#x0E15;&#x0E38;</span><textarea v-model="form.notes" class="input form-textarea" rows="3" /></label>
      <div class="form-actions"><button type="button" class="btn btn-ghost" @click="showForm = false">&#x0E22;&#x0E01;&#x0E40;&#x0E25;&#x0E34;&#x0E01;</button><button type="submit" class="btn btn-primary" :disabled="saving">{{ saving ? '&#x0E01;&#x0E33;&#x0E25;&#x0E31;&#x0E07;&#x0E1A;&#x0E31;&#x0E19;&#x0E17;&#x0E36;&#x0E01;...' : '&#x0E1A;&#x0E31;&#x0E19;&#x0E17;&#x0E36;&#x0E01;&#x0E19;&#x0E31;&#x0E14;' }}</button></div>
    </form>

    <div v-if="loading" class="empty-state card-feature-pink-dark"><p class="body-sm">&#x0E01;&#x0E33;&#x0E25;&#x0E31;&#x0E07;&#x0E42;&#x0E2B;&#x0E25;&#x0E14;...</p></div>
    <div v-else-if="error" class="badge badge-danger error-box">{{ error }}</div>
    <div v-else-if="!orderedAppointments.length" class="empty-state"><p class="body-sm section-meta">&#x0E22;&#x0E31;&#x0E07;&#x0E44;&#x0E21;&#x0E48;&#x0E21;&#x0E35;&#x0E01;&#x0E32;&#x0E23;&#x0E19;&#x0E31;&#x0E14;&#x0E2B;&#x0E21;&#x0E32;&#x0E22;</p></div>

    <div v-else class="timeline-list">
      <article v-for="appointment in orderedAppointments" :key="appointment.id" class="timeline-item">
        <div class="timeline-dot" :data-status="appointment.status" />
        <div class="timeline-body">
          <div class="timeline-row">
            <div>
              <p class="body-sm-medium">{{ formatThaiDate(appointment.apptDate) }}</p>
              <p class="caption section-meta">{{ appointment.apptType || '-' }}</p>
            </div>
            <StatusBadge variant="appointment" :status="appointment.status" />
          </div>
          <p v-if="appointment.notes" class="caption section-meta">{{ appointment.notes }}</p>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.appointment-section,
.appointment-form,
.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}
.section-header,
.timeline-row {
  display: flex;
  justify-content: space-between;
  gap: var(--spacing-md);
}
.section-meta { color: var(--color-slate); }
.form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--spacing-md); }
.form-textarea { min-height: 8rem; resize: vertical; }
.form-actions { display: flex; justify-content: flex-end; gap: var(--spacing-sm); }
.empty-state { display: grid; place-items: center; min-height: 10rem; }
.error-box { width: fit-content; }
.timeline-list { display: flex; flex-direction: column; gap: var(--spacing-md); }
.timeline-item { display: grid; grid-template-columns: auto 1fr; gap: var(--spacing-md); }
.timeline-dot { width: var(--spacing-md); height: var(--spacing-md); margin-top: var(--spacing-xs); border-radius: var(--rounded-full); background: var(--color-stone); }
.timeline-dot[data-status='scheduled'] { background: var(--color-pink-600); }
.timeline-dot[data-status='completed'] { background: var(--color-inr-safe); }
.timeline-dot[data-status='missed'] { background: var(--color-inr-high); }
.timeline-body { display: flex; flex-direction: column; gap: var(--spacing-xs); padding-bottom: var(--spacing-md); border-bottom: 1px solid var(--color-hairline-soft); }
</style>
