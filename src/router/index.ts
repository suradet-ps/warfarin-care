import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '#/stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/screening' },
    { path: '/login', name: 'login', component: () => import('#/views/LoginView.vue') },
    { path: '/setup', name: 'setup', component: () => import('#/views/SetupView.vue') },
    { path: '/screening', name: 'screening', component: () => import('#/views/ScreeningView.vue') },
    { path: '/active', name: 'active', component: () => import('#/views/ActiveView.vue') },
    { path: '/appointments', name: 'appointments', component: () => import('#/views/AppointmentsView.vue') },
    { path: '/patient/:hn', name: 'patient-detail', component: () => import('#/views/PatientDetailView.vue') },
    { path: '/slip/:visitId', name: 'slip', component: () => import('#/views/SlipView.vue') },
    { path: '/review', name: 'review', component: () => import('#/views/ReviewView.vue') },
    { path: '/reports', name: 'reports', component: () => import('#/views/ReportsView.vue') },
    { path: '/settings', name: 'settings', component: () => import('#/views/SettingsView.vue') },
  ],
})

// Routes that do not require an authenticated session.
const PUBLIC_PATHS = new Set<string>(['/login', '/setup'])

router.beforeEach(async (to) => {
  const auth = useAuthStore()
  // Ensure the auth bootstrap has run before the first navigation decision so
  // a logged-in user is not flashed through the login screen on a hard load.
  if (!auth.bootstrapped) {
    await auth.bootstrap()
  }

  const isPublic = PUBLIC_PATHS.has(to.path)

  if (isPublic) {
    if (auth.currentUser) {
      return { path: '/' }
    }
    // First-run: no users yet. Always force the bootstrap onto the setup
    // screen, even if the user typed `/login` or hit the app cold.
    if (!auth.hasUsers && to.path !== '/setup') {
      return { path: '/setup' }
    }
    if (to.path === '/setup' && auth.hasUsers) {
      return { path: '/login' }
    }
    return true
  }

  if (!auth.currentUser) {
    return { path: '/login', query: { redirect: to.fullPath } }
  }
  return true
})

export default router
