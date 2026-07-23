<script setup lang="ts">
const _props = defineProps<{
  label: string;
  color?: 'coral' | 'pink' | 'success' | 'danger' | 'muted' | 'yellow';
}>();
</script>

<template>
  <span :class="['badge', `pill-${props.color ?? 'muted'}`]">{{ props.label }}</span>
</template>

<style scoped>
.pill-coral { background: var(--color-coral-100); color: var(--color-coral-500); }
.pill-pink { background: var(--color-pink-100); color: var(--color-pink-600); }
.pill-success { background: var(--color-inr-safe); color: var(--color-on-primary); }
.pill-danger { background: var(--color-inr-high-bg); color: var(--color-inr-high); }
.pill-muted { background: var(--color-surface); color: var(--color-stone); }
.pill-yellow { background: var(--color-yellow-100); color: var(--color-yellow-500); }
</style>
