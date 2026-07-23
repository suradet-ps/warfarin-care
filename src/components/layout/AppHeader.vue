<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useAlertStore } from '#/stores/alerts.ts';
import { useAuthStore } from '#/stores/auth.ts';

const route = useRoute();
const router = useRouter();
const alertStore = useAlertStore();
const authStore = useAuthStore();

const pageTitle = computed(() => {
  const map: Record<string, string> = {
    '/screening': 'คัดกรองผู้ป่วย',
    '/active': 'ผู้ป่วยคลินิกวาร์ฟาริน',
    '/appointments': 'การนัดหมาย',
    '/review': 'ตรวจสอบการบันทึก',
    '/reports': 'รายงาน',
    '/settings': 'ตั้งค่าระบบ',
  };
  if (route.path.startsWith('/patient/')) {
    return 'ข้อมูลผู้ป่วย';
  }
  if (route.path.startsWith('/slip/')) {
    return 'Warfarin Assessment & Recommendation';
  }
  return map[route.path] ?? 'คลินิกวาร์ฟาริน';
});

const totalAlerts = computed(() => alertStore.criticalCount + alertStore.warningCount);

async function handleLogout() {
  await authStore.logout();
  await router.replace('/login');
}

onMounted(() => {
  if (alertStore.alerts.length === 0) {
    void alertStore.fetchAlerts();
  }
});
</script>

<template>
  <header class="app-header" role="banner">
    <div>
      <h1 class="header-title">{{ pageTitle }}</h1>
      <p class="caption header-subtitle">ติดตาม INR, ขนาดยา และนัดหมายอย่างต่อเนื่อง</p>
    </div>
    <div class="header-actions">
      <div v-if="authStore.currentUser" class="user-pill" :title="`ผู้ใช้: ${authStore.currentUser.username}`" :aria-label="`ผู้ใช้ปัจจุบัน ${authStore.currentUser.username} ตำแหน่ง ${authStore.currentUser.role}`">
        <span class="user-pill-name">{{ authStore.currentUser.username }}</span>
        <span class="user-pill-role">{{ authStore.currentUser.role }}</span>
      </div>
      <div v-if="totalAlerts > 0" class="alert-pill" role="status" :aria-label="`${totalAlerts} แจ้งเตือน`"><Bell :size="18" aria-hidden="true" /><span>{{ totalAlerts }} แจ้งเตือน</span></div>
      <button v-if="authStore.currentUser" type="button" class="logout-btn" title="ออกจากระบบ" aria-label="ออกจากระบบ" @click="handleLogout">
        <LogOut :size="18" aria-hidden="true" />
        <span>ออกจากระบบ</span>
      </button>
    </div>
  </header>
</template>

<style scoped>
.app-header {
  min-height: 4rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--spacing-xl);
  border-bottom: 1px solid var(--color-hairline);
  background: var(--color-canvas);
}
.header-title {
  font-size: var(--typography-heading-4-size);
  font-weight: var(--typography-heading-4-weight);
  color: var(--color-ink);
}
.header-subtitle { color: var(--color-slate); }
.header-actions { display: flex; align-items: center; gap: var(--spacing-sm); }
.user-pill {
  display: inline-flex;
  flex-direction: column;
  align-items: flex-end;
  line-height: 1.1;
  padding: var(--spacing-xs) var(--spacing-md);
  border-radius: var(--rounded-full);
  background: var(--color-pink-50);
  color: var(--color-charcoal);
}
.user-pill-name {
  font-size: var(--typography-body-sm-medium-size);
  font-weight: var(--typography-body-sm-medium-weight);
  color: var(--color-ink);
}
.user-pill-role {
  font-size: var(--typography-micro-size);
  color: var(--color-slate);
}
.alert-pill {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-xs) var(--spacing-md);
  border-radius: var(--rounded-full);
  background: var(--color-inr-high-bg);
  color: var(--color-inr-high);
  font-size: var(--typography-body-sm-medium-size);
  font-weight: var(--typography-body-sm-medium-weight);
}
.logout-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-xs) var(--spacing-md);
  border: 1px solid var(--color-hairline-strong);
  border-radius: var(--rounded-full);
  background: var(--color-canvas);
  color: var(--color-charcoal);
  font-size: var(--typography-body-sm-medium-size);
  font-weight: var(--typography-body-sm-medium-weight);
  font-family: inherit;
  cursor: pointer;
  transition: background 150ms ease, color 150ms ease, border-color 150ms ease;
}
.logout-btn:hover {
  background: var(--color-pink-50);
  color: var(--color-primary);
  border-color: var(--color-primary);
}
</style>
