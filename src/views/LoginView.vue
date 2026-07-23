<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useAuthStore } from '#/stores/auth.ts';

const router = useRouter();
const route = useRoute();
const store = useAuthStore();

const username = ref('');
const password = ref('');
const submitting = ref(false);
const localError = ref<string | null>(null);

onMounted(() => {
  // If the user lands here already authenticated (e.g. after a hot reload),
  // bounce them back to the original target.
  if (store.currentUser) {
    void router.replace((route.query.redirect as string) || '/');
  }
});

async function handleSubmit() {
  localError.value = null;
  store.reset();
  if (!(username.value.trim() && password.value)) {
    localError.value = 'กรุณากรอกชื่อผู้ใช้และรหัสผ่าน';
    return;
  }
  submitting.value = true;
  try {
    await store.login({
      username: username.value.trim(),
      password: password.value,
    });
    const target = (route.query.redirect as string) || '/';
    await router.replace(target);
  } catch (e) {
    // Backend already returns a localized, user-safe Thai message. Show as-is.
    localError.value = store.error ?? (e instanceof Error ? e.message : String(e));
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="auth-shell">
    <form class="auth-card" @submit.prevent="handleSubmit">
      <div class="auth-logo">
        <svg width="56" height="56" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" focusable="false">
          <g stroke="#EC4899" stroke-width="10" stroke-linecap="round" stroke-linejoin="round">
            <path d="M100 185C100 185 185 130 185 80C185 45 155 25 125 25C108 25 100 38 100 38C100 38 92 25 75 25C45 25 15 45 15 80C15 130 100 185 100 185Z" />
            <path d="M100 160C100 160 160 115 160 80C160 55 142 42 125 42C112 42 100 50 100 50C100 50 88 42 75 42C58 42 40 55 40 80C40 115 100 160 100 160Z" opacity="0.6" />
            <path d="M100 135C100 135 135 105 135 80C135 68 128 60 120 60C110 60 100 68 100 68C100 68 90 60 80 60C72 60 65 68 65 80C65 105 100 135 100 135Z" opacity="0.3" />
          </g>
        </svg>
        <h1 class="auth-title">Warfarin Care</h1>
        <p class="auth-subtitle">เข้าสู่ระบบเพื่อใช้งาน</p>
      </div>

      <label class="auth-field">
        <span class="auth-label">ชื่อผู้ใช้</span>
        <input
          v-model="username"
          type="text"
          class="auth-input"
          autocomplete="username"
          autocapitalize="off"
          autocorrect="off"
          spellcheck="false"
          required
          :disabled="submitting"
        />
      </label>

      <label class="auth-field">
        <span class="auth-label">รหัสผ่าน</span>
        <input
          v-model="password"
          type="password"
          class="auth-input"
          autocomplete="current-password"
          required
          :disabled="submitting"
        />
      </label>

      <div v-if="localError" class="auth-error" role="alert">
        {{ localError }}
      </div>

      <button type="submit" class="auth-submit" :disabled="submitting">
        <LogIn :size="18" />
        <span>{{ submitting ? 'กำลังเข้าสู่ระบบ...' : 'เข้าสู่ระบบ' }}</span>
      </button>
    </form>
  </div>
</template>

<style scoped>
.auth-shell {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  width: 100%;
  background: var(--color-pink-50);
  padding: var(--spacing-xl);
}
.auth-card {
  width: 100%;
  max-width: 22rem;
  background: var(--color-canvas);
  border: 1px solid var(--color-hairline);
  border-radius: var(--rounded-xl);
  box-shadow:
    rgba(131, 24, 67, 0.08) 0 8px 24px,
    rgba(131, 24, 67, 0.04) 0 2px 6px;
  padding: var(--spacing-xxl) var(--spacing-xxl) var(--spacing-xxl);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}
.auth-logo {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-xs);
  margin-bottom: var(--spacing-sm);
}
.auth-title {
  font-size: var(--typography-heading-4-size);
  font-weight: var(--typography-heading-4-weight);
  color: var(--color-ink);
  letter-spacing: -0.03em;
  margin: 0;
}
.auth-subtitle {
  font-size: var(--typography-body-sm-medium-size);
  color: var(--color-slate);
  margin: 0;
}
.auth-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.auth-label {
  font-size: var(--typography-caption-bold-size);
  font-weight: var(--typography-caption-bold-weight);
  color: var(--color-charcoal);
}
.auth-input {
  width: 100%;
  padding: var(--spacing-sm) var(--spacing-md);
  border: 1px solid var(--color-hairline-strong);
  border-radius: var(--rounded-md);
  font-size: var(--typography-body-md-size);
  color: var(--color-ink);
  background: var(--color-canvas);
  font-family: inherit;
  transition: border-color 150ms ease, box-shadow 150ms ease;
  box-sizing: border-box;
}
.auth-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px rgba(190, 24, 93, 0.15);
}
.auth-input:disabled {
  background: var(--color-surface-soft);
  color: var(--color-stone);
}
.auth-error {
  background: var(--color-inr-high-bg);
  color: var(--color-inr-critical);
  border-radius: var(--rounded-md);
  padding: var(--spacing-sm) var(--spacing-md);
  font-size: var(--typography-body-sm-medium-size);
  font-weight: var(--typography-body-sm-medium-weight);
}
.auth-submit {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-xs);
  margin-top: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-xl);
  border: none;
  border-radius: var(--rounded-full);
  background: var(--color-primary);
  color: var(--color-on-primary);
  font-size: var(--typography-button-md-size);
  font-weight: var(--typography-button-md-weight);
  font-family: inherit;
  cursor: pointer;
  transition: background 150ms ease, transform 80ms ease;
}
.auth-submit:hover:not(:disabled) {
  background: var(--color-primary-hover);
}
.auth-submit:active:not(:disabled) {
  transform: scale(0.98);
}
.auth-submit:disabled {
  background: var(--color-stone);
  cursor: not-allowed;
}
</style>
