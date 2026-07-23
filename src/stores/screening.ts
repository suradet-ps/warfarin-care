import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { PatientDrugRecord } from '#/types/patient.ts';
import { dateInputToday, dateInputYearsAgo } from '#/utils/clinic.ts';

export interface SearchFilters {
  keyword?: string;
  dateFrom?: string;
  dateTo?: string;
  enrollmentStatus?: 'all' | 'enrolled' | 'not_enrolled';
  page: number;
  pageSize: number;
}

export const useScreeningStore = defineStore('screening', () => {
  const results = ref<PatientDrugRecord[]>([]);
  const total = ref(0);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const filters = ref<SearchFilters>({
    keyword: '',
    dateFrom: dateInputYearsAgo(1),
    dateTo: dateInputToday(),
    enrollmentStatus: 'all',
    page: 1,
    pageSize: 50,
  });

  function formatInvokeError(input: unknown): string {
    if (typeof input === 'string') {
      return input;
    }
    if (input instanceof Error) {
      return input.message;
    }
    if (
      input &&
      typeof input === 'object' &&
      'message' in input &&
      typeof input.message === 'string'
    ) {
      return input.message;
    }
    return 'เกิดข้อผิดพลาดที่ไม่ทราบสาเหตุ';
  }

  async function search() {
    loading.value = true;
    error.value = null;
    try {
      const res = await invoke<{ items: PatientDrugRecord[]; total: number }>(
        'search_warfarin_patients',
        {
          filters: filters.value,
        },
      );
      results.value = res.items;
      total.value = res.total;
    } catch (e) {
      results.value = [];
      total.value = 0;
      error.value = formatInvokeError(e);
    } finally {
      loading.value = false;
    }
  }

  function setPage(page: number) {
    filters.value.page = Math.max(1, page);
  }

  function resetPage() {
    filters.value.page = 1;
  }

  return { results, total, loading, error, filters, search, setPage, resetPage };
});
