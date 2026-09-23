export type UserRole = 'Admin' | 'Pharmacist' | 'Clinician' | 'Viewer';

export type Permission =
  | 'write_visit'
  | 'approve_visit'
  | 'write_outcome'
  | 'write_appointment'
  | 'write_patient_status'
  | 'enroll_patient'
  | 'manage_interactions'
  | 'manage_settings'
  | 'manage_users';

export interface PublicUser {
  id: number;
  username: string;
  role: UserRole;
  permissions: Permission[];
  createdAt: string;
}

/** Admin list row: account state without the password hash. */
export interface ManagedUser {
  id: number;
  username: string;
  role: UserRole;
  active: boolean;
  lockedUntil: string | null;
  createdAt: string;
}

export interface CreateUserInput {
  username: string;
  password: string;
  role: UserRole;
}

export const ROLE_LABELS: Record<UserRole, string> = {
  Admin: 'ผู้ดูแลระบบ',
  Pharmacist: 'เภสัชกร',
  Clinician: 'แพทย์',
  Viewer: 'ผู้ชม',
};

export interface LoginInput {
  username: string;
  password: string;
}

export interface SetupAdminInput {
  username: string;
  password: string;
}
