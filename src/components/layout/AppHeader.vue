<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { Bell } from 'lucide-vue-next'
import { useAlertStore } from '#/stores/alerts'

const route = useRoute()
const alertStore = useAlertStore()

const pageTitle = computed(() => {
  const map: Record<string, string> = {
    '/screening': 'คัดกรองผู้ป่วย',
    '/active': 'ผู้ป่วยคลินิกวาร์ฟาริน',
    '/appointments': 'การนัดหมาย',
    '/review': 'ตรวจสอบการบันทึก',
    '/reports': 'รายงาน',
    '/settings': 'ตั้งค่าระบบ',
  }
  if (route.path.startsWith('/patient/')) return 'ข้อมูลผู้ป่วย'
  if (route.path.startsWith('/slip/')) return 'Warfarin Assessment & Recommendation'
  return map[route.path] ?? 'คลินิกวาร์ฟาริน'
})

const totalAlerts = computed(() => alertStore.criticalCount + alertStore.warningCount)

onMounted(() => {
  if (!alertStore.alerts.length) {
    void alertStore.fetchAlerts()
  }
})
</script>

<template>
  <header class="app-header">
    <div>
      <h1 class="header-title">{{ pageTitle }}</h1>
      <p class="caption header-subtitle">ติดตาม INR, ขนาดยา และนัดหมายอย่างต่อเนื่อง</p>
    </div>
    <div class="header-actions">
      <div v-if="totalAlerts > 0" class="alert-pill"><Bell :size="18" /><span>{{ totalAlerts }} แจ้งเตือน</span></div>
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
.header-actions { display: flex; align-items: center; }
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
</style>
