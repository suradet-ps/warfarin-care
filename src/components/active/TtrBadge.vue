<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{ ttr?: number | null }>();

const badgeClass = computed(() => {
  if (props.ttr === null || props.ttr === undefined) {
    return 'badge-muted';
  }
  if (props.ttr >= 65) {
    return 'good';
  }
  if (props.ttr >= 50) {
    return 'fair';
  }
  return 'poor';
});
</script>

<template>
  <span :class="['badge', badgeClass]">TTR {{ ttr !== null && ttr !== undefined ? `${ttr.toFixed(0)}%` : '-' }}</span>
</template>

<style scoped>
.good { background: var(--color-ttr-good); color: #fff; }
.fair { background: var(--color-ttr-warn); color: var(--color-ink-deep); }
.poor { background: var(--color-ttr-bad); color: #fff; }
</style>
