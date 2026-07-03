export type UserRole = 'Admin' | 'User'

export interface PublicUser {
  id: number
  username: string
  role: UserRole
  createdAt: string
}

export interface LoginInput {
  username: string
  password: string
}

export interface SetupAdminInput {
  username: string
  password: string
}
