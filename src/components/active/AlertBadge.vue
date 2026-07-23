<script setup lang="ts">
import { computed } from 'vue';
import type { PatientAlert } from '#/types/alert.ts';

const props = defineProps<{ alerts: PatientAlert[] }>();

const _summary = computed(() => {
  const critical = props.alerts.filter((alert) => alert.severity === 'critical').length;
  const warning = props.alerts.filter((alert) => alert.severity === 'warning').length;
  return { critical, warning };
});
</script>

<template>
  <span v-if="alerts.length" :class="['badge', summary.critical ? 'badge-danger' : 'badge-warning']">
    {{ summary.critical || summary.warning }}
  </span>
  <span v-else class="badge badge-muted">-</span>
</template>
