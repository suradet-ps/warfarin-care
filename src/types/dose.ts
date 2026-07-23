export interface PillRenderData {
  mg: number;
  count: number;
  is_half: boolean;
}

export interface DaySchedule {
  day_index: number;
  total_dose: number;
  pills: PillRenderData[];
  is_stop_day: boolean;
  is_special_day: boolean;
}

export interface PillLineSummary {
  mg: number;
  dispensed_count: number;
  usage_note: string;
}

export interface TotalPillsSummary {
  header: string;
  pill_lines: PillLineSummary[];
}

export interface RegimenOption {
  description: string;
  weekly_dose_actual: number;
  weekly_schedule: DaySchedule[];
  total_pills_summary: TotalPillsSummary;
}

export interface DoseOptionsInput {
  weekly_dose: number;
  allow_half: boolean;
  available_pills: number[];
  special_day_pattern: string;
  days_until_appointment: number;
  start_day_of_week: number;
}

export type AvailablePills = Record<number, boolean>;

export const DEFAULT_AVAILABLE_PILLS: AvailablePills = {
  5: true,
  3: true,
  2: true,
};
