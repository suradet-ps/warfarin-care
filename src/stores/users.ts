import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { CreateUserInput, ManagedUser, UserRole } from '#/types/auth.ts';

export const useUsersStore = defineStore('users', () => {
  const users = ref<ManagedUser[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function loadUsers() {
    loading.value = true;
    error.value = null;
    try {
      users.value = await invoke<ManagedUser[]>('list_users');
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function createUser(input: CreateUserInput) {
    await invoke('create_user', { input });
    await loadUsers();
  }

  async function resetPassword(userId: number, newPassword: string) {
    await invoke('reset_user_password', { userId, newPassword });
    await loadUsers();
  }

  async function setRole(userId: number, role: UserRole) {
    await invoke('set_user_role', { userId, role });
    await loadUsers();
  }

  async function setActive(userId: number, active: boolean) {
    await invoke('set_user_active', { userId, active });
    await loadUsers();
  }

  return {
    users,
    loading,
    error,
    loadUsers,
    createUser,
    resetPassword,
    setRole,
    setActive,
  };
});
