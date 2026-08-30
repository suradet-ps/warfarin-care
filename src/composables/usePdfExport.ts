import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { toPng } from 'html-to-image';
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

  // The capture target is parked off-screen (position: fixed; left: -9999px)
  // so it stays out of the visible layout. html-to-image clones the element
  // at its computed position, so an off-screen element is drawn outside the
  // canvas and the exported PDF comes out blank. Park the element at the
  // viewport origin for the duration of the capture, then restore it.
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
    return await toPng(element, {
      backgroundColor,
      cacheBust: true,
      pixelRatio: Math.max(window.devicePixelRatio, 2),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
      // Position the cloned node at the canvas origin too: the copied
      // computed style keeps `position: fixed`, which can mis-render
      // inside the SVG foreignObject.
      style: {
        position: 'absolute',
        left: '0',
        top: '0',
        margin: '0',
      },
    });
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

function loadImage(imageData: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('failed to decode captured report image'));
    image.src = imageData;
  });
}

interface ImageSlice {
  dataUrl: string;
  heightPx: number;
}

function sliceImage(image: HTMLImageElement, pageCount: number): ImageSlice[] {
  const sliceHeight = image.naturalHeight / pageCount;
  const canvas = document.createElement('canvas');
  canvas.width = image.naturalWidth;
  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('canvas 2d context unavailable');
  }

  const slices: ImageSlice[] = [];
  for (let page = 0; page < pageCount; page += 1) {
    const sourceY = Math.round(page * sliceHeight);
    const currentSliceHeight = Math.min(Math.round(sliceHeight), image.naturalHeight - sourceY);
    // Resizing the canvas clears it, so only the current slice is drawn.
    canvas.height = currentSliceHeight;
    context.drawImage(
      image,
      0,
      sourceY,
      image.naturalWidth,
      currentSliceHeight,
      0,
      0,
      image.naturalWidth,
      currentSliceHeight,
    );
    slices.push({ dataUrl: canvas.toDataURL('image/png'), heightPx: currentSliceHeight });
  }
  return slices;
}

/**
 * Builds a PDF from a single captured image, slicing it across A4 pages when
 * the report is taller than one page so text stays readable on long lists.
 */
async function buildMultiPagePdf(
  imageData: string,
  elementWidth: number,
  elementHeight: number,
): Promise<jsPDF> {
  const pdf = new jsPDF({
    orientation: 'portrait',
    unit: 'mm',
    format: 'a4',
    compress: true,
  });
  const pageWidth = pdf.internal.pageSize.getWidth();
  const pageHeight = pdf.internal.pageSize.getHeight();
  const scale = pageWidth / elementWidth;
  const totalHeight = elementHeight * scale;
  const pageCount = Math.max(1, Math.ceil(totalHeight / pageHeight));

  if (pageCount === 1) {
    const offsetY = Math.max(0, (pageHeight - totalHeight) / 2);
    pdf.addImage(imageData, 'PNG', 0, offsetY, pageWidth, totalHeight, undefined, 'FAST');
    return pdf;
  }

  const image = await loadImage(imageData);
  const slices = sliceImage(image, pageCount);
  slices.forEach((slice, index) => {
    if (index > 0) {
      pdf.addPage();
    }
    pdf.addImage(
      slice.dataUrl,
      'PNG',
      0,
      0,
      pageWidth,
      (slice.heightPx / image.naturalWidth) * pageWidth,
      undefined,
      'FAST',
    );
  });
  return pdf;
}

/**
 * Captures a hidden printable element and saves it as an A4 PDF via the
 * system save dialog. Returns true when a file was written.
 */
export function usePdfExport(captureElement: Ref<HTMLElement | null>) {
  const exporting = ref(false);
  const error = ref<string | null>(null);

  async function exportPdf(fileName: string): Promise<boolean> {
    const element = captureElement.value;
    if (!element) {
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

      const imageData = await captureElementImage(element);
      if (await isBlankImage(imageData)) {
        throw new Error('ไม่สามารถสร้างภาพรายงานได้ (ภาพว่างเปล่า)');
      }
      const rect = element.getBoundingClientRect();
      const pdf = await buildMultiPagePdf(imageData, rect.width, rect.height);

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
