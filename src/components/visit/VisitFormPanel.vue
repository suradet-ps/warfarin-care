<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-vue-next';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import ConfirmDialog from '#/components/shared/ConfirmDialog.vue';
import DayDoseTable from '#/components/visit/DayDoseTable.vue';
import DoseOptionsPanel from '#/components/visit/DoseOptionsPanel.vue';
import { useVisitStore } from '#/stores/visit.ts';
import type { AppointmentDayLoad } from '#/types/appointment.ts';
import type { DispensingRecord } from '#/types/dispensing.ts';
import type { InrRecord } from '#/types/inr.ts';
import type { PatientDetail } from '#/types/patient.ts';
import type { DoseSuggestion, VisitInput, WfVisit } from '#/types/visit.ts';
import {
  aggregateDispensingByVisit,
  dateInputToday,
  emptyDoseSchedule,
  formatThaiDate,
  mergeDoseSchedules,
  normalizeDoseSchedule,
  scheduleAverageDose,
  scheduleWeeklyTotal,
} from '#/utils/clinic.ts';
import {
  createRegimenOptionSnapshot,
  findMatchingRegimenOption,
  getDosePeriodDays,
  jsDayToDoseDayIndex,
  regimenOptionMatchesSchedule,
} from '#/utils/regimen.ts';
import { useDoseCalculator } from '@/composables/useDoseCalculator.ts';
import type { AvailablePills, RegimenOption } from '@/types/dose.ts';

const visitStore = useVisitStore();
const { generateDoseOptions, DEFAULT_AVAILABLE_PILLS } = useDoseCalculator();

const props = defineProps<{ hn: string; editVisit?: WfVisit | null }>();
const modelValue = defineModel<boolean>({ default: false });
const emit = defineEmits<{ (e: 'saved', visitId: number): void; (e: 'updated'): void }>();

const saving = ref(false);
const error = ref<string | null>(null);
const loadingSuggestion = ref(false);
const loadingAppointmentLoad = ref(false);
const isEditMode = ref(false);
const editingVisitId = ref<number | null>(null);

const visitDate = ref(dateInputToday());
const inrValue = ref<number | null>(null);
const inrSource = ref<'lab_order' | 'lab_app_order' | 'manual'>('lab_order');
const currentDoseMgday = ref<number | null>(null);
const currentDoseDetail = ref(emptyDoseSchedule());
const newDoseMgday = ref<number | null>(null);
const newDoseDetail = ref(emptyDoseSchedule());
const nextAppointment = ref('');
const nextInrDue = ref('');
const adherence = ref<'good' | 'fair' | 'poor'>('good');
const notes = ref('');
const suggestion = ref<DoseSuggestion | null>(null);
const targetLow = ref(2.0);
const targetHigh = ref(3.0);
const latestHosxpDispense = ref<DispensingRecord | null>(null);
const latestHosxpVisit = ref<ReturnType<typeof aggregateDispensingByVisit>[number] | null>(null);
const currentDoseSource = ref<'visit' | 'hosxp' | 'manual'>('manual');
const currentDoseSourceText = ref('');
const doseOptions = ref<RegimenOption[]>([]);
const selectedDoseOptionIndex = ref<number | null>(null);
const loadingDoseOptions = ref(false);
const doseOptionsError = ref<string | null>(null);
const doseOptionsHint = ref<string | null>(null);
const availablePills = ref<AvailablePills>({ ...DEFAULT_AVAILABLE_PILLS });
const allowHalf = ref(true);
const specialDayPattern = ref<'fri-sun' | 'mon-wed-fri'>('fri-sun');
const appointmentDayLoad = ref<AppointmentDayLoad | null>(null);
const panelRef = ref<HTMLDivElement | null>(null);
const showDoseConfirm = ref(false);
const pendingSave = ref(false);

const sideEffectOptionsHigh = [
  { key: 'body_bleeding', label: 'เลือดออกตามร่างกาย' },
  { key: 'blood_urine', label: 'เลือดปนออกมาในปัสสาวะ' },
  { key: 'bleeding_gums', label: 'เลือดออกตามไรฟัน' },
  { key: 'hemoptysis', label: 'ไอเป็นเลือด' },
  { key: 'hematoma', label: 'ห้อเลือด' },
];
const sideEffectOptionsLow = [
  { key: 'headache', label: 'ปวดหัว' },
  { key: 'dizziness_fatigue', label: 'เวียนศีรษะหรืออ่อนเพลีย' },
  { key: 'faint_breath', label: 'รู้สึกหวิวหรือหายใจติดขัด' },
  { key: 'numbness_weakness', label: 'มีอาการชา หรือกล้ามเนื้ออ่อนแรง' },
  { key: 'other', label: 'อื่นๆ' },
];
const selectedSideEffects = ref<string[]>([]);

async function loadDefaults() {
  // Avoid re-running the heavy default-load work when the panel is reopened
  // for the same patient + visit-date without changes (e.g. user toggles the
  // panel a few times to peek).
  const cacheKey = {
    hn: props.hn,
    date: props.editVisit?.visitDate ?? '',
    editId: props.editVisit?.id ?? null,
  };
  if (
    lastLoaded.value &&
    lastLoaded.value.hn === cacheKey.hn &&
    lastLoaded.value.date === cacheKey.date &&
    lastLoaded.value.editId === cacheKey.editId
  ) {
    return;
  }

  loadingSuggestion.value = false;
  doseOptionsError.value = null;
  doseOptionsHint.value = null;
  doseOptions.value = [];
  selectedDoseOptionIndex.value = null;
  suggestion.value = null;

  if (props.editVisit) {
    isEditMode.value = true;
    editingVisitId.value = props.editVisit.id;
    visitDate.value = props.editVisit.visitDate;
    inrValue.value = props.editVisit.inrValue ?? null;
    inrSource.value = (props.editVisit.inrSource as typeof inrSource.value) ?? 'manual';
    currentDoseMgday.value = props.editVisit.currentDoseMgday ?? null;
    currentDoseDetail.value = normalizeDoseSchedule(props.editVisit.doseDetail);
    newDoseMgday.value = props.editVisit.newDoseMgday ?? null;
    newDoseDetail.value = normalizeDoseSchedule(props.editVisit.newDoseDetail);
    nextAppointment.value = props.editVisit.nextAppointment ?? '';
    nextInrDue.value = props.editVisit.nextInrDue ?? '';
    adherence.value = (props.editVisit.adherence as typeof adherence.value) ?? 'good';
    notes.value = props.editVisit.notes ?? '';
    selectedSideEffects.value = props.editVisit.sideEffects ?? [];

    const [patientData] = await Promise.all([
      invoke<PatientDetail>('get_patient_detail', { hn: props.hn }),
    ]);
    targetLow.value = patientData.patient.targetInrLow;
    targetHigh.value = patientData.patient.targetInrHigh;
    currentDoseSource.value = 'visit';
    currentDoseSourceText.value = 'ขนาดยาจากการบันทึกครั้งก่อน';
    return;
  }

  isEditMode.value = false;
  editingVisitId.value = null;

  try {
    const [latestInr, visits, patientData] = await Promise.all([
      invoke<InrRecord | null>('get_latest_inr', { hn: props.hn }),
      invoke<WfVisit[]>('get_visit_history', { hn: props.hn }),
      invoke<PatientDetail>('get_patient_detail', { hn: props.hn }),
    ]);
    if (latestInr) {
      inrValue.value = latestInr.value;
      inrSource.value = (latestInr.source as typeof inrSource.value) ?? 'lab_order';
    }
    const lastVisit = visits[0];
    const aggregatedVisits = aggregateDispensingByVisit(patientData.dispensingHistory ?? []);
    latestHosxpVisit.value = aggregatedVisits.find((visit) => visit.mgPerWeek > 0) ?? null;
    latestHosxpDispense.value = latestHosxpVisit.value?.items[0] ?? null;
    if (lastVisit) {
      currentDoseMgday.value = lastVisit.newDoseMgday ?? lastVisit.currentDoseMgday ?? null;
      currentDoseDetail.value = normalizeDoseSchedule(
        lastVisit.newDoseDetail ?? lastVisit.doseDetail,
      );
      currentDoseSource.value = 'visit';
      currentDoseSourceText.value = 'ใช้ขนาดยาจาก visit ล่าสุดที่บันทึกในคลินิก';
    } else if (latestHosxpVisit.value) {
      applyHosxpDose(latestHosxpVisit.value);
    } else {
      currentDoseSource.value = 'manual';
      currentDoseSourceText.value = 'ไม่พบขนาดยาเดิมที่คำนวณได้จากระบบ กรุณากรอกเอง';
    }
    targetLow.value = patientData.patient.targetInrLow;
    targetHigh.value = patientData.patient.targetInrHigh;
    applyCurrentDoseAsNew();
  } catch {
    // non-critical
  }
  lastLoaded.value = cacheKey;
}

function applyHosxpDose(visit: NonNullable<typeof latestHosxpVisit.value>) {
  currentDoseDetail.value = mergeDoseSchedules(visit.combinedSchedule);
  currentDoseMgday.value = visit.mgPerDayAverage;
  currentDoseSource.value = 'hosxp';
  currentDoseSourceText.value = `ดึงจาก HosXP วันที่ ${formatThaiDate(visit.vstdate)}${visit.usageTextSummary === '-' ? '' : `: ${visit.usageTextSummary}`}`;
  applyCurrentDoseAsNew();
}

function applyCurrentDoseAsNew() {
  newDoseDetail.value = normalizeDoseSchedule(currentDoseDetail.value);
  newDoseMgday.value = currentDoseMgday.value;
  selectedDoseOptionIndex.value = null;
}

function autoSelectMatchingDoseOption() {
  const matched = findMatchingRegimenOption(doseOptions.value, newDoseDetail.value);
  selectedDoseOptionIndex.value = matched?.index ?? null;
}

async function fetchSuggestion() {
  if (currentDoseMgday.value === null || inrValue.value === null) return;
  loadingSuggestion.value = true;
  loadingDoseOptions.value = true;
  doseOptionsError.value = null;
  doseOptionsHint.value = null;
  doseOptions.value = [];
  selectedDoseOptionIndex.value = null;

  try {
    const currentWeekly = currentDoseMgday.value * 7;

    suggestion.value = await invoke<DoseSuggestion>('suggest_dose', {
      currentDoseMgday: currentDoseMgday.value,
      currentInr: inrValue.value,
      targetLow: targetLow.value,
      targetHigh: targetHigh.value,
    });
    if (suggestion.value) {
      const suggestedWeekly = suggestion.value.suggestedDoseMgweek;
      const unchanged = Math.abs(suggestedWeekly - currentWeekly) < 0.25;
      const targetWeekly = unchanged ? currentWeekly : suggestedWeekly;

      if (unchanged) {
        newDoseMgday.value = currentWeekly / 7;
        newDoseDetail.value = JSON.parse(JSON.stringify(currentDoseDetail.value));
        doseOptionsHint.value =
          'ขนาดยาคงที่หลังปัดเศษ ระบบใส่ขนาดยาเดิมให้อัตโนมัติ และยังสามารถเลือกการ์ดวิธีกินยาที่มี mg/week เท่ากันได้';
      } else {
        newDoseMgday.value = suggestedWeekly / 7;
      }

      const daysUntilAppointment = getDosePeriodDays(visitDate.value, nextAppointment.value) ?? 28;
      const startDayOfWeek = jsDayToDoseDayIndex(new Date(`${visitDate.value}T00:00:00`).getDay());

      const options = await generateDoseOptions(
        targetWeekly,
        availablePills.value,
        allowHalf.value,
        specialDayPattern.value,
        daysUntilAppointment,
        startDayOfWeek,
      );
      doseOptions.value = options;
      autoSelectMatchingDoseOption();

      if (options.length === 0) {
        doseOptionsError.value = 'ไม่พบตัวเลือกที่เหมาะสม ลองปรับการตั้งค่ายา';
      }
    }
  } catch (e) {
    doseOptionsError.value = String(e);
  } finally {
    loadingSuggestion.value = false;
    loadingDoseOptions.value = false;
  }
}

async function refreshAppointmentDayLoad() {
  if (!nextAppointment.value) {
    appointmentDayLoad.value = null;
    return;
  }

  loadingAppointmentLoad.value = true;
  try {
    appointmentDayLoad.value = await invoke<AppointmentDayLoad>('get_appointment_day_load', {
      apptDate: nextAppointment.value,
    });
  } catch {
    appointmentDayLoad.value = null;
  } finally {
    loadingAppointmentLoad.value = false;
  }
}

function handleSelectDoseOption(index: number) {
  selectedDoseOptionIndex.value = index;
  const option = doseOptions.value[index];

  const dayMap: Record<number, string> = {
    0: 'mon',
    1: 'tue',
    2: 'wed',
    3: 'thu',
    4: 'fri',
    5: 'sat',
    6: 'sun',
  };

  const newDetail = emptyDoseSchedule();
  for (const day of option.weekly_schedule) {
    const dayKey = dayMap[day.day_index];
    if (dayKey && dayKey in newDetail) {
      newDetail[dayKey as keyof typeof newDetail] = day.is_stop_day ? 0 : day.total_dose;
    }
  }

  newDoseDetail.value = newDetail;
  newDoseMgday.value = option.weekly_dose_actual / 7;
}

const currentDoseAvg = computed(() => scheduleAverageDose(currentDoseDetail.value));
const currentDoseWeek = computed(() => scheduleWeeklyTotal(currentDoseDetail.value));
const newDoseAvg = computed(() => scheduleAverageDose(newDoseDetail.value));
const newDoseWeek = computed(() => scheduleWeeklyTotal(newDoseDetail.value));
const activeDoseOption = computed<RegimenOption | null>(() => {
  if (selectedDoseOptionIndex.value !== null) {
    const selectedOption = doseOptions.value[selectedDoseOptionIndex.value] ?? null;
    if (selectedOption && regimenOptionMatchesSchedule(selectedOption, newDoseDetail.value)) {
      return selectedOption;
    }
  }

  return findMatchingRegimenOption(doseOptions.value, newDoseDetail.value)?.option ?? null;
});

const regimenSnapshot = computed<RegimenOption>(() =>
  createRegimenOptionSnapshot({
    schedule: newDoseDetail.value,
    visitDate: visitDate.value,
    nextAppointment: nextAppointment.value,
    baseOption: activeDoseOption.value,
  }),
);

const doseChanged = computed(() => Math.abs(newDoseWeek.value - currentDoseWeek.value) >= 0.25);

watch(
  currentDoseDetail,
  () => {
    currentDoseMgday.value = currentDoseAvg.value;
  },
  { deep: true },
);
watch(
  newDoseDetail,
  () => {
    newDoseMgday.value = newDoseAvg.value;
    const selectedOption =
      selectedDoseOptionIndex.value === null
        ? null
        : (doseOptions.value[selectedDoseOptionIndex.value] ?? null);

    if (selectedOption && !regimenOptionMatchesSchedule(selectedOption, newDoseDetail.value)) {
      selectedDoseOptionIndex.value = null;
    }
  },
  { deep: true },
);
watch(nextAppointment, () => {
  void refreshAppointmentDayLoad();
});

async function handleSubmit() {
  if (!nextAppointment.value.trim()) {
    error.value = 'กรุณาระบุวันนัดครั้งต่อไปก่อนบันทึกการทำคลินิก';
    return;
  }

  if (doseChanged.value && !pendingSave.value) {
    showDoseConfirm.value = true;
    return;
  }

  showDoseConfirm.value = false;
  saving.value = true;
  error.value = null;
  try {
    const input: VisitInput = {
      hn: props.hn,
      visitDate: visitDate.value,
      inrValue: inrValue.value ?? undefined,
      inrSource: inrSource.value,
      currentDoseMgday: currentDoseMgday.value ?? undefined,
      doseDetail: currentDoseDetail.value,
      newDoseMgday: newDoseMgday.value ?? undefined,
      newDoseDetail: newDoseDetail.value,
      newDoseDescription: regimenSnapshot.value.description,
      selectedDoseOption: regimenSnapshot.value,
      doseChanged: doseChanged.value,
      nextAppointment: nextAppointment.value || undefined,
      nextInrDue: nextInrDue.value || undefined,
      adherence: adherence.value,
      sideEffects: selectedSideEffects.value.length > 0 ? selectedSideEffects.value : null,
      notes: notes.value || undefined,
    };

    if (isEditMode.value && editingVisitId.value !== null) {
      await visitStore.updateVisit(editingVisitId.value, input);
      emit('updated');
    } else {
      const visitId = await invoke<number>('save_visit', { visit: input });
      emit('saved', visitId);
    }
    modelValue.value = false;
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
    pendingSave.value = false;
  }
}

function confirmDoseChange() {
  pendingSave.value = true;
  showDoseConfirm.value = false;
  void handleSubmit();
}

function cancelDoseChange() {
  showDoseConfirm.value = false;
}

const lastLoaded = ref<{ hn: string; date: string; editId: number | null } | null>(null);

function handlePanelKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault();
    modelValue.value = false;
  }
}

watch(
  () => modelValue.value,
  (open) => {
    if (open) {
      void loadDefaults();
      void nextTick(() => {
        const panel = panelRef.value;
        if (panel) {
          const firstInput = panel.querySelector(
            'input, select, textarea, button:not([aria-hidden])',
          ) as HTMLElement | null;
          firstInput?.focus();
        }
      });
      document.addEventListener('keydown', handlePanelKeydown);
    } else {
      document.removeEventListener('keydown', handlePanelKeydown);
      pendingSave.value = false;
      showDoseConfirm.value = false;
    }
  },
);
onMounted(() => {
  if (modelValue.value) void loadDefaults();
});
onUnmounted(() => {
  document.removeEventListener('keydown', handlePanelKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="panel-overlay" @click.self="modelValue = false" role="dialog" aria-modal="true" aria-label="บันทึกการทำคลินิก">
      <div ref="panelRef" class="visit-panel card">
        <div class="panel-header">
          <h3 class="h4">{{ isEditMode ? 'แก้ไขประวัติการทำคลินิก' : 'บันทึกการทำคลินิก' }}</h3>
          <button class="btn btn-ghost" @click="modelValue = false" aria-label="ปิด面板"><X :size="18" /></button>
        </div>

        <div v-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">{{ error }}</div>

        <div class="panel-body">
          <div class="form-row">
            <label class="form-field">
              <span class="caption label">วันที่</span>
              <input class="input" type="date" v-model="visitDate" aria-label="วันที่ทำคลินิก" />
            </label>
            <label class="form-field">
              <span class="caption label">ค่า INR</span>
              <input class="input" type="number" step="0.1" v-model.number="inrValue" aria-label="ค่า INR วัดได้" />
            </label>
          </div>

          <div class="form-section">
            <p class="caption label">ตารางยาเดิม</p>
            <div class="dose-source-card">
              <div>
                <p class="body-sm-medium">{{ currentDoseSource === 'hosxp' ? 'ขนาดยาเดิมจาก HosXP' : currentDoseSource === 'visit' ? 'ขนาดยาเดิมจาก visit ล่าสุด' : 'กรอกขนาดยาเดิมเอง' }}</p>
                <p class="caption dose-source-text">{{ currentDoseSourceText }}</p>
              </div>
              <button
                v-if="latestHosxpVisit && currentDoseSource !== 'hosxp'"
                type="button"
                class="btn btn-secondary btn-compact"
                @click="applyHosxpDose(latestHosxpVisit)"
              >
                ใช้ค่าจาก HosXP ล่าสุด
              </button>
            </div>
            <DayDoseTable v-model="currentDoseDetail" />
            <p class="caption" style="color: var(--color-slate)">รวม: {{ currentDoseWeek.toFixed(1) }} mg/สัปดาห์ (เฉลี่ย {{ currentDoseAvg.toFixed(2) }} mg/วัน)</p>
            <p v-if="latestHosxpVisit?.parseNotes.length" class="caption dose-warning">{{ latestHosxpVisit.parseNotes.join(' | ') }}</p>
          </div>

          <div class="suggestion-row">
            <button class="btn btn-primary" @click="fetchSuggestion" :disabled="loadingSuggestion || inrValue === null">
              {{ loadingSuggestion ? 'กำลังคำนวณ...' : 'คำนวณขนาดยา' }}
            </button>
            <span v-if="suggestion" class="body-sm">
              แนะนำ: <strong>{{ suggestion.suggestedDoseMgweek.toFixed(1) }} mg/สัปดาห์</strong> &mdash; {{ suggestion.recommendation }}
            </span>
          </div>

          <div v-if="doseOptions.length > 0 || loadingDoseOptions || doseOptionsError" class="dose-options-section">
            <p class="caption label">ตัวเลือกตารางการกินยา</p>
            <DoseOptionsPanel
              :options="doseOptions"
              :selected-index="selectedDoseOptionIndex"
              :loading="loadingDoseOptions"
              @select="handleSelectDoseOption"
            />
            <p v-if="doseOptionsError" class="caption" style="color: var(--color-coral-500)">{{ doseOptionsError }}</p>
            <p v-else-if="doseOptionsHint" class="caption" style="color: var(--color-slate)">{{ doseOptionsHint }}</p>
            <p v-if="selectedDoseOptionIndex !== null" class="body-sm-medium" style="color: var(--color-inr-safe)">
              ✓ เลือกตัวเลือก {{ selectedDoseOptionIndex + 1 }} แล้ว ตารางยาใหม่จะถูกอัพเดทด้านล่าง
            </p>
          </div>

          <div class="form-section">
            <p class="caption label">ตารางยาใหม่</p>
            <DayDoseTable v-model="newDoseDetail" />
            <p class="caption" style="color: var(--color-slate)">รวม: {{ newDoseWeek.toFixed(1) }} mg/สัปดาห์ (เฉลี่ย {{ newDoseAvg.toFixed(2) }} mg/วัน)</p>
          </div>

          <div class="form-row">
            <label class="form-field">
              <span class="caption label">นัดครั้งต่อไป</span>
              <input class="input" type="date" v-model="nextAppointment" required />
              <span v-if="loadingAppointmentLoad" class="caption helper-text">กำลังโหลดคิวนัดของวันดังกล่าว...</span>
              <span v-else-if="appointmentDayLoad" class="caption helper-text">
                วันที่ {{ formatThaiDate(appointmentDayLoad.apptDate) }} มีนัดแล้ว {{ appointmentDayLoad.scheduledCount }} คน
              </span>
              <span v-else class="caption helper-text">เลือกวันนัดเพื่อดูจำนวนผู้ป่วยที่นัดไว้แล้ว</span>
            </label>
          </div>

          <div class="form-section">
            <p class="caption label">การรับประทานยา</p>
            <div class="radio-group">
              <label v-for="opt in [['good', 'ดี'], ['fair', 'พอใช้'], ['poor', 'ไม่ดี']]" :key="opt[0]" class="radio-label">
                <input type="radio" :value="opt[0]" v-model="adherence" />
                {{ opt[1] }}
              </label>
            </div>
          </div>

          <div class="form-section">
            <p class="caption label">อาการไม่พึงประสงค์จากระดับยาสูง (INR &gt; 3.0)</p>
            <div class="checkbox-grid">
              <label v-for="se in sideEffectOptionsHigh" :key="se.key" class="checkbox-label">
                <input type="checkbox" :value="se.key" v-model="selectedSideEffects" />
                {{ se.label }}
              </label>
            </div>
          </div>

          <div class="form-section">
            <p class="caption label">อาการไม่พึงประสงค์จากระดับยาต่ำ (&lt; 2.0)</p>
            <div class="checkbox-grid">
              <label v-for="se in sideEffectOptionsLow" :key="se.key" class="checkbox-label">
                <input type="checkbox" :value="se.key" v-model="selectedSideEffects" />
                {{ se.label }}
              </label>
            </div>
          </div>

          <label class="form-field">
            <span class="caption label">หมายเหตุ</span>
            <textarea class="input" rows="3" v-model="notes" />
          </label>
        </div>

        <div class="panel-footer">
          <button class="btn btn-ghost" @click="modelValue = false">ยกเลิก</button>
          <button class="btn btn-primary" @click="handleSubmit" :disabled="saving">
            {{ saving ? 'กำลังบันทึก...' : (isEditMode ? 'บันทึกการเปลี่ยนแปลง' : 'บันทึก & เปิดใบพิมพ์') }}
          </button>
        </div>
      </div>
    </div>

    <ConfirmDialog
      :open="showDoseConfirm"
      title="ยืนยันการเปลี่ยนขนาดยา"
      :message="`ขนาดยาจะเปลี่ยนจาก ${currentDoseWeek.toFixed(1)} mg/สัปดาห์ เป็น ${newDoseWeek.toFixed(1)} mg/สัปดาห์ (${doseChanged ? 'เปลี่ยน' : 'คงเดิม'}) กรุณาตรวจสอบและยืนยันการบันทึก`"
      confirm-label="ยืนยันการเปลี่ยนขนาดยา"
      @confirm="confirmDoseChange"
      @cancel="cancelDoseChange"
    />
  </Teleport>
</template>

<style scoped>
.panel-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.35); z-index: 50;
  display: flex; align-items: stretch; justify-content: flex-end;
}
.visit-panel {
  width: min(760px, 100vw); height: 100vh; display: flex; flex-direction: column;
  border-radius: var(--rounded-xl) 0 0 var(--rounded-xl); overflow: hidden;
}
.panel-header {
  display: flex; justify-content: space-between; align-items: center;
  padding: var(--spacing-xl); border-bottom: 1px solid var(--color-hairline-soft);
}
.panel-body { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: var(--spacing-lg); padding: var(--spacing-xl); }
.panel-footer { display: flex; justify-content: flex-end; gap: var(--spacing-md); padding: var(--spacing-xl); border-top: 1px solid var(--color-hairline-soft); }
.form-row { display: grid; grid-template-columns: 180px 120px; gap: var(--spacing-md); }
.form-row .input { height: 44px; }
.form-field { display: flex; flex-direction: column; gap: var(--spacing-xs); }
.dose-source-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-md);
  padding: var(--spacing-md);
  border: 1px solid var(--color-hairline-soft);
  border-radius: var(--rounded-xl);
  background: var(--color-pink-100);
}
.dose-source-text { color: var(--color-slate); }
.dose-warning { color: var(--color-coral-500); }
.btn-compact { padding-inline: var(--spacing-md); white-space: nowrap; }
.form-section { display: flex; flex-direction: column; gap: var(--spacing-sm); }
.label { color: var(--color-slate); }
.suggestion-row { display: flex; align-items: center; gap: var(--spacing-md); flex-wrap: wrap; }
.dose-options-section { display: flex; flex-direction: column; gap: var(--spacing-sm); margin-top: var(--spacing-sm); }
.radio-group { display: flex; gap: var(--spacing-lg); }
.radio-label { display: flex; align-items: center; gap: var(--spacing-xs); cursor: pointer; }
.checkbox-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: var(--spacing-xs); }
.checkbox-label { display: flex; align-items: center; gap: var(--spacing-xs); cursor: pointer; }
.helper-text { color: var(--color-slate); }
</style>
