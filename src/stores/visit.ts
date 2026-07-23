import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { DoseSuggestion, VisitInput, WfVisit } from '#/types/visit.ts';

export const useVisitStore = defineStore('visit', () => {
  const visits = ref<WfVisit[]>([]);
  const currentSuggestion = ref<DoseSuggestion | null>(null);
  const loading = ref(false);

  async function fetchVisits(hn: string) {
    loading.value = true;
    try {
      visits.value = await invoke<WfVisit[]>('get_visit_history', { hn });
    } finally {
      loading.value = false;
    }
  }

  async function saveVisit(input: VisitInput): Promise<number> {
    const id = await invoke<number>('save_visit', { visit: input });
    await fetchVisits(input.hn);
    return id;
  }

  async function updateVisit(visitId: number, input: VisitInput): Promise<void> {
    await invoke('update_visit', { visitId, visit: input });
    await fetchVisits(input.hn);
  }

  async function deleteVisit(visitId: number, hn: string) {
    await invoke('delete_visit', { visitId });
    await fetchVisits(hn);
  }

  async function getSuggestion(
    currentDose: number,
    currentInr: number,
    targetLow: number,
    targetHigh: number,
  ) {
    currentSuggestion.value = await invoke<DoseSuggestion>('suggest_dose', {
      currentDose,
      currentInr,
      targetLow,
      targetHigh,
    });
  }

  return {
    visits,
    currentSuggestion,
    loading,
    fetchVisits,
    saveVisit,
    updateVisit,
    deleteVisit,
    getSuggestion,
  };
});
