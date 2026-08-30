import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { toPng } from 'html-to-image';

type CaptureOptions = Parameters<typeof toPng>[1];

import jsPDF from 'jspdf';
import { nextTick, type Ref, ref } from 'vue';
import { getCssVar } from '#/utils/clinic.ts';

function normalizeError(errorValue: unknown): string {
  if (errorValue instanceof Error) {
    return errorValue.message;
  }
  return String(errorValue);
}

function ensurePdfExtension(path: string): string {
  return path.toLowerCase().endsWith('.pdf') ? path : `${path}.pdf`;
}

async function waitForStableCapture(): Promise<void> {
  await nextTick();
  await document.fonts.ready;
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
}

async function captureElementImage(element: HTMLElement): Promise<string> {
  const backgroundColor = getCssVar('--color-canvas') || 'white';
  const rect = element.getBoundingClientRect();
  const options: CaptureOptions = {
    backgroundColor,
    cacheBust: true,
    pixelRatio: Math.max(window.devicePixelRatio, 2),
    width: Math.round(rect.width),
    height: Math.round(rect.height),
    // The capture target is parked off-screen (position: fixed;
    // left: -9999px) so it stays out of the visible layout. html-to-image
    // clones the element at its computed position, so place the clone at
    // the canvas origin; keeping the copied `position: fixed` renders the
    // content outside the canvas and the PDF comes out blank.
    style: {
      position: 'absolute',
      left: '0',
      top: '0',
      margin: '0',
    },
  };

  const imageData = await toPng(element, options);
  if (!(await isBlankImage(imageData))) {
    return imageData;
  }

  // Fallback for engines where the off-screen clone still renders blank:
  // park the element at the viewport origin for one retry, then restore it.
  const previous = {
    position: element.style.position,
    left: element.style.left,
    top: element.style.top,
    zIndex: element.style.zIndex,
  };
  element.style.position = 'fixed';
  element.style.left = '0';
  element.style.top = '0';
  element.style.zIndex = '9999';
  try {
    return await toPng(element, options);
  } finally {
    element.style.position = previous.position;
    element.style.left = previous.left;
    element.style.top = previous.top;
    element.style.zIndex = previous.zIndex;
  }
}

/**
 * Detects a fully-background capture (e.g. a render failure inside
 * html-to-image) so a blank PDF is never silently written to disk.
 */
const MAX_RGB_CHANNEL = 255;
const ALPHA_CHANNEL_OFFSET = 3;
const BLANK_SAMPLE_STRIDE_BYTES = 64;

function loadImage(imageData: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('failed to decode captured report image'));
    image.src = imageData;
  });
}

async function isBlankImage(imageData: string): Promise<boolean> {
  const image = await loadImage(imageData);
  const canvas = document.createElement('canvas');
  canvas.width = image.naturalWidth;
  canvas.height = image.naturalHeight;
  const context = canvas.getContext('2d');
  if (!context) {
    return false;
  }
  context.drawImage(image, 0, 0);
  const { data } = context.getImageData(0, 0, canvas.width, canvas.height);
  for (let offset = 0; offset < data.length; offset += BLANK_SAMPLE_STRIDE_BYTES) {
    const alpha = data[offset + ALPHA_CHANNEL_OFFSET];
    const isNonWhite =
      data[offset] !== MAX_RGB_CHANNEL ||
      data[offset + 1] !== MAX_RGB_CHANNEL ||
      data[offset + 2] !== MAX_RGB_CHANNEL;
    if (alpha !== 0 && isNonWhite) {
      return false;
    }
  }
  return true;
}

async function buildPdfFromTargets(targets: HTMLElement[]): Promise<jsPDF> {
  const pdf = new jsPDF({
    orientation: 'portrait',
    unit: 'mm',
    format: 'a4',
    compress: true,
  });
  const pageWidth = pdf.internal.pageSize.getWidth();
  const pageHeight = pdf.internal.pageSize.getHeight();

  const captures = await Promise.all(
    targets.map(async (target) => {
      const imageData = await captureElementImage(target);
      if (await isBlankImage(imageData)) {
        throw new Error('ไม่สามารถสร้างภาพรายงานได้ (ภาพว่างเปล่า)');
      }
      const rect = target.getBoundingClientRect();
      return { imageData, rect };
    }),
  );

  captures.forEach(({ imageData, rect }, index) => {
    if (index > 0) {
      pdf.addPage();
    }
    const renderHeight = (rect.height / rect.width) * pageWidth;
    const offsetY = Math.max(0, (pageHeight - renderHeight) / 2);
    pdf.addImage(imageData, 'PNG', 0, offsetY, pageWidth, renderHeight, undefined, 'FAST');
  });
  return pdf;
}

/**
 * Captures a printable element (or its paginated `.report-page` children)
 * and saves it as an A4 PDF via the system save dialog. Returns true when
 * a file was written.
 */
export function usePdfExport(captureElement: Ref<HTMLElement | null>, pageSelector?: string) {
  const exporting = ref(false);
  const error = ref<string | null>(null);

  async function exportPdf(fileName: string): Promise<boolean> {
    const container = captureElement.value;
    if (!container) {
      return false;
    }

    exporting.value = true;
    error.value = null;

    try {
      await waitForStableCapture();

      const filePath = await save({
        defaultPath: fileName,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });

      if (!filePath) {
        return false;
      }

      const targets = pageSelector
        ? Array.from(container.querySelectorAll<HTMLElement>(pageSelector))
        : [container];
      if (targets.length === 0) {
        throw new Error('รายงานยังไม่พร้อมส่งออก (ไม่พบหน้าพิมพ์)');
      }

      const pdf = await buildPdfFromTargets(targets);
      const bytes = new Uint8Array(pdf.output('arraybuffer'));
      await invoke('save_slip_pdf', {
        path: ensurePdfExtension(filePath),
        bytes: Array.from(bytes),
      });
      return true;
    } catch (errorValue) {
      error.value = normalizeError(errorValue);
      return false;
    } finally {
      exporting.value = false;
    }
  }

  return { exporting, error, exportPdf };
}
