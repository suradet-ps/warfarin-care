<script setup lang="ts">
defineProps<{
  mg: number;
  isHalf?: boolean;
}>();
</script>

<template>
  <span
    class="pill" :class="[
      `pill-${mg}`,
      isHalf ? 'pill-half' : '',
    ]"
  >
    <span v-if="!isHalf" class="pill-text">{{ mg }}</span>
  </span>
</template>

<style scoped>
.pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 9999px;
  vertical-align: middle;
  flex-shrink: 0;
}

.pill-half {
  clip-path: polygon(0 0, 50% 0, 50% 100%, 0% 100%);
}

.pill-text {
  font-size: var(--typography-micro-uppercase-size);
  font-weight: var(--typography-caption-bold-weight);
  color: var(--color-on-primary);
}

.pill-1 {
  background: var(--color-surface);
  border: 1px solid var(--color-hairline);
}

.pill-1 .pill-text {
  color: var(--color-stone);
}

.pill-2 {
  background: #fed7aa;
}

.pill-2 .pill-text {
  color: #9a3412;
}

.pill-3 {
  background: #bae6fd;
}

.pill-3 .pill-text {
  color: #0369a1;
}

.pill-5 {
  background: #fbcfe8;
}

.pill-5 .pill-text {
  color: #be185d;
}
</style>