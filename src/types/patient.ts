import type { WfAppointment } from '#/types/appointment.ts';
import type { DispensingRecord } from '#/types/dispensing.ts';
import type { InrRecord } from '#/types/inr.ts';
import type { WfOutcome } from '#/types/outcome.ts';
import type { WfVisit } from '#/types/visit.ts';

export interface WfPatient {
  id: number;
  hn: string;
  enrolledAt: string;
  enrolledBy?: string;
  status: 'active' | 'inactive' | 'deceased' | 'transferred' | 'discharged';
  indication?: 'AF' | 'DVT' | 'PE' | 'mechanical_valve' | 'other';
  targetInrLow: number;
  targetInrHigh: number;
  notes?: string;
  createdAt: string;
  updatedAt: string;
}

export interface HosxpPatient {
  hn: string;
  pname: string;
  fname: string;
  lname: string;
  birthday: string;
  sex: string;
  addrpart?: string;
  phone?: string;
  informTel?: string;
}

export interface PatientDrugRecord {
  hn: string;
  pname: string;
  fname: string;
  lname: string;
  birthday: string;
  sex: string;
  phone?: string;
  firstDispenseDate: string;
  lastDispenseDate: string;
  totalDispenseVisits: number;
  strengthsReceived: string[];
  isEnrolled: boolean;
  enrollmentStatus?: string;
}

export interface EnrollmentInput {
  hn: string;
  indication: string;
  targetInrLow: number;
  targetInrHigh: number;
  enrolledAt: string;
  enrolledBy: string;
  notes?: string;
}

export interface PatientDetail {
  patient: WfPatient;
  hosxpInfo: HosxpPatient;
  latestInr?: InrRecord;
  currentDoseMgday?: number;
  ttr6months?: number | null;
  nextAppointment?: string;
  inrHistory?: InrRecord[];
  dispensingHistory?: DispensingRecord[];
  visits?: WfVisit[];
  appointments?: WfAppointment[];
  outcomes?: WfOutcome[];
}

export interface ActivePatientSummary {
  patient: WfPatient;
  hosxpInfo: HosxpPatient;
  latestInr?: InrRecord;
  currentDoseMgday?: number;
  ttr6months?: number | null;
  nextAppointment?: string;
  lastVisitDate?: string;
}
