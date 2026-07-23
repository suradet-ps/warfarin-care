<script setup lang="ts">
import { computed, ref } from 'vue';
import { usePatientStore } from '#/stores/patient.ts';
import { dateInputToday } from '#/utils/clinic.ts';

const props = defineProps<{ hn: string; currentStatus: string }>();
const emit = defineEmits<{ (e: 'update:modelValue', v: boolean): void; (e: 'saved'): void }>();

const store = usePatientStore();
const saving = ref(false);
const error = ref<string | null>(null);
const newStatus = ref(props.currentStatus);
const reason = ref('');
const effectiveDate = ref(dateInputToday());
const confirmOpen = ref(false);

const statusOptions: { value: string; label: string }[] = [
  { value: 'active', label: 'อยู่ในการดูแล' },
  { value: 'inactive', label: 'ไม่ใช้งาน' },
  { value: 'deceased', label: 'เสียชีวิต' },
  { value: 'transferred', label: 'โอนย้าย' },
  { value: 'discharged', label: 'จำหน่าย' },
];

const selectedStatusLabel = computed(
  () => statusOptions.find((option) => option.value === newStatus.value)?.label ?? newStatus.value,
);
const _confirmMessage = computed(
  () =>
    `เปลี่ยนสถานะผู้ป่วย HN ${props.hn} เป็น "${selectedStatusLabel.value}" ตั้งแต่วันที่ ${effectiveDate.value}`,
);

function _handleSave() {
  error.value = null;
  confirmOpen.value = true;
}

async function _confirmSave() {
  saving.value = true;
  confirmOpen.value = false;
  error.value = null;
  try {
    await store.updateStatus(props.hn, newStatus.value, reason.value, effectiveDate.value);
    emit('saved');
    emit('update:modelValue', false);
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('update:modelValue', false)">
    <div class="modal-panel card">
      <h3 class="heading-md">&#x0E40;&#x0E1B;&#x0E25;&#x0E35;&#x0E48;&#x0E22;&#x0E19;&#x0E2A;&#x0E16;&#x0E32;&#x0E19;&#x0E30;&#x0E1C;&#x0E39;&#x0E49;&#x0E1B;&#x0E48;&#x0E27;&#x0E22;</h3>
      <p class="body-sm" style="color: var(--color-slate)">HN: {{ hn }}</p>

      <div v-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">{{ error }}</div>

      <label class="form-field">
        <span class="caption" style="color: var(--color-slate)">&#x0E2A;&#x0E16;&#x0E32;&#x0E19;&#x0E30;&#x0E43;&#x0E2B;&#x0E21;&#x0E48;</span>
        <select class="input" v-model="newStatus">
          <option v-for="opt in statusOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
        </select>
      </label>

      <label class="form-field">
        <span class="caption" style="color: var(--color-slate)">&#x0E27;&#x0E31;&#x0E19;&#x0E17;&#x0E35;&#x0E48;&#x0E21;&#x0E35;&#x0E1C;&#x0E25;</span>
        <input class="input" type="date" v-model="effectiveDate" />
      </label>

      <label class="form-field">
        <span class="caption" style="color: var(--color-slate)">&#x0E40;&#x0E2B;&#x0E15;&#x0E38;&#x0E1C;&#x0E25;</span>
        <textarea class="input" rows="3" v-model="reason" placeholder="&#x0E40;&#x0E2B;&#x0E15;&#x0E38;&#x0E1C;&#x0E25;&#x0E17;&#x0E35;&#x0E48;&#x0E40;&#x0E1B;&#x0E25;&#x0E35;&#x0E48;&#x0E22;&#x0E19;&#x0E2A;&#x0E16;&#x0E32;&#x0E19;&#x0E30;..." />
      </label>

      <div class="modal-actions">
        <button class="btn btn-ghost" @click="emit('update:modelValue', false)">&#x0E22;&#x0E01;&#x0E40;&#x0E25;&#x0E34;&#x0E01;</button>
        <button class="btn btn-primary" @click="handleSave" :disabled="saving">
          {{ saving ? '&#x0E01;&#x0E33;&#x0E25;&#x0E31;&#x0E07;&#x0E40;&#x0E01;&#x0E47;&#x0E1A;...' : '&#x0E1A;&#x0E31;&#x0E19;&#x0E17;&#x0E36;&#x0E01;' }}
        </button>
      </div>
    </div>

    <ConfirmDialog
      :open="confirmOpen"
      title="ยืนยันการเปลี่ยนสถานะ"
      :message="confirmMessage"
      confirm-label="ยืนยันการบันทึก"
      cancel-label="กลับไปแก้ไข"
      @update:open="(v: boolean) => (confirmOpen = v)"
      @cancel="confirmOpen = false"
      @confirm="confirmSave"
    />
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed; inset: 0; background: rgba(0, 0, 0, 0.4);
  display: flex; align-items: center; justify-content: center; z-index: 100;
}
.modal-panel {
  width: min(480px, 90vw);
  display: flex; flex-direction: column; gap: var(--spacing-lg); padding: var(--spacing-xxl);
}
.form-field { display: flex; flex-direction: column; gap: var(--spacing-xs); }
.modal-actions { display: flex; gap: var(--spacing-md); justify-content: flex-end; margin-top: var(--spacing-md); }
</style>
