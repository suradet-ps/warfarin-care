export type AlertSeverity = 'critical' | 'warning';

export interface PatientAlert {
  hn: string;
  patientName: string;
  alertType:
    | 'critical_high_inr'
    | 'critical_low_inr'
    | 'inr_above_range'
    | 'inr_below_range'
    | 'no_recent_inr'
    | 'missed_appointment'
    | 'low_ttr'
    | 'adverse_event_recent';
  severity: AlertSeverity;
  message: string;
  value?: number;
  date?: string;
}
