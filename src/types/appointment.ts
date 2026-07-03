export interface WfAppointment {
  id: number
  hn: string
  apptDate: string
  apptType?: 'inr_check' | 'clinic_visit' | 'urgent'
  status: 'scheduled' | 'completed' | 'missed' | 'cancelled'
  notes?: string
  createdAt: string
  /**
   * Backend-computed: true only when the appointment is in the past, the
   * clinic ran that day, and the patient has no visit record for that day.
   * Use this to decide whether to show "เกินนัด" badges. Optional because
   * older callers that don't use the new query leave it undefined.
   */
  isOverdue?: boolean
}

export interface AppointmentInput {
  hn: string
  apptDate: string
  apptType?: string
  notes?: string
}

export interface AppointmentDayLoad {
  apptDate: string
  scheduledCount: number
}
