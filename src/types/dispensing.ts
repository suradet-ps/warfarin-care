import type { DoseSchedule } from '#/types/visit.ts';

export interface ParsedDoseInfo {
  tabletsPerDose: number;
  mgPerDose: number;
  mgPerWeek: number;
  mgPerDayAverage: number;
  schedule: DoseSchedule;
  matchedDays: string[];
}

export interface DispensingRecord {
  hn: string;
  vn?: string;
  an?: string;
  vstdate: string;
  icode: string;
  drugName: string;
  strength: string;
  qty: number;
  unitprice: number;
  drugusageCode?: string;
  spUseCode?: string;
  usageText?: string;
  parsedDose?: ParsedDoseInfo;
  usageParseNote?: string;
}
