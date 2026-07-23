import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { EnrollmentInput, PatientDetail, WfPatient } from '#/types/patient.ts';

export const usePatientStore = defineStore('patient', () => {
  const activePatients = ref<WfPatient[]>([]);
  const currentPatient = ref<PatientDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchActivePatients() {
    loading.value = true;
    error.value = null;
    try {
      activePatients.value = await invoke<WfPatient[]>('get_active_patients');
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchPatientDetail(hn: string) {
    loading.value = true;
    error.value = null;
    try {
      currentPatient.value = await invoke<PatientDetail>('get_patient_detail', { hn });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function enrollPatient(input: EnrollmentInput) {
    const id = await invoke<number>('enroll_patient', { input });
    await fetchActivePatients();
    return id;
  }

  async function updateStatus(hn: string, status: string, reason: string, effectiveDate?: string) {
    await invoke('update_patient_status', { hn, status, reason, effectiveDate });
    await fetchActivePatients();
  }

  return {
    activePatients,
    currentPatient,
    loading,
    error,
    fetchActivePatients,
    fetchPatientDetail,
    enrollPatient,
    updateStatus,
  };
});
