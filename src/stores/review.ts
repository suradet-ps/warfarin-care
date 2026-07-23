import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useReviewStore = defineStore('review', () => {
  const pendingCount = ref(0);

  async function fetchPendingCount() {
    try {
      pendingCount.value = await invoke<number>('get_pending_review_count');
    } catch {}
  }

  return {
    pendingCount,
    fetchPendingCount,
  };
});
