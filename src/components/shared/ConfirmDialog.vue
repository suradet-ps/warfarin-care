<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';

const props = defineProps<{
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: 'danger' | 'default';
}>();

const emit = defineEmits<{
  'update:open': [value: boolean];
  confirm: [];
  cancel: [];
}>();

const dialogRef = ref<HTMLDialogElement | null>(null);
const confirmBtnRef = ref<HTMLButtonElement | null>(null);

watch(
  () => props.open,
  async (open) => {
    const el = dialogRef.value;
    if (!el) {
      return;
    }
    if (open && !el.open) {
      el.showModal();
      await nextTick();
      confirmBtnRef.value?.focus();
    } else if (!open && el.open) {
      el.close();
    }
  },
  { immediate: true },
);

function close() {
  emit('update:open', false);
  emit('cancel');
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault();
    close();
  }
}
</script>

<template>
  <dialog
    ref="dialogRef"
    class="confirm-dialog"
    role="alertdialog"
    aria-modal="true"
    :aria-labelledby="`confirm-title-${title}`"
    :aria-describedby="`confirm-msg-${title}`"
    @cancel.prevent="close"
    @close="emit('update:open', false)"
    @keydown="handleKeydown"
  >
    <div class="dialog-box card">
      <div class="dialog-body">
        <h3 class="h4" :id="`confirm-title-${title}`">{{ title }}</h3>
        <p class="body-sm dialog-message" :id="`confirm-msg-${title}`">{{ message }}</p>
      </div>
      <div class="dialog-actions">
        <button type="button" class="btn btn-secondary" @click="close">
          {{ cancelLabel ?? 'ยกเลิก' }}
        </button>
        <button
          ref="confirmBtnRef"
          type="button"
          :class="['btn', variant === 'danger' ? 'btn-danger' : 'btn-primary']"
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
