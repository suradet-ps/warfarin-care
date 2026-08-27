<script setup lang="ts">
import { Search } from 'lucide-vue-next';
import { ref } from 'vue';

withDefaults(
  defineProps<{
    modelValue: string | undefined;
    placeholder?: string;
    ariaLabel?: string;
  }>(),
  {
    placeholder: 'ค้นหา...',
    ariaLabel: 'ค้นหา',
  },
);

const emit = defineEmits<{
  'update:modelValue': [value: string];
  enter: [];
}>();

const inputRef = ref<HTMLInputElement | null>(null);

function onInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLInputElement).value);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault();
    emit('enter');
  }
}

defineExpose({
  focus: () => inputRef.value?.focus(),
});
</script>

<template>
  <div class="search-box">
    <Search :size="16" class="search-icon" aria-hidden="true" />
    <input
      ref="inputRef"
      :value="modelValue"
      :placeholder="placeholder"
      :aria-label="ariaLabel"
      type="text"
      class="search-input"
      @input="onInput"
      @keydown="onKeydown"
    />
  </div>
</template>

<style scoped>
.search-box {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  min-width: var(--search-box-min-width, 16rem);
  max-width: 100%;
  background: var(--color-canvas);
  border: 1px solid var(--color-hairline-soft);
  border-radius: var(--rounded-md);
  padding: var(--spacing-sm) var(--spacing-md);
  transition: border-color 150ms ease, box-shadow 150ms ease;
}
.search-box:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px rgba(190, 24, 93, 0.15);
}
.search-icon {
  color: var(--color-stone);
  flex-shrink: 0;
}
.search-input {
  width: 100%;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-size: var(--typography-body-sm-size);
  color: var(--color-ink);
}
.search-input::placeholder {
  color: var(--color-stone);
}
</style>