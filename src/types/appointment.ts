export interface WfAppointment {
  id: number
  hn: string
  apptDate: string
  apptType?: 'inr_check' | 'clinic_visit' | 'urgent'
  status: 'scheduled' | 'completed' | 'missed' | 'cancelled'
  notes?: string
  createdAt: string
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
