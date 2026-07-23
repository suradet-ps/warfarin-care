import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { LoginInput, PublicUser, SetupAdminInput } from '#/types/auth.ts';

const BOOTSTRAPPED_KEY = '__warfarin_auth_bootstrap';

export const useAuthStore = defineStore('auth', () => {
  const hasUsers = ref(false);
  const currentUser = ref<PublicUser | null>(null);
  const bootstrapped = ref(false);
  const loading = ref(false);
  const error = ref<string | null>(null);

  function reset() {
    error.value = null;
  }

  async function bootstrap() {
    if (bootstrapped.value) {
      return;
    }
    try {
      hasUsers.value = await invoke<boolean>('has_users');
      const user = await invoke<PublicUser | null>('current_user');
      currentUser.value = user;
    } catch (e) {
      error.value = String(e);
    } finally {
      bootstrapped.value = true;
    }
  }

  async function login(input: LoginInput): Promise<PublicUser> {
    loading.value = true;
    error.value = null;
    try {
      const user = await invoke<PublicUser>('login', { input });
      currentUser.value = user;
      hasUsers.value = true;
      return user;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function setupAdmin(input: SetupAdminInput): Promise<PublicUser> {
    loading.value = true;
    error.value = null;
    try {
      const user = await invoke<PublicUser>('setup_admin', { input });
      currentUser.value = user;
      hasUsers.value = true;
      return user;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function logout() {
    try {
      await invoke('logout');
    } catch {
    } finally {
      currentUser.value = null;
    }
  }

  return {
    hasUsers,
    currentUser,
    bootstrapped,
    loading,
    error,
    bootstrap,
    login,
    setupAdmin,
    logout,
    reset,
  };
});

// Marker used by the router guard to ensure `bootstrap` ran at least once
// before the first navigation decision.
export const authBootstrapKey = BOOTSTRAPPED_KEY;
