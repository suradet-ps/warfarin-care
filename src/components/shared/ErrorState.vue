<script setup lang="ts">
defineProps<{
  message: string
}>()

const emit = defineEmits<{
  retry: []
}>()
</script>

<template>
  <div class="error-state card card-feature-coral" role="alert">
    <p class="body-sm-medium error-text">{{ message }}</p>
    <button type="button" class="btn btn-secondary btn-sm" @click="emit('retry')">
      ลองใหม่
    </button>
  </div>
</template>

<style scoped>
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-md);
  padding: var(--spacing-xl);
  text-align: center;
}
.error-text { color: var(--color-inr-high); }
.btn-sm { padding: var(--spacing-xs) var(--spacing-md); font-size: var(--typography-body-sm-size); }
</style>
