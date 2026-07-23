import type { RegimenOption } from '#/types/dose.ts';

export interface DoseSchedule {
  mon: number;
  tue: number;
  wed: number;
  thu: number;
  fri: number;
  sat: number;
  sun: number;
}

export interface PillLineSummary {
  mg: number;
  dispensedCount: number;
  usageNote: string;
}

export interface TotalPillsSummary {
  header: string;
  pillLines: PillLineSummary[];
}

export interface WfVisit {
  id: number;
  hn: string;
  visitDate: string;
  inrValue?: number;
  inrSource?: 'lab_order' | 'lab_app_order' | 'manual';
  currentDoseMgday?: number;
  doseDetail?: DoseSchedule;
  newDoseMgday?: number;
  newDoseDetail?: DoseSchedule;
  newDoseDescription?: string;
  doseChanged: boolean;
  nextAppointment?: string;
  nextInrDue?: string;
  physician?: string;
  notes?: string;
  sideEffects?: string[];
  adherence?: 'good' | 'fair' | 'poor';
  createdBy?: string;
  createdAt: string;
  totalPillsSummary?: TotalPillsSummary;
  selectedDoseOption?: RegimenOption;
  reviewedAt?: string;
  reviewedBy?: string;
}

export interface VisitInput {
  hn: string;
  visitDate: string;
  inrValue?: number;
  inrSource?: string;
  currentDoseMgday?: number;
  doseDetail?: DoseSchedule;
  newDoseMgday?: number;
  newDoseDetail?: DoseSchedule;
  newDoseDescription?: string;
  doseChanged: boolean;
  nextAppointment?: string;
  nextInrDue?: string;
  physician?: string;
  notes?: string;
  sideEffects?: string[] | null;
  adherence?: string;
  createdBy?: string;
  selectedDoseOption?: RegimenOption;
}

export interface DoseSuggestion {
  suggestedDoseMgweek: number;
  adjustmentPercent: number;
  recommendation: string;
  urgency: 'normal' | 'caution' | 'urgent' | 'hold';
  recheckDays: number;
}
