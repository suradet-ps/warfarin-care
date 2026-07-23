<script setup lang="ts">
defineProps<{
  message?: string;
}>();
</script>

<template>
  <div class="loading-state card" role="status" aria-live="polite">
    <div class="loading-spinner" aria-hidden="true"></div>
    <p class="body-sm">{{ message ?? 'กำลังโหลด...' }}</p>
  </div>
</template>

<style scoped>
.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-md);
  padding: var(--spacing-xxl);
  color: var(--color-stone);
}
.loading-spinner {
  width: 24px;
  height: 24px;
  border: 3px solid var(--color-hairline);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
