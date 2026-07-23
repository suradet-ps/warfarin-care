<script setup lang="ts">
import { computed } from 'vue';
import PillBadge from '#/components/shared/PillBadge.vue';

const props = defineProps<{
  status: string;
  variant?: 'patient' | 'appointment';
}>();

const config = computed(() => {
  if (props.variant === 'appointment') {
    const appointmentMap: Record<
      string,
      { label: string; color: 'pink' | 'success' | 'danger' | 'muted' }
    > = {
      scheduled: { label: 'นัดหมาย', color: 'pink' },
      completed: { label: 'เสร็จสิ้น', color: 'success' },
      missed: { label: 'ขาดนัด', color: 'danger' },
      cancelled: { label: 'ยกเลิก', color: 'muted' },
    };
    return appointmentMap[props.status] ?? { label: props.status, color: 'muted' };
  }

  const patientMap: Record<
    string,
    { label: string; color: 'success' | 'muted' | 'danger' | 'coral' }
  > = {
    active: { label: 'กำลังติดตาม', color: 'success' },
    inactive: { label: 'หยุดติดตาม', color: 'muted' },
    deceased: { label: 'เสียชีวิต', color: 'danger' },
    transferred: { label: 'ส่งต่อ', color: 'coral' },
    discharged: { label: 'จำหน่าย', color: 'muted' },
  };
  return patientMap[props.status] ?? { label: props.status, color: 'muted' };
});
</script>

<template>
  <PillBadge :label="config.label" :color="config.color" />
</template>
