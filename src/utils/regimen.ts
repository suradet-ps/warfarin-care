import type {
  DaySchedule,
  PillRenderData,
  RegimenOption,
  TotalPillsSummary,
} from '#/types/dose.ts';
import type { DoseSchedule } from '#/types/visit.ts';
import { doseDayKeys, normalizeDoseSchedule, scheduleWeeklyTotal } from '#/utils/clinic.ts';

const DAY_LABELS = ['จ.', 'อ.', 'พ.', 'พฤ.', 'ศ.', 'ส.', 'อา.'] as const;

const WHOLE_PILL_UNITS = [
  { value: 5, mg: 5, is_half: false },
  { value: 3, mg: 3, is_half: false },
  { value: 2, mg: 2, is_half: false },
  { value: 2.5, mg: 5, is_half: true },
  { value: 1.5, mg: 3, is_half: true },
  { value: 1, mg: 2, is_half: true },
] as const;

type PillUnit = (typeof WHOLE_PILL_UNITS)[number];

function roundDose(value: number): number {
  return Math.round(value * 10) / 10;
}

function clonePills(pills: PillRenderData[]): PillRenderData[] {
  return pills.map((pill) => ({ ...pill }));
}

function cloneWeeklySchedule(weeklySchedule: DaySchedule[]): DaySchedule[] {
  return weeklySchedule.map((day) => ({
    ...day,
    pills: clonePills(day.pills),
  }));
}

function sameDose(left: number, right: number): boolean {
  return Math.abs(left - right) < 0.01;
}

function normalizeDate(value?: string | null): Date | null {
  if (!value) {
    return null;
  }
  const date = new Date(`${value}T00:00:00`);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  date.setHours(0, 0, 0, 0);
  return date;
}

function modeDose(doses: number[]): number | null {
  if (doses.length === 0) {
    return null;
  }
  const counts = new Map<number, number>();
  for (const dose of doses) {
    const rounded = roundDose(dose);
    counts.set(rounded, (counts.get(rounded) ?? 0) + 1);
  }

  return (
    [...counts.entries()].sort((a, b) => {
      if (b[1] !== a[1]) {
        return b[1] - a[1];
      }
      return a[0] - b[0];
    })[0]?.[0] ?? null
  );
}

function buildDoseDescription(schedule: DoseSchedule): string {
  const grouped = new Map<number, string[]>();
  const stopDays: string[] = [];

  for (const [index, key] of doseDayKeys.entries()) {
    const dose = roundDose(schedule[key] ?? 0);
    const label = DAY_LABELS[index];
    if (dose <= 0) {
      stopDays.push(label);
      continue;
    }

    const existing = grouped.get(dose) ?? [];
    existing.push(label);
    grouped.set(dose, existing);
  }

  const positiveDoses = [...grouped.keys()].sort((a, b) => {
    const diff = (grouped.get(b)?.length ?? 0) - (grouped.get(a)?.length ?? 0);
    if (diff !== 0) {
      return diff;
    }
    return a - b;
  });

  if (positiveDoses.length === 0) {
    return stopDays.length > 0
      ? `หยุดยา ${stopDays.length} วัน (${stopDays.join(', ')})`
      : 'ไม่มียาที่ต้องรับประทาน';
  }

  if (positiveDoses.length === 1 && stopDays.length === 0) {
    return `ทุกวัน ${positiveDoses[0].toFixed(1)} mg`;
  }

  const [primaryDose, ...specialDoses] = positiveDoses;
  const parts = [`วันปกติ ${primaryDose.toFixed(1)} mg`];

  for (const dose of specialDoses) {
    const days = grouped.get(dose) ?? [];
    parts.push(`วันพิเศษ ${dose.toFixed(1)} mg (${days.join(', ')})`);
  }

  if (stopDays.length > 0) {
    parts.push(`หยุดยา ${stopDays.length} วัน (${stopDays.join(', ')})`);
  }

  return parts.join(', ');
}

function buildPillsForDose(dose: number): PillRenderData[] {
  const target = Math.round(dose * 2);
  if (target <= 0) {
    return [];
  }

  let best: PillRenderData[] | null = null;

  function search(index: number, remaining: number, chosen: PillRenderData[]) {
    if (remaining === 0) {
      if (
        !best ||
        chosen.reduce((sum, pill) => sum + pill.count, 0) <
          best.reduce((sum, pill) => sum + pill.count, 0)
      ) {
        best = chosen.map((pill) => ({ ...pill }));
      }
      return;
    }

    if (index >= WHOLE_PILL_UNITS.length) {
      return;
    }
    if (
      best &&
      chosen.reduce((sum, pill) => sum + pill.count, 0) >=
        best.reduce((sum, pill) => sum + pill.count, 0)
    ) {
      return;
    }

    const unit = WHOLE_PILL_UNITS[index];
    const unitValue = Math.round(unit.value * 2);
    const maxCount = Math.floor(remaining / unitValue);

    for (let count = maxCount; count >= 0; count -= 1) {
      const nextChosen = chosen.slice();
      if (count > 0) {
        nextChosen.push({
          mg: unit.mg,
          count,
          is_half: unit.is_half,
        });
      }
      search(index + 1, remaining - count * unitValue, nextChosen);
    }
  }

  search(0, target, []);

  return best ?? [];
}

function buildWeeklyScheduleFromDoseSchedule(schedule: DoseSchedule): DaySchedule[] {
  const primaryDose = modeDose(doseDayKeys.map((key) => schedule[key]).filter((dose) => dose > 0));

  return doseDayKeys.map((key, dayIndex) => {
    const totalDose = roundDose(schedule[key] ?? 0);

    return {
      day_index: dayIndex,
      total_dose: totalDose,
      pills: buildPillsForDose(totalDose),
      is_stop_day: totalDose <= 0,
      is_special_day:
        totalDose > 0 && primaryDose !== null ? !sameDose(totalDose, primaryDose) : false,
    };
  });
}

function buildSummaryLines(
  weeklySchedule: DaySchedule[],
  visitDate?: string | null,
  nextAppointment?: string | null,
): TotalPillsSummary {
  const startDate = normalizeDate(visitDate) ?? new Date();
  startDate.setHours(0, 0, 0, 0);

  const endDate = normalizeDate(nextAppointment);
  const totalDays = endDate
    ? Math.max(1, Math.round((endDate.getTime() - startDate.getTime()) / (1000 * 60 * 60 * 24)))
    : 7;

  const pillUsage = new Map<number, { whole: number; half: number }>();
  const scheduleByDay = new Map(weeklySchedule.map((day) => [day.day_index, day]));

  for (let offset = 0; offset < totalDays; offset += 1) {
    const currentDate = new Date(startDate);
    currentDate.setDate(startDate.getDate() + offset);
    const dayIndex = jsDayToDoseDayIndex(currentDate.getDay());
    const regimenDay = scheduleByDay.get(dayIndex);
    if (!regimenDay) {
      continue;
    }

    for (const pill of regimenDay.pills) {
      const usage = pillUsage.get(pill.mg) ?? { whole: 0, half: 0 };
      if (pill.is_half) {
        usage.half += pill.count;
      } else {
        usage.whole += pill.count;
      }
      pillUsage.set(pill.mg, usage);
    }
  }

  const pillLines = [...pillUsage.entries()]
    .sort((a, b) => b[0] - a[0])
    .map(([mg, usage]) => {
      const dispensedCount = usage.whole + Math.ceil(usage.half / 2);
      const usageNoteParts: string[] = [];
      if (usage.whole > 0) {
        usageNoteParts.push(`ใช้เต็มเม็ด ${usage.whole} ครั้ง`);
      }
      if (usage.half > 0) {
        usageNoteParts.push(`ใช้ครึ่งเม็ด ${usage.half} ครั้ง`);
      }

      return {
        mg,
        dispensed_count: dispensedCount,
        usage_note: usageNoteParts.join(', '),
      };
    })
    .filter((line) => line.dispensed_count > 0);

  const header = endDate
    ? `รวมยาถึงวันนัด (${totalDays} วัน): ${visitDate} - ${nextAppointment}`
    : `รวมยาตามตาราง (${totalDays} วัน) เริ่ม ${visitDate ?? '-'}`;

  return {
    header,
    pill_lines: pillLines,
  };
}

export function jsDayToDoseDayIndex(jsDay: number): number {
  return (jsDay + 6) % 7;
}

export function getDosePeriodDays(
  visitDate?: string | null,
  nextAppointment?: string | null,
): number | null {
  const start = normalizeDate(visitDate);
  const end = normalizeDate(nextAppointment);
  if (!(start && end)) {
    return null;
  }

  const diffDays = Math.round((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
  return diffDays > 0 ? diffDays : null;
}

export function regimenOptionMatchesSchedule(
  option: RegimenOption,
  schedule?: Partial<DoseSchedule> | string | null,
): boolean {
  const normalized = normalizeDoseSchedule(schedule);

  return option.weekly_schedule.every((day) => {
    const key = doseDayKeys[day.day_index];
    return key ? sameDose(normalized[key], day.total_dose) : false;
  });
}

export function findMatchingRegimenOption(
  options: RegimenOption[],
  schedule?: Partial<DoseSchedule> | string | null,
): { index: number; option: RegimenOption } | null {
  const index = options.findIndex((option) => regimenOptionMatchesSchedule(option, schedule));
  if (index < 0) {
    return null;
  }

  return {
    index,
    option: options[index],
  };
}

export function createRegimenOptionSnapshot(args: {
  schedule?: Partial<DoseSchedule> | string | null;
  visitDate?: string | null;
  nextAppointment?: string | null;
  baseOption?: RegimenOption | null;
}): RegimenOption {
  const normalizedSchedule = normalizeDoseSchedule(args.schedule);
  const weeklySchedule =
    args.baseOption && regimenOptionMatchesSchedule(args.baseOption, normalizedSchedule)
      ? cloneWeeklySchedule(args.baseOption.weekly_schedule)
      : buildWeeklyScheduleFromDoseSchedule(normalizedSchedule);

  return {
    description: args.baseOption?.description || buildDoseDescription(normalizedSchedule),
    weekly_dose_actual:
      args.baseOption?.weekly_dose_actual ?? scheduleWeeklyTotal(normalizedSchedule),
    weekly_schedule: weeklySchedule,
    total_pills_summary: buildSummaryLines(weeklySchedule, args.visitDate, args.nextAppointment),
  };
}
