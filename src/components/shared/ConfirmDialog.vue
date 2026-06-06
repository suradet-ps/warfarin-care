<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  open: boolean
  title: string
  message: string
  confirmLabel?: string
  cancelLabel?: string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: []
  cancel: []
}>()

const dialogRef = ref<HTMLDialogElement | null>(null)

watch(
  () => props.open,
  (open) => {
    const el = dialogRef.value
    if (!el) return
    if (open && !el.open) {
      el.showModal()
    } else if (!open && el.open) {
      el.close()
    }
  },
  { immediate: true },
)

function close() {
  emit('update:open', false)
  emit('cancel')
}
</script>

<template>
  <dialog
    ref="dialogRef"
    class="confirm-dialog"
    @cancel.prevent="close"
    @close="emit('update:open', false)"
  >
    <div class="dialog-box card">
      <div class="dialog-body">
        <h3 class="h4">{{ title }}</h3>
        <p class="body-sm dialog-message">{{ message }}</p>
      </div>
      <div class="dialog-actions">
        <button type="button" class="btn btn-secondary" @click="close">
          {{ cancelLabel ?? 'ยกเลิก' }}
        </button>
        <button
          type="button"
          class="btn btn-primary"
          @click="emit('confirm')"
        >
          {{ confirmLabel ?? 'ยืนยัน' }}
        </button>
      </div>
    </div>
  </dialog>
</template>

<style scoped>
.confirm-dialog {
  border: none;
  padding: 0;
  background: transparent;
  color: var(--color-ink);
  max-width: min(100%, 32rem);
  width: 100%;
}
.confirm-dialog::backdrop {
  background: color-mix(in srgb, var(--color-ink-deep) 24%, transparent);
}
.dialog-box {
  width: 100%;
  box-shadow: var(--elevation-4);
}
.dialog-body {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}
.dialog-message {
  color: var(--color-slate);
}
.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-xl);
}
</style>
