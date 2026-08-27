<script setup lang="ts">
import { computed } from 'vue';
import type { InrRecord } from '#/types/inr.ts';
import { formatThaiDate } from '#/utils/clinic.ts';

const props = defineProps<{
  inrRecords: InrRecord[];
  targetLow: number;
  targetHigh: number;
}>();

const chartWidth = 960;
const chartHeight = 320;
const padding = { top: 20, right: 56, bottom: 40, left: 20 };

type InrPointStatus = 'in_range' | 'above' | 'below' | 'critical_high' | 'critical_low';
type ChartPoint = InrRecord & { x: number; y: number; status: InrPointStatus };

function classifyInr(value: number, low: number, high: number): InrPointStatus {
  if (value > 4.0) {
    return 'critical_high';
  }
  if (value < 1.5) {
    return 'critical_low';
  }
  if (value > high) {
    return 'above';
  }
  if (value < low) {
    return 'below';
  }
  return 'in_range';
}

const inrPointColors: Record<InrPointStatus, string> = {
  in_range: 'var(--color-inr-safe)',
  above: 'var(--color-inr-high)',
  below: 'var(--color-inr-low)',
  critical_high: 'var(--color-inr-critical)',
  critical_low: 'var(--color-inr-critical)',
};

const displayRecords = computed(() => {
  const ordered = [...props.inrRecords].sort((a, b) => a.date.localeCompare(b.date));
  const latest = ordered.at(-1);
  if (!latest) {
    return [];
  }

  const latestDate = new Date(latest.date);
  if (Number.isNaN(latestDate.getTime())) {
    return ordered;
  }

  const windowStart = new Date(latestDate);
  windowStart.setFullYear(windowStart.getFullYear() - 1);
  const startKey = windowStart.toISOString().slice(0, 10);

  return ordered.filter((record) => record.date >= startKey);
});

const valueRange = computed(() => {
  const values = displayRecords.value.map((record) => record.value);
  if (values.length === 0) {
    return null;
  }

  const rawMin = Math.min(...values, props.targetLow, props.targetHigh);
  const rawMax = Math.max(...values, props.targetLow, props.targetHigh);
  const min = Math.max(0, Math.floor((rawMin - 0.4) * 2) / 2);
  const max = Math.ceil((rawMax + 0.4) * 2) / 2;

  return { min, max: max === min ? max + 1 : max };
});

const plotWidth = computed(() => chartWidth - padding.left - padding.right);
const plotHeight = computed(() => chartHeight - padding.top - padding.bottom);

function xForDate(date: string): number {
  const first = displayRecords.value[0];
  const last = displayRecords.value.at(-1);
  if (!(first && last)) {
    return padding.left;
  }

  const firstTime = new Date(first.date).getTime();
  const lastTime = new Date(last.date).getTime();
  const currentTime = new Date(date).getTime();
  const span = Math.max(lastTime - firstTime, 1);
  const offset = currentTime - firstTime;
  return padding.left + (offset / span) * plotWidth.value;
}

function yForValue(value: number): number {
  const range = valueRange.value;
  if (!range) {
    return padding.top;
  }
  return padding.top + ((range.max - value) / (range.max - range.min)) * plotHeight.value;
}

const points = computed<ChartPoint[]>(() =>
  displayRecords.value.map((record) => ({
    ...record,
    x: xForDate(record.date),
    y: yForValue(record.value),
    status: classifyInr(record.value, props.targetLow, props.targetHigh),
  })),
);

function buildSmoothPath(series: ChartPoint[]): string {
  if (series.length === 0) {
    return '';
  }
  if (series.length === 1) {
    return `M ${series[0].x} ${series[0].y}`;
  }
  if (series.length === 2) {
    return `M ${series[0].x} ${series[0].y} L ${series[1].x} ${series[1].y}`;
  }

  let path = `M ${series[0].x} ${series[0].y}`;
  for (let index = 0; index < series.length - 1; index += 1) {
    const p0 = series[index - 1] ?? series[index];
    const p1 = series[index];
    const p2 = series[index + 1];
    const p3 = series[index + 2] ?? p2;

    const cp1x = p1.x + (p2.x - p0.x) / 6;
    const cp1y = p1.y + (p2.y - p0.y) / 6;
    const cp2x = p2.x - (p3.x - p1.x) / 6;
    const cp2y = p2.y - (p3.y - p1.y) / 6;

    path += ` C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${p2.x} ${p2.y}`;
  }
  return path;
}

const linePath = computed(() => buildSmoothPath(points.value));

const lineSegments = computed(() => {
  if (points.value.length < 2) {
    return [];
  }
  const segments: { path: string; color: string }[] = [];
  for (let i = 0; i < points.value.length - 1; i++) {
    const p1 = points.value[i];
    const p2 = points.value[i + 1];
    const segPoints = [p1, p2];
    const path = buildSmoothPath(segPoints);
    const color =
      p1.status === 'in_range' && p2.status === 'in_range'
        ? 'var(--color-inr-safe)'
        : p1.status.startsWith('critical') || p2.status.startsWith('critical')
          ? 'var(--color-inr-critical)'
          : p1.status === 'above' || p2.status === 'above'
            ? 'var(--color-inr-high)'
            : 'var(--color-inr-low)';
    segments.push({ path, color });
  }
  return segments;
});

const targetBand = computed(() => {
  if (!valueRange.value) {
    return null;
  }
  const yTop = yForValue(props.targetHigh);
  const yBottom = yForValue(props.targetLow);
  return {
    y: Math.min(yTop, yBottom),
    height: Math.abs(yBottom - yTop),
    top: yTop,
    bottom: yBottom,
  };
});

const yTicks = computed(() => {
  const range = valueRange.value;
  if (!range) {
    return [];
  }

  const step = Math.max(0.5, Math.ceil(((range.max - range.min) / 4) * 2) / 2);
  const ticks: number[] = [];
  for (let value = range.min; value <= range.max + 0.001; value += step) {
    ticks.push(Number(value.toFixed(1)));
  }
  return ticks;
});

const xTicks = computed(() => {
  const records = displayRecords.value;
  if (records.length === 0) {
    return [];
  }
  if (records.length <= 6) {
    return records.map((record) => ({
      label: new Date(record.date).toLocaleDateString('th-TH', { month: 'short', day: 'numeric' }),
      x: xForDate(record.date),
    }));
  }

  const lastIndex = records.length - 1;
  return Array.from({ length: 6 }, (_, index) => {
    const pointIndex = Math.round((index / 5) * lastIndex);
    const record = records[pointIndex];
    return {
      label: new Date(record.date).toLocaleDateString('th-TH', { month: 'short' }),
      x: xForDate(record.date),
    };
  });
});
</script>

<template>
  <div class="trend-card card">
    <div class="trend-header">
      <div>
        <h3 class="h5">INR ย้อนหลัง</h3>
        <p class="body-sm trend-meta" v-if="displayRecords.length">
          ล่าสุด: {{ formatThaiDate(displayRecords[displayRecords.length - 1]?.date) }} · แสดงย้อนหลัง 1 ปี
        </p>
      </div>
      <span class="badge badge-success">เป้าหมาย {{ targetLow.toFixed(1) }}-{{ targetHigh.toFixed(1) }}</span>
    </div>

    <div v-if="!displayRecords.length" class="trend-empty card">
      <p class="body-sm" style="color: var(--color-stone)">ไม่มีข้อมูล INR</p>
    </div>

    <div v-else class="chart-legend" aria-label="ตำนานสีกราฟ INR">
      <span class="legend-item"><span class="legend-dot" style="background: var(--color-inr-safe)"></span>อยู่ในเป้าหมาย</span>
      <span class="legend-item"><span class="legend-dot" style="background: var(--color-inr-low)"></span>ต่ำกว่าเป้าหมาย</span>
      <span class="legend-item"><span class="legend-dot" style="background: var(--color-inr-high)"></span>สูงกว่าเป้าหมาย</span>
      <span class="legend-item"><span class="legend-dot" style="background: var(--color-inr-critical)"></span>วิกฤต</span>
    </div>

    <div v-if="displayRecords.length" class="chart-shell">
      <svg class="chart-svg" :viewBox="`0 0 ${chartWidth} ${chartHeight}`" role="img" :aria-label="`กราฟแนวโน้ม INR แสดง ${displayRecords.length} ค่าย้อนหลัง 1 ปี เป้าหมาย ${targetLow.toFixed(1)} ถึง ${targetHigh.toFixed(1)}`">
        <rect
          v-if="targetBand"
          :x="padding.left"
          :y="targetBand.y"
          :width="plotWidth"
          :height="targetBand.height"
          class="target-band"
        />

        <g class="grid">
          <line
            v-for="tick in yTicks"
            :key="`y-${tick}`"
            :x1="padding.left"
            :y1="yForValue(tick)"
            :x2="chartWidth - padding.right"
            :y2="yForValue(tick)"
          />
        </g>

        <g class="targets">
          <line
            :x1="padding.left"
            :y1="yForValue(targetHigh)"
            :x2="chartWidth - padding.right"
            :y2="yForValue(targetHigh)"
            class="target-line"
          />
          <line
            :x1="padding.left"
            :y1="yForValue(targetLow)"
            :x2="chartWidth - padding.right"
            :y2="yForValue(targetLow)"
            class="target-line"
          />
        </g>

        <path
          v-for="(seg, idx) in lineSegments"
          :key="`seg-${idx}`"
          :d="seg.path"
          :stroke="seg.color"
          fill="none"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="trend-line-segment"
        />

        <g class="point-layer">
          <circle
            v-for="point in points"
            :key="`${point.date}-${point.value}`"
            :cx="point.x"
            :cy="point.y"
            r="5"
            :fill="inrPointColors[point.status]"
            stroke="var(--color-canvas)"
            stroke-width="2"
            class="trend-point"
            :aria-label="`INR ${point.value.toFixed(2)} วันที่ ${point.date} - ${point.status === 'in_range' ? 'อยู่ในเป้าหมาย' : point.status === 'above' ? 'สูงกว่าเป้าหมาย' : point.status === 'below' ? 'ต่ำกว่าเป้าหมาย' : 'วิกฤต'}`"
          />
        </g>

        <g class="x-axis">
          <template v-for="tick in xTicks" :key="`x-${tick.x}-${tick.label}`">
            <line :x1="tick.x" :y1="padding.top" :x2="tick.x" :y2="chartHeight - padding.bottom" class="axis-grid" />
            <text :x="tick.x" :y="chartHeight - 12" text-anchor="middle">{{ tick.label }}</text>
          </template>
        </g>

        <g class="y-axis">
          <text
            v-for="tick in yTicks"
            :key="`label-${tick}`"
            :x="chartWidth - 8"
            :y="yForValue(tick) + 4"
            text-anchor="end"
          >
            {{ tick.toFixed(1) }}
          </text>
        </g>
      </svg>
    </div>
  </div>
</template>

<style scoped>
.trend-card { display: flex; flex-direction: column; gap: var(--spacing-lg); }
.trend-header { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--spacing-md); }
.trend-meta { color: var(--color-slate); }
.trend-empty { display: grid; place-items: center; min-height: 16rem; }
.chart-legend {
  display: flex;
  gap: var(--spacing-lg);
  flex-wrap: wrap;
  padding: var(--spacing-xs) 0;
}
.legend-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: var(--typography-caption-size);
  color: var(--color-slate);
}
.legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.chart-shell {
  border: 1px solid var(--color-hairline-soft);
  border-radius: var(--rounded-xl);
  overflow: hidden;
  background: linear-gradient(180deg, rgba(255,255,255,0.96) 0%, rgba(248,248,248,0.98) 100%);
}
.chart-svg {
  display: block;
  width: 100%;
  height: auto;
}
.grid line,
.axis-grid {
  stroke: var(--color-hairline-soft);
  stroke-width: 1;
}
.target-band {
  fill: color-mix(in srgb, var(--color-inr-safe) 10%, transparent);
}
.target-line {
  stroke: var(--color-inr-safe);
  stroke-width: 1.5;
  stroke-dasharray: 7 7;
}
.trend-line-segment {
  opacity: 0.85;
}
.trend-point {
  cursor: pointer;
  transition: r 0.15s ease;
}
.trend-point:hover {
  r: 7;
}
.x-axis text,
.y-axis text {
  fill: var(--color-slate);
  font-size: var(--typography-micro-size);
}
</style>
