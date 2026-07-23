<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { toPng } from 'html-to-image';
import { jsPDF } from 'jspdf';
import { computed, nextTick, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import type { InrRecord } from '#/types/inr.ts';
import type { PatientDetail } from '#/types/patient.ts';
import type { WfVisit } from '#/types/visit.ts';
import { getCssVar } from '#/utils/clinic.ts';

const route = useRoute();

const visitId = computed(() => Number(route.params.visitId));
const visit = ref<WfVisit | null>(null);
const patient = ref<PatientDetail | null>(null);
const ttr = ref<number | null>(null);
const loading = ref(false);
const printing = ref(false);
const exportingPdf = ref(false);
const loadError = ref<string | null>(null);
const actionError = ref<string | null>(null);
const slipCapture = ref<HTMLElement | null>(null);

const canOutput = computed(
  () => Boolean(visit.value && patient.value && slipCapture.value) && !loading.value,
);
const defaultPdfFileName = computed(() => {
  if (!visit.value) {
    return 'warfarin-slip.pdf';
  }
  return `warfarin-slip-${visit.value.hn}-${visit.value.visitDate}.pdf`;
});

function normalizeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function ensurePdfExtension(path: string): string {
  return path.toLowerCase().endsWith('.pdf') ? path : `${path}.pdf`;
}

async function waitForStableSlip(): Promise<void> {
  await nextTick();
  await document.fonts.ready;
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

async function loadSlip() {
  if (!Number.isInteger(visitId.value) || visitId.value <= 0) {
    visit.value = null;
    patient.value = null;
    ttr.value = null;
    loadError.value = 'visit id ไม่ถูกต้อง';
    loading.value = false;
    return;
  }

  loading.value = true;
  loadError.value = null;
  actionError.value = null;
  visit.value = null;
  patient.value = null;
  ttr.value = null;

  try {
    const currentVisit = await invoke<WfVisit>('get_visit_by_id', { visitId: visitId.value });
    const [patientDetail, inrHistory, ttrValue] = await Promise.all([
      invoke<PatientDetail>('get_patient_detail', { hn: currentVisit.hn }),
      invoke<InrRecord[]>('get_inr_history', { hn: currentVisit.hn }),
      invoke<number | null>('calculate_ttr', { hn: currentVisit.hn, windowDays: 182 }),
    ]);

    visit.value = currentVisit;
    patient.value = { ...patientDetail, inrHistory };
    ttr.value = ttrValue;
  } catch (error) {
    loadError.value = normalizeError(error);
  } finally {
    loading.value = false;
  }
}

async function printSlip() {
  if (!canOutput.value) {
    return;
  }

  printing.value = true;
  actionError.value = null;

  try {
    await waitForStableSlip();
    await Promise.resolve(window.print());
  } catch (error) {
    actionError.value = normalizeError(error);
  } finally {
    printing.value = false;
  }
}

async function exportPdf() {
  if (!(canOutput.value && slipCapture.value && visit.value)) {
    return;
  }

  exportingPdf.value = true;
  actionError.value = null;

  try {
    await waitForStableSlip();

    const filePath = await save({
      defaultPath: defaultPdfFileName.value,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });

    if (!filePath) {
      return;
    }

    const backgroundColor = getCssVar('--color-canvas') || 'white';
    const rect = slipCapture.value.getBoundingClientRect();
    const imageData = await toPng(slipCapture.value, {
      backgroundColor,
      cacheBust: true,
      pixelRatio: Math.max(window.devicePixelRatio, 2),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    });

    const pdf = new jsPDF({
      orientation: 'portrait',
      unit: 'mm',
      format: 'a4',
      compress: true,
    });

    const pageWidth = pdf.internal.pageSize.getWidth();
    const pageHeight = pdf.internal.pageSize.getHeight();
    const imageAspectRatio = rect.height / rect.width;
    let renderWidth = pageWidth;
    let renderHeight = renderWidth * imageAspectRatio;

    if (renderHeight > pageHeight) {
      renderHeight = pageHeight;
      renderWidth = renderHeight / imageAspectRatio;
    }

    const offsetX = (pageWidth - renderWidth) / 2;
    const offsetY = (pageHeight - renderHeight) / 2;
    pdf.addImage(imageData, 'PNG', offsetX, offsetY, renderWidth, renderHeight, undefined, 'FAST');

    const bytes = new Uint8Array(pdf.output('arraybuffer'));
    await invoke('save_slip_pdf', {
      path: ensurePdfExtension(filePath),
      bytes: Array.from(bytes),
    });
  } catch (error) {
    actionError.value = normalizeError(error);
  } finally {
    exportingPdf.value = false;
  }
}

watch(
  visitId,
  () => {
    void loadSlip();
  },
  { immediate: true },
);
</script>

<template>
  <div class="slip-view">
    <div class="slip-toolbar">
      <button type="button" class="btn btn-secondary" :disabled="!canOutput || exportingPdf" @click="exportPdf">
        <Download :size="16" />
        {{ exportingPdf ? 'กำลังสร้าง PDF...' : 'ส่งออก PDF' }}
      </button>
      <button type="button" class="btn btn-primary print-button" :disabled="!canOutput || printing" @click="printSlip">
        <Printer :size="16" />
        {{ printing ? 'กำลังเปิดหน้าพิมพ์...' : 'พิมพ์' }}
      </button>
    </div>

    <div v-if="loading" class="card loading-card body-sm">กำลังโหลด...</div>
    <div v-else-if="loadError" class="card card-feature-coral">{{ loadError }}</div>
    <template v-else-if="visit && patient">
      <div v-if="actionError" class="card card-feature-coral action-error">{{ actionError }}</div>
      <div class="slip-preview-frame">
        <div class="slip-preview-surface">
          <div ref="slipCapture" class="slip-capture">
            <PhysicianSlip :visit="visit" :patient="patient" :ttr="ttr" />
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style>
@page {
  size: A4 portrait;
  margin: 6mm;
}

@media print {
  html,
  body,
  #app {
    background: var(--color-canvas) !important;
  }

  .sidebar,
  .app-header,
  .slip-toolbar,
  .action-error {
    display: none !important;
  }

  .app-shell,
  .app-main,
  .app-content,
  .slip-view,
  .slip-preview-frame,
  .slip-preview-surface,
  .slip-capture {
    display: block !important;
    height: auto !important;
    overflow: visible !important;
    max-width: none !important;
    padding: 0 !important;
    margin: 0 !important;
    background: var(--color-canvas) !important;
  }
}
</style>

<style scoped>
.slip-view {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
  align-items: center;
}

.slip-toolbar {
  width: 100%;
  max-width: calc(210mm + var(--spacing-md) * 2);
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-sm);
}

.slip-preview-frame {
  display: flex;
  justify-content: center;
  width: 100%;
  overflow-x: auto;
  padding-bottom: var(--spacing-sm);
}

.slip-preview-surface {
  border-radius: var(--rounded-xl);
  box-shadow: var(--elevation-2);
}

.slip-capture {
  width: 210mm;
  max-width: 210mm;
}

.loading-card {
  width: 100%;
  max-width: 210mm;
  padding: var(--spacing-xxl);
  text-align: center;
  color: var(--color-slate);
}

.action-error {
  width: 100%;
  max-width: 210mm;
}

@media print {
  .slip-preview-surface {
    box-shadow: none;
    border-radius: 0;
  }
}
</style>
