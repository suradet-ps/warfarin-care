<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import AppHeader from '#/components/layout/AppHeader.vue';
import AppSidebar from '#/components/layout/AppSidebar.vue';
import { useAuthStore } from '#/stores/auth.ts';
import { useSyncStore } from '#/stores/sync.ts';

const route = useRoute();
const router = useRouter();
const syncStore = useSyncStore();
const authStore = useAuthStore();

const isAuthScreen = computed(() => route.path === '/login' || route.path === '/setup');

function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
    e.preventDefault();
    if (route.path.startsWith('/patient/')) {
      const hn = route.params.hn as string;
      const event = new CustomEvent('open-visit-panel', { detail: { hn } });
      window.dispatchEvent(event);
    }
  }
}

const hideSplash = () => {
  const splash = document.getElementById('splash-overlay');
  const card = splash?.querySelector('.splash-card') as HTMLElement;
  if (card) {
    card.classList.add('splash-card-fade-out');
  }
  if (splash) {
    splash.classList.add('splash-fade-out');
    setTimeout(() => splash.remove(), 400);
  }
};

onMounted(async () => {
  setTimeout(hideSplash, 1200);
  try {
    await authStore.bootstrap();
  } catch {
    /* bootstrap errors handled internally */
  }

  if (authStore.currentUser) {
    try {
      await syncStore.refreshAll();
    } catch {
      /* bootstrap errors handled internally */
    }

    syncStore.startAutoSync();
  }

  document.addEventListener('keydown', handleGlobalKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', handleGlobalKeydown);
});
</script>

<template>
  <div v-if="isAuthScreen" class="auth-router">
    <RouterView />
  </div>
  <div v-else class="app-shell">
    <a href="#main-content" class="skip-link">ข้ามไปยังเนื้อหาหลัก</a>
    <AppSidebar />
    <div class="app-main">
      <AppHeader />
      <main id="main-content" class="app-content" tabindex="-1">
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