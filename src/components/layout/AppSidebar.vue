<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { BarChart3, CalendarDays, ClipboardCheck, Search, Settings, Users } from 'lucide-vue-next'
import { useAlertStore } from '#/stores/alerts'
import { useReviewStore } from '#/stores/review'
import { useSettingsStore } from '#/stores/settings'

const route = useRoute()
const alertStore = useAlertStore()
const reviewStore = useReviewStore()
const settingsStore = useSettingsStore()

const navItems = [
  { name: 'screening', label: 'คัดกรอง', icon: Search, path: '/screening' },
  { name: 'active', label: 'ผู้ป่วยทั้งหมด', icon: Users, path: '/active' },
  { name: 'appointments', label: 'การนัดหมาย', icon: CalendarDays, path: '/appointments' },
  { name: 'review', label: 'ตรวจสอบ', icon: ClipboardCheck, path: '/review' },
  { name: 'reports', label: 'รายงาน', icon: BarChart3, path: '/reports' },
  { name: 'settings', label: 'ตั้งค่า', icon: Settings, path: '/settings' },
]

const totalAlerts = computed(() => alertStore.criticalCount + alertStore.warningCount)
const pendingReviewCount = computed(() => reviewStore.pendingCount)

onMounted(() => {
  void Promise.all([alertStore.fetchAlerts(), settingsStore.loadSettings(), reviewStore.fetchPendingCount()])
})
</script>

<template>
  <nav class="sidebar">
    <div class="sidebar-logo">
      <svg class="sidebar-logo-icon" width="40" height="40" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" focusable="false">
        <defs>
          <filter id="sidebar_shadow" x="-20%" y="-20%" width="140%" height="140%">
            <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#831843" flood-opacity="0.3"/>
          </filter>
        </defs>
        <g filter="url(#sidebar_shadow)" stroke="#EC4899" stroke-width="12" stroke-linecap="round" stroke-linejoin="round">
          <path d="M100 185C100 185 185 130 185 80C185 45 155 25 125 25C108 25 100 38 100 38C100 38 92 25 75 25C45 25 15 45 15 80C15 130 100 185 100 185Z"/>
          <path d="M100 160C100 160 160 115 160 80C160 55 142 42 125 42C112 42 100 50 100 50C100 50 88 42 75 42C58 42 40 55 40 80C40 115 100 160 100 160Z" opacity="0.6"/>
          <path d="M100 135C100 135 135 105 135 80C135 68 128 60 120 60C110 60 100 68 100 68C100 68 90 60 80 60C72 60 65 68 65 80C65 105 100 135 100 135Z" opacity="0.3"/>
        </g>
      </svg>
      <div class="sidebar-logo-text">
        <span class="sidebar-logo-title">วาร์ฟาริน</span>
        <span class="sidebar-logo-sub">คลินิก</span>
      </div>
    </div>
    <ul class="sidebar-nav">
      <li v-for="item in navItems" :key="item.name">
        <RouterLink :to="item.path" class="sidebar-nav-item" :class="{ active: route.path.startsWith(item.path) }">
          <component :is="item.icon" :size="20" />
          <span>{{ item.label }}</span>
          <span v-if="item.name === 'active' && totalAlerts > 0" class="sidebar-badge">{{ totalAlerts }}</span>
          <span v-if="item.name === 'review' && pendingReviewCount > 0" class="sidebar-badge review-badge">{{ pendingReviewCount }}</span>
        </RouterLink>
      </li>
    </ul>
    <div class="sidebar-footer">
      <span class="micro footer-text">{{ settingsStore.hospitalName }}</span>
    </div>
  </nav>
</template>

<style scoped>
.sidebar {
  width: 15rem;
  min-width: 15rem;
  background: var(--color-pink-50);
  display: flex;
  flex-direction: column;
  padding: var(--spacing-xl) 0;
  border-right: 1px solid rgba(236, 72, 153, 0.15);
}

.sidebar-logo {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: 0 var(--spacing-xl) var(--spacing-xxl);
}
.sidebar-logo-icon {
  width: 40px;
  height: 40px;
  flex-shrink: 0;
}
.sidebar-logo-text {
  display: flex;
  flex-direction: column;
}
.sidebar-logo-title {
  font-size: var(--typography-heading-5-size);
  font-weight: var(--typography-heading-5-weight);
  color: var(--color-ink);
}
.sidebar-logo-sub,
.footer-text {
  color: var(--color-slate);
}

.sidebar-nav {
  list-style: none;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xxs);
  padding: 0 var(--spacing-sm);
}

.sidebar-nav-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--rounded-full);
  color: var(--color-slate);
  text-decoration: none;
  transition: background 150ms ease, color 150ms ease;
}
.sidebar-nav-item:hover {
  background: var(--color-pink-100);
  color: var(--color-ink);
}
.sidebar-nav-item.active {
  background: var(--color-pink-600);
  color: var(--color-on-dark);
}
.sidebar-nav-item.active:hover {
  background: var(--color-pink-600);
}

.sidebar-badge {
  margin-left: auto;
  padding: 0 var(--spacing-xs);
  border-radius: var(--rounded-full);
  background: var(--color-inr-high);
  color: var(--color-on-dark);
  font-size: var(--typography-micro-size);
  font-weight: var(--typography-micro-weight);
}
.review-badge {
  background: var(--color-coral-500);
}

.sidebar-footer {
  padding: var(--spacing-xl);
  border-top: 1px solid rgba(236, 72, 153, 0.15);
  margin-top: var(--spacing-xl);
}
</style>