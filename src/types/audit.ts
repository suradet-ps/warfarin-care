export interface AuditLogEntry {
  id: number;
  hn?: string;
  action: string;
  actor: string;
  timestamp: string;
  oldValue?: string;
  newValue?: string;
  detail?: string;
  createdAt: string;
}

export interface AuditLogFilter {
  hn?: string;
  action?: string;
  dateFrom?: string;
  dateTo?: string;
  page?: number;
  pageSize?: number;
}

export const AUDIT_ACTIONS: Record<string, string> = {
  visit_saved: 'บันทึก visit',
  visit_updated: 'แก้ไข visit',
  visit_deleted: 'ลบ visit',
  dose_changed: 'เปลี่ยนขนาดยา',
  status_changed: 'เปลี่ยนสถานะ',
  adverse_event: 'บันทึกอาการไม่พึงประสงค์',
  login: 'เข้าสู่ระบบ',
  logout: 'ออกจากระบบ',
  patient_enrolled: 'นำผู้ป่วยเข้าคลินิก',
  interaction_check: 'ตรวจสอบปฏิกิริยา',
};
