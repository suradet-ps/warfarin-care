<script setup lang="ts">
import { ChevronDown, ChevronUp } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { useVisitStore } from '#/stores/visit.ts';
import type { WfVisit } from '#/types/visit.ts';
import {
  doseDayKeys,
  doseDayLabels,
  formatThaiDate,
  normalizeDoseSchedule,
} from '#/utils/clinic.ts';

const props = defineProps<{ visits: WfVisit[]; hn: string }>();

const emit = defineEmits<{ (e: 'deleted'): void; (e: 'edit', visit: WfVisit): void }>();

const visitStore = useVisitStore();

const expandedIds = ref<Set<number>>(new Set());
const deleteTargetId = ref<number | null>(null);

const sortedVisits = computed(() =>
  [...props.visits].sort((a, b) => `${b.visitDate}`.localeCompare(`${a.visitDate}`)),
);

function toggleExpand(id: number) {
  if (expandedIds.value.has(id)) {
    expandedIds.value.delete(id);
  } else {
    expandedIds.value.add(id);
  }
}

function confirmDelete(id: number) {
  deleteTargetId.value = id;
}

function handleEdit(visit: WfVisit) {
  emit('edit', visit);
}

async function handleConfirmedDelete() {
  if (deleteTargetId.value === null) {
    return;
  }
  await visitStore.deleteVisit(deleteTargetId.value, props.hn);
  deleteTargetId.value = null;
  emit('deleted');
}

function handleCancelDelete() {
  deleteTargetId.value = null;
}

const adherenceLabels: Record<string, string> = {
  good: 'ดี',
  fair: 'พอใช้',
  poor: 'ไม่ดี',
};

function adherenceBadgeClass(a?: string | null) {
  if (a === 'good') {
    return 'badge-success';
  }
  if (a === 'poor') {
    return 'badge-danger';
  }
  return 'badge-tag-coral';
}
</script>

<template>
  <div class="visit-list">
    <div v-if="visits.length === 0" class="body-sm" style="color: var(--color-stone)">ยังไม่มีบันทึกการทำคลินิก</div>
    <div v-for="v in sortedVisits" :key="v.id" class="visit-item card">
      <div class="visit-header" @click="toggleExpand(v.id)">
        <div class="visit-meta">
          <span class="body-sm-medium">{{ formatThaiDate(v.visitDate) }}</span>
          <span v-if="v.reviewedAt" class="badge badge-success">
            <Check :size="12" /> ยืนยันแล้ว
          </span>
          <span v-if="v.inrValue != null" class="badge badge-tag-coral">INR {{ v.inrValue.toFixed(1) }}</span>
          <span v-if="v.newDoseMgday != null" class="caption" style="color: var(--color-slate)">
            ยา {{ (v.newDoseMgday * 7).toFixed(1) }} mg/สัปดาห์
          </span>
          <span v-if="v.adherence" :class="['badge', adherenceBadgeClass(v.adherence)]">
            {{ adherenceLabels[v.adherence] ?? v.adherence }}
          </span>
        </div>
        <div class="visit-actions">
          <button class="btn-icon" title="แก้ไข" @click.stop="handleEdit(v)">
            <Pencil :size="14" />
          </button>
          <button class="btn-icon" title="ลบ" @click.stop="confirmDelete(v.id)">
            <Trash2 :size="14" />
          </button>
          <component :is="expandedIds.has(v.id) ? ChevronUp : ChevronDown" :size="16" style="color: var(--color-slate); flex-shrink: 0" />
        </div>
      </div>

      <div v-if="expandedIds.has(v.id)" class="visit-detail">
        <div v-if="v.nextAppointment" class="detail-row">
          <span class="caption" style="color: var(--color-slate)">นัดต่อไป:</span>
          <span class="body-sm">{{ formatThaiDate(v.nextAppointment) }}</span>
        </div>
        <div v-if="v.nextInrDue" class="detail-row">
          <span class="caption" style="color: var(--color-slate)">ตรวจ INR:</span>
          <span class="body-sm">{{ formatThaiDate(v.nextInrDue) }}</span>
        </div>
        <div v-if="v.newDoseDetail" class="detail-row">
          <span class="caption" style="color: var(--color-slate)">ตารางยาใหม่:</span>
          <div class="dose-mini-grid">
            <span v-for="k in doseDayKeys" :key="k" class="dose-cell">
              <span class="caption" style="color: var(--color-stone)">{{ doseDayLabels[k] }}</span>
              <span class="body-sm">{{ normalizeDoseSchedule(v.newDoseDetail)[k] }}</span>
            </span>
          </div>
        </div>
        <div v-if="v.notes" class="detail-row">
          <span class="caption" style="color: var(--color-slate)">หมายเหตุ:</span>
          <span class="body-sm">{{ v.notes }}</span>
        </div>
      </div>
    </div>
  </div>

  <ConfirmDialog
    :open="deleteTargetId !== null"
    title="ยืนยันการลบ"
    message="คุณต้องการลบประวัติการทำคลินิกนี้ใช่หรือไม่? การลบจะไม่สามารถกู้คืนได้"
    confirm-label="ลบ"
    @update:open="(v: boolean) => { if (!v) handleCancelDelete() }"
    @confirm="handleConfirmedDelete"
    @cancel="handleCancelDelete"
  />
</template>

<style scoped>
.visit-list { display: flex; flex-direction: column; gap: var(--spacing-md); }
.visit-item { padding: var(--spacing-md); display: flex; flex-direction: column; gap: var(--spacing-sm); }
.visit-header { display: flex; justify-content: space-between; align-items: center; cursor: pointer; gap: var(--spacing-sm); }
.visit-meta { display: flex; align-items: center; gap: var(--spacing-sm); flex-wrap: wrap; }
.visit-actions { display: flex; align-items: center; gap: var(--spacing-xs); }
.btn-icon {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--rounded-full);
  background: transparent;
  color: var(--color-slate);
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}
.btn-icon:hover {
  background: var(--color-inr-high-bg);
  color: var(--color-inr-high);
}
.visit-detail { display: flex; flex-direction: column; gap: var(--spacing-xs); padding-top: var(--spacing-sm); border-top: 1px solid var(--color-hairline-soft); }
.detail-row { display: flex; gap: var(--spacing-sm); align-items: flex-start; }
.dose-mini-grid { display: flex; gap: var(--spacing-sm); flex-wrap: wrap; }
.dose-cell { display: flex; flex-direction: column; align-items: center; gap: 2px; }
</style>
