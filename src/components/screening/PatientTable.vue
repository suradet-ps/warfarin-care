<script setup lang="ts">
import { computed } from 'vue';
import type { PatientDrugRecord } from '#/types/patient.ts';
import { calculateAge, formatThaiDate } from '#/utils/clinic.ts';

const props = defineProps<{
  records: PatientDrugRecord[];
  total: number;
  page: number;
  pageSize: number;
}>();
const emit = defineEmits<{
  enroll: [hn: string];
  pageChange: [page: number];
}>();

const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.pageSize)));
const rangeStart = computed(() => (props.total === 0 ? 0 : (props.page - 1) * props.pageSize + 1));
const rangeEnd = computed(() => Math.min(props.total, props.page * props.pageSize));

function ageLabel(birthday: string): string {
  const age = calculateAge(birthday);
  return age === null ? '-' : `${age} ปี`;
}

function getStrengthClass(strength: string): string {
  if (strength.includes('2')) {
    return 'pill-strength-2mg';
  }
  if (strength.includes('3')) {
    return 'pill-strength-3mg';
  }
  if (strength.includes('5')) {
    return 'pill-strength-5mg';
  }
  return 'pill-muted';
}

function getStrengthLabel(strength: string): string {
  if (strength.includes('2')) {
    return '2 mg';
  }
  if (strength.includes('3')) {
    return '3 mg';
  }
  if (strength.includes('5')) {
    return '5 mg';
  }
  return strength;
}

function goToPage(page: number) {
  if (page < 1 || page > totalPages.value || page === props.page) {
    return;
  }
  emit('pageChange', page);
}
</script>

<template>
  <div class="table-wrap">
    <div class="table-meta">
      <p class="caption" style="color: var(--color-slate)">พบ {{ props.total }} รายการ</p>
      <p v-if="props.total > 0" class="caption" style="color: var(--color-stone)">แสดง {{ rangeStart }}-{{ rangeEnd }} จากทั้งหมด {{ props.total }}</p>
    </div>

    <table class="table">
      <thead>
        <tr>
          <th>HN</th>
          <th>ชื่อ-สกุล</th>
          <th>อายุ</th>
          <th>วันล่าสุดที่รับยา</th>
          <th>ครั้งที่รับยา</th>
          <th>ความแรง</th>
          <th>สถานะ</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="props.records.length === 0">
          <td colspan="8" style="text-align:center; color: var(--color-stone); padding: var(--spacing-xxl)">ไม่พบข้อมูลใน opitemrece ตามเงื่อนไขที่ค้นหา</td>
        </tr>
        <tr v-for="r in props.records" :key="r.hn">
          <td><span style="font-weight:500; font-family: monospace">{{ r.hn }}</span></td>
          <td>{{ r.pname }}{{ r.fname }} {{ r.lname }}</td>
          <td>{{ ageLabel(r.birthday) }}</td>
          <td>{{ formatThaiDate(r.lastDispenseDate) }}</td>
          <td>{{ r.totalDispenseVisits }}</td>
          <td><span v-for="s in r.strengthsReceived" :key="s" :class="['badge', getStrengthClass(s)]" style="margin-right: 4px">{{ getStrengthLabel(s) }}</span></td>
          <td>
            <span v-if="r.isEnrolled" class="badge badge-success">ลงทะเบียนแล้ว</span>
            <span v-else class="badge badge-muted">ยังไม่ลงทะเบียน</span>
          </td>
          <td>
            <button v-if="!r.isEnrolled" class="btn btn-primary" style="padding: 6px 16px; font-size: var(--typography-micro-size)" @click="emit('enroll', r.hn)">นำเข้าคลินิก</button>
          </td>
        </tr>
      </tbody>
    </table>

    <div v-if="totalPages > 1" class="pagination">
      <button type="button" class="btn btn-secondary pagination-button" :disabled="props.page <= 1" @click="goToPage(props.page - 1)">ก่อนหน้า</button>
      <span class="caption pagination-label">หน้า {{ props.page }} / {{ totalPages }}</span>
      <button type="button" class="btn btn-secondary pagination-button" :disabled="props.page >= totalPages" @click="goToPage(props.page + 1)">ถัดไป</button>
    </div>
  </div>
</template>

<style scoped>
.table-wrap {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  overflow-x: auto;
}

.table-meta,
.pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-md);
  flex-wrap: wrap;
}

.pagination {
  justify-content: flex-end;
}

.pagination-button {
  min-width: 110px;
}

.pagination-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.pagination-label {
  color: var(--color-slate);
}
</style>
