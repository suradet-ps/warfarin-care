<script setup lang="ts">
import { ref } from 'vue'
import { usePatientStore } from '#/stores/patient'
import type { EnrollmentInput } from '#/types/patient'
import { X } from 'lucide-vue-next'
import { DEFAULT_TARGET_INR, DEFAULT_TARGET_INR_BY_INDICATION } from '#/utils/clinic'

const props = defineProps<{ hn: string }>()
const emit = defineEmits<{ close: []; enrolled: [] }>()

const store = usePatientStore()
const saving = ref(false)
const error = ref<string | null>(null)

const form = ref<EnrollmentInput>({
  hn: props.hn,
  indication: 'AF',
  targetInrLow: DEFAULT_TARGET_INR.low,
  targetInrHigh: DEFAULT_TARGET_INR.high,
  enrolledAt: new Date().toISOString().split('T')[0],
  enrolledBy: '',
  notes: '',
})

function onIndicationChange() {
  const d = DEFAULT_TARGET_INR_BY_INDICATION[form.value.indication]
  if (d) {
    form.value.targetInrLow = d.low
    form.value.targetInrHigh = d.high
  }
}

async function handleSubmit() {
  saving.value = true
  error.value = null
  try {
    await store.enrollPatient(form.value)
    emit('enrolled')
  } catch (e) {
    error.value = String(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal-box card">
      <div class="modal-header">
        <h3 class="h4">นำเข้าคลินิก — HN {{ hn }}</h3>
        <button class="btn btn-ghost" style="padding: 4px;" @click="emit('close')"><X :size="18" /></button>
      </div>

      <form @submit.prevent="handleSubmit" class="modal-form">
        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">ข้อบ่งชี้</span>
          <select class="input" v-model="form.indication" @change="onIndicationChange">
            <option value="AF">AF (Atrial Fibrillation)</option>
            <option value="DVT">DVT (Deep Vein Thrombosis)</option>
            <option value="PE">PE (Pulmonary Embolism)</option>
            <option value="mechanical_valve">Mechanical Valve</option>
            <option value="other">อื่นๆ</option>
          </select>
        </label>

        <div class="form-row">
          <label class="form-field">
            <span class="caption" style="color:var(--color-slate)">เป้าหมาย INR (ต่ำสุด)</span>
            <input class="input" type="number" step="0.1" v-model.number="form.targetInrLow" />
          </label>
          <label class="form-field">
            <span class="caption" style="color:var(--color-slate)">เป้าหมาย INR (สูงสุด)</span>
            <input class="input" type="number" step="0.1" v-model.number="form.targetInrHigh" />
          </label>
        </div>

        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">วันที่ลงทะเบียน</span>
          <input class="input" type="date" v-model="form.enrolledAt" />
        </label>

        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">ลงทะเบียนโดย</span>
          <input class="input" v-model="form.enrolledBy" placeholder="ชื่อเภสัชกร / พยาบาล" />
        </label>

        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">หมายเหตุ</span>
          <textarea class="input" rows="2" v-model="form.notes" style="height: auto; resize: vertical;"></textarea>
        </label>

        <div v-if="error" class="error-msg badge badge-danger">{{ error }}</div>

        <div class="modal-actions">
          <button type="button" class="btn btn-secondary" @click="emit('close')">ยกเลิก</button>
          <button type="submit" class="btn btn-primary" :disabled="saving">{{ saving ? 'กำลังบันทึก...' : 'ลงทะเบียน' }}</button>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay { position: fixed; inset: 0; background: rgba(5, 0, 56, 0.4); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal-box { width: 480px; box-shadow: var(--elevation-4); max-height: 90vh; overflow-y: auto; }
.modal-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--spacing-xl); }
.modal-form { display: flex; flex-direction: column; gap: var(--spacing-md); }
.form-field { display: flex; flex-direction: column; gap: var(--spacing-xs); }
.form-row { display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-md); }
.modal-actions { display: flex; justify-content: flex-end; gap: var(--spacing-sm); margin-top: var(--spacing-md); }
.error-msg { justify-content: flex-start; }
</style>
