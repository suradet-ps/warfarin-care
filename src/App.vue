<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import AppSidebar from '#/components/layout/AppSidebar.vue'
import AppHeader from '#/components/layout/AppHeader.vue'
import { useAuthStore } from '#/stores/auth'
import { useSyncStore } from '#/stores/sync'

const route = useRoute()
const syncStore = useSyncStore()
const authStore = useAuthStore()

// The login + first-time setup views are full-page layouts. Render them
// without the sidebar/header chrome so the auth experience is uncluttered.
const isAuthScreen = computed(
  () => route.path === '/login' || route.path === '/setup',
)

const hideSplash = () => {
  const splash = document.getElementById('splash-overlay')
  const card = splash?.querySelector('.splash-card') as HTMLElement
  if (card) {
    card.classList.add('splash-card-fade-out')
  }
  if (splash) {
    splash.classList.add('splash-fade-out')
    setTimeout(() => splash.remove(), 400)
  }
}

onMounted(async () => {
  // Minimum splash display time (1.2s) for visual smoothness
  setTimeout(hideSplash, 1200)

  // The router guard also calls `bootstrap` lazily, but doing it here lets
  // the auth store settle before any app-shell fetches fire.
  try {
    await authStore.bootstrap()
  } catch (error) {
    console.error('Failed to bootstrap auth:', error)
  }

  if (authStore.currentUser) {
    try {
      await syncStore.refreshAll()
    } catch (error) {
      console.error('Failed to refresh sync status:', error)
    }
    syncStore.startAutoSync()
  }
})
</script>

<template>
  <div v-if="isAuthScreen" class="auth-router">
    <RouterView />
  </div>
  <div v-else class="app-shell">
    <AppSidebar />
    <div class="app-main">
      <AppHeader />
      <main class="app-content">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: var(--color-canvas);
}
.app-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.app-content {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-xl);
}
.auth-router {
  height: 100vh;
  width: 100%;
}
</style>