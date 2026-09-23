<script setup lang="ts">
import { KeyRound, Plus, ShieldCheck, ShieldX, UserPlus, X } from 'lucide-vue-next';
import { computed, onMounted, ref } from 'vue';
import ConfirmDialog from '#/components/shared/ConfirmDialog.vue';
import { useAuthStore } from '#/stores/auth.ts';
import { useUsersStore } from '#/stores/users.ts';
import { type ManagedUser, ROLE_LABELS, type UserRole } from '#/types/auth.ts';
import { formatThaiDate } from '#/utils/clinic.ts';

const store = useUsersStore();
const authStore = useAuthStore();

const USERNAME_PATTERN = /^[A-Za-z0-9_.-]{3,32}$/;
const PASSWORD_HAS_LETTER = /[A-Za-z]/;
const PASSWORD_HAS_DIGIT = /\d/;

const error = ref<string | null>(null);
const busyId = ref<number | null>(null);

const showCreate = ref(false);
const creating = ref(false);
const createError = ref<string | null>(null);
const createForm = ref({ username: '', password: '', confirm: '', role: 'Pharmacist' as UserRole });

const resetTarget = ref<ManagedUser | null>(null);
const resetting = ref(false);
const resetError = ref<string | null>(null);
const resetForm = ref({ password: '', confirm: '' });

const toggleTarget = ref<ManagedUser | null>(null);

const roleEntries = Object.entries(ROLE_LABELS) as [UserRole, string][];

const orderedUsers = computed(() =>
  [...store.users].sort((a, b) => a.username.localeCompare(b.username)),
);

function isSelf(user: ManagedUser): boolean {
  return authStore.currentUser?.id === user.id;
}

function isLocked(user: ManagedUser): boolean {
  return user.lockedUntil !== null && new Date(user.lockedUntil) > new Date();
}

function passwordProblem(password: string): string | null {
  if (password.length < 8 || password.length > 128) {
    return 'รหัสผ่านต้องมีความยาว 8-128 ตัวอักษร';
  }
  if (!(PASSWORD_HAS_LETTER.test(password) && PASSWORD_HAS_DIGIT.test(password))) {
    return 'รหัสผ่านต้องมีตัวอักษรและตัวเลขอย่างน้อยอย่างละ 1 ตัว';
  }
  return null;
}

function openCreate() {
  createForm.value = { username: '', password: '', confirm: '', role: 'Pharmacist' };
  createError.value = null;
  showCreate.value = true;
}

async function submitCreate() {
  const username = createForm.value.username.trim();
  if (!USERNAME_PATTERN.test(username)) {
    createError.value = 'ชื่อผู้ใช้ต้องยาว 3-32 ตัว และใช้ได้เฉพาะ A-Z a-z 0-9 _ . -';
    return;
  }
  const pwProblem = passwordProblem(createForm.value.password);
  if (pwProblem) {
    createError.value = pwProblem;
    return;
  }
  if (createForm.value.password !== createForm.value.confirm) {
    createError.value = 'รหัสผ่านยืนยันไม่ตรงกัน';
    return;
  }

  creating.value = true;
  createError.value = null;
  try {
    await store.createUser({
      username,
      password: createForm.value.password,
      role: createForm.value.role,
    });
    showCreate.value = false;
  } catch (e) {
    createError.value = String(e);
  } finally {
    creating.value = false;
  }
}

function openReset(user: ManagedUser) {
  resetTarget.value = user;
  resetForm.value = { password: '', confirm: '' };
  resetError.value = null;
}

async function submitReset() {
  if (!resetTarget.value) {
    return;
  }
  const pwProblem = passwordProblem(resetForm.value.password);
  if (pwProblem) {
    resetError.value = pwProblem;
    return;
  }
  if (resetForm.value.password !== resetForm.value.confirm) {
    resetError.value = 'รหัสผ่านยืนยันไม่ตรงกัน';
    return;
  }

  resetting.value = true;
  resetError.value = null;
  try {
    await store.resetPassword(resetTarget.value.id, resetForm.value.password);
    resetTarget.value = null;
  } catch (e) {
    resetError.value = String(e);
  } finally {
    resetting.value = false;
  }
}

async function onRoleChange(user: ManagedUser, event: Event) {
  const role = (event.target as HTMLSelectElement).value as UserRole;
  if (role === user.role) {
    return;
  }
  busyId.value = user.id;
  error.value = null;
  try {
    await store.setRole(user.id, role);
  } catch (e) {
    error.value = String(e);
    await store.loadUsers();
  } finally {
    busyId.value = null;
  }
}

async function confirmToggle() {
  if (!toggleTarget.value) {
    return;
  }
  busyId.value = toggleTarget.value.id;
  error.value = null;
  const target = toggleTarget.value;
  toggleTarget.value = null;
  try {
    await store.setActive(target.id, !target.active);
  } catch (e) {
    error.value = String(e);
  } finally {
    busyId.value = null;
  }
}

onMounted(() => {
  void store.loadUsers();
});
</script>

<template>
  <div class="users-panel">
    <div class="panel-header">
      <div>
        <h3 class="h4">ผู้ใช้งานระบบ</h3>
        <p class="caption panel-meta">
          สร้างบัญชี กำหนดตำแหน่ง และระงับการใช้งาน เฉพาะผู้ดูแลระบบเท่านั้น
        </p>
      </div>
      <button type="button" class="btn btn-primary" @click="openCreate">
        <UserPlus :size="16" /> เพิ่มผู้ใช้
      </button>
    </div>

    <div v-if="error" class="card card-feature-coral body-sm error-banner">{{ error }}</div>

    <div v-if="store.loading" class="card body-sm loading-state">กำลังโหลดรายชื่อผู้ใช้...</div>
    <div v-else-if="store.users.length === 0" class="card body-sm empty-state">
      ยังไม่มีผู้ใช้งานในระบบ
    </div>
    <div v-else class="table-wrap card">
      <table class="table">
        <thead>
          <tr>
            <th>ชื่อผู้ใช้</th>
            <th>ตำแหน่ง</th>
            <th>สถานะ</th>
            <th>สร้างเมื่อ</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="user in orderedUsers" :key="user.id">
            <td>
              <div class="user-cell">
                <span class="body-sm-medium">{{ user.username }}</span>
                <span v-if="isSelf(user)" class="badge badge-tag-purple">คุณ</span>
              </div>
            </td>
            <td>
              <select
                class="input role-select"
                :value="user.role"
                :disabled="isSelf(user) || busyId === user.id"
                :aria-label="`ตำแหน่งของ ${user.username}`"
                @change="onRoleChange(user, $event)"
              >
                <option v-for="[value, label] in roleEntries" :key="value" :value="value">
                  {{ label }}
                </option>
              </select>
            </td>
            <td>
              <div class="status-cell">
                <span v-if="user.active" class="badge badge-success">
                  <ShieldCheck :size="12" /> ใช้งาน
                </span>
                <span v-else class="badge badge-danger">
                  <ShieldX :size="12" /> ระงับ
                </span>
                <span v-if="isLocked(user)" class="badge badge-tag-coral">ถูกล็อก</span>
              </div>
            </td>
            <td class="caption cell-meta">{{ formatThaiDate(user.createdAt) }}</td>
            <td>
              <div class="row-actions">
                <button
                  type="button"
                  class="btn btn-secondary row-btn"
                  :disabled="busyId === user.id"
                  @click="openReset(user)"
                >
                  <KeyRound :size="14" /> รีเซ็ตรหัสผ่าน
                </button>
                <button
                  v-if="user.active"
                  type="button"
                  class="btn btn-secondary row-btn"
                  :disabled="isSelf(user) || busyId === user.id"
                  @click="toggleTarget = user"
                >
                  ระงับการใช้งาน
                </button>
                <button
                  v-else
                  type="button"
                  class="btn btn-primary row-btn"
                  :disabled="busyId === user.id"
                  @click="toggleTarget = user"
                >
                  เปิดใช้งาน
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Create user -->
    <div v-if="showCreate" class="modal-overlay" @click.self="showCreate = false">
      <div class="modal-box card">
        <div class="modal-header">
          <h3 class="h4"><Plus :size="16" /> เพิ่มผู้ใช้ใหม่</h3>
          <button class="btn btn-ghost icon-close" aria-label="ปิด" @click="showCreate = false">
            <X :size="18" />
          </button>
        </div>
        <form class="modal-form" @submit.prevent="submitCreate">
          <label class="form-field">
            <span class="caption form-label">ชื่อผู้ใช้</span>
            <input
              v-model="createForm.username"
              class="input"
              autocomplete="off"
              placeholder="3-32 ตัวอักษร"
            />
          </label>
          <label class="form-field">
            <span class="caption form-label">ตำแหน่ง</span>
            <select v-model="createForm.role" class="input">
              <option v-for="[value, label] in roleEntries" :key="value" :value="value">
                {{ label }}
              </option>
            </select>
          </label>
          <label class="form-field">
            <span class="caption form-label">รหัสผ่าน</span>
            <input
              v-model="createForm.password"
              class="input"
              type="password"
              autocomplete="new-password"
            />
          </label>
          <label class="form-field">
            <span class="caption form-label">ยืนยันรหัสผ่าน</span>
            <input
              v-model="createForm.confirm"
              class="input"
              type="password"
              autocomplete="new-password"
            />
          </label>
          <div v-if="createError" class="badge badge-danger error-msg">{{ createError }}</div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" @click="showCreate = false">
              ยกเลิก
            </button>
            <button type="submit" class="btn btn-primary" :disabled="creating">
              {{ creating ? 'กำลังบันทึก...' : 'สร้างบัญชี' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Reset password -->
    <div v-if="resetTarget" class="modal-overlay" @click.self="resetTarget = null">
      <div class="modal-box card">
        <div class="modal-header">
          <h3 class="h4"><KeyRound :size="16" /> รีเซ็ตรหัสผ่าน: {{ resetTarget.username }}</h3>
          <button class="btn btn-ghost icon-close" aria-label="ปิด" @click="resetTarget = null">
            <X :size="18" />
          </button>
        </div>
        <form class="modal-form" @submit.prevent="submitReset">
          <label class="form-field">
            <span class="caption form-label">รหัสผ่านใหม่</span>
            <input
              v-model="resetForm.password"
              class="input"
              type="password"
              autocomplete="new-password"
            />
          </label>
          <label class="form-field">
            <span class="caption form-label">ยืนยันรหัสผ่านใหม่</span>
            <input
              v-model="resetForm.confirm"
              class="input"
              type="password"
              autocomplete="new-password"
            />
          </label>
          <p class="caption form-label">
            การรีเซ็ตจะปลดล็อกบัญชีที่ถูกล็อกอยู่ด้วย
          </p>
          <div v-if="resetError" class="badge badge-danger error-msg">{{ resetError }}</div>
          <div class="modal-actions">
            <button type="button" class="btn btn-secondary" @click="resetTarget = null">
              ยกเลิก
            </button>
            <button type="submit" class="btn btn-primary" :disabled="resetting">
              {{ resetting ? 'กำลังบันทึก...' : 'รีเซ็ตรหัสผ่าน' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <ConfirmDialog
      :open="toggleTarget !== null"
      :title="toggleTarget?.active ? 'ยืนยันการระงับการใช้งาน' : 'ยืนยันการเปิดใช้งาน'"
      :message="
        toggleTarget?.active
          ? `บัญชี ${toggleTarget?.username} จะไม่สามารถเข้าสู่ระบบได้จนกว่าจะเปิดใช้งานอีกครั้ง`
          : `บัญชี ${toggleTarget?.username} จะสามารถเข้าสู่ระบบได้อีกครั้ง`
      "
      :confirm-label="toggleTarget?.active ? 'ระงับการใช้งาน' : 'เปิดใช้งาน'"
      :variant="toggleTarget?.active ? 'danger' : 'default'"
      @update:open="(value: boolean) => { if (!value) toggleTarget = null }"
      @confirm="confirmToggle"
      @cancel="toggleTarget = null"
    />
  </div>
</template>

<style scoped>
.users-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--spacing-md);
}
.panel-meta {
  color: var(--color-slate);
  margin-top: var(--spacing-xs);
}
.error-banner {
  justify-content: flex-start;
}
.loading-state,
.empty-state {
  color: var(--color-slate);
  padding: var(--spacing-xl);
}
.table-wrap {
  overflow-x: auto;
}
.table {
  width: 100%;
  border-collapse: collapse;
}
.table th,
.table td {
  padding: var(--spacing-sm) var(--spacing-md);
  text-align: left;
  border-bottom: 1px solid var(--color-hairline-soft);
  vertical-align: middle;
}
.table th {
  font-weight: var(--typography-caption-bold-weight);
  color: var(--color-slate);
  font-size: var(--typography-caption-size);
  text-transform: uppercase;
}
.user-cell,
.status-cell {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  flex-wrap: wrap;
}
.cell-meta {
  color: var(--color-slate);
  white-space: nowrap;
}
.role-select {
  padding: 6px 12px;
  font-size: var(--typography-body-sm-size);
  min-width: 9rem;
}
.row-actions {
  display: flex;
  gap: var(--spacing-xs);
  justify-content: flex-end;
}
.row-btn {
  padding: 4px 12px;
  font-size: var(--typography-micro-size);
  gap: var(--spacing-xxs);
}
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(5, 0, 56, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.modal-box {
  width: 480px;
  box-shadow: var(--elevation-4);
  max-height: 90vh;
  overflow-y: auto;
}
.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-xl);
  gap: var(--spacing-sm);
}
.icon-close {
  padding: 4px;
}
.modal-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}
.form-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.form-label {
  color: var(--color-slate);
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-md);
}
.error-msg {
  justify-content: flex-start;
}
</style>
