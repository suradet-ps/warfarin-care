<script setup lang="ts">
defineProps<{
  message: string;
  actionLabel?: string;
}>();

const emit = defineEmits<{
  action: [];
}>();
</script>

<template>
  <div class="empty-state" role="status">
    <p class="body-sm">{{ message }}</p>
    <button
      v-if="actionLabel"
      type="button"
      class="btn btn-primary btn-sm"
      @click="emit('action')"
    >
      {{ actionLabel }}
    </button>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-md);
  padding: var(--spacing-xxl);
  color: var(--color-stone);
  text-align: center;
}
.btn-sm { padding: var(--spacing-xs) var(--spacing-md); font-size: var(--typography-body-sm-size); }
</style>
