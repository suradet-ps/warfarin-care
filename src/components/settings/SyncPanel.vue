<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useSyncStore } from '#/stores/sync.ts';

const syncStore = useSyncStore();

const formUrl = ref('');
const formKey = ref('');
const showKey = ref(false);
const testing = ref(false);
const saving = ref(false);
const actionError = ref<string | null>(null);
const testResult = ref<boolean | null>(null);
const testMessage = ref<string | null>(null);

const keyPlaceholder = computed(() =>
  syncStore.summary.hasAnonKey ? '••••••••••••••••••••••' : 'eyJhbGciOi...',
);

const machineIdShort = computed(() => {
  const machineId = syncStore.info.machineId;
  if (!machineId) {
    return '-';
  }
  if (machineId.length <= 16) {
    return machineId;
  }
  return `${machineId.slice(0, 8)}...${machineId.slice(-4)}`;
});

const lastSyncLabel = computed(() => {
  if (!syncStore.info.lastSyncAt) {
    return 'ยังไม่เคย sync';
  }

  const date = new Date(syncStore.info.lastSyncAt);
  return new Intl.DateTimeFormat('th-TH', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
});

const statusBadgeLabel = computed(() => {
  switch (syncStore.status) {
    case 'pushing':
      return 'กำลัง push';
    case 'pulling':
      return 'กำลัง pull';
    case 'syncing':
      return 'กำลัง sync';
    case 'success':
      return 'พร้อมใช้งาน';
    case 'error':
      return 'มีข้อผิดพลาด';
    default:
      return syncStore.info.configured ? 'พร้อมตั้งค่าแล้ว' : 'ยังไม่ตั้งค่า';
  }
});

const statusBadgeClass = computed(() => {
  switch (syncStore.status) {
    case 'success':
      return 'badge-success';
    case 'error':
      return 'badge-tag-coral';
    case 'pushing':
    case 'pulling':
    case 'syncing':
      return 'badge-tag-purple';
    default:
      return syncStore.info.configured ? 'badge-success' : 'badge-tag-purple';
  }
});

onMounted(async () => {
  await syncStore.refreshAll();
  formUrl.value = syncStore.summary.supabaseUrl ?? '';
});

async function handleTestConnection() {
  actionError.value = null;
  testResult.value = null;
  testMessage.value = null;

  if (!(formUrl.value.trim() && formKey.value.trim())) {
    actionError.value = 'กรอก Project URL และ Anon Key ก่อนทดสอบการเชื่อมต่อ';
    return;
  }

  testing.value = true;
  try {
    const result = await syncStore.testConnection(formUrl.value.trim(), formKey.value.trim());
    testResult.value = result.ok;
    testMessage.value = result.message;
    if (!result.ok) {
      actionError.value = result.message;
    }
  } catch (error) {
    testResult.value = false;
    const msg = error instanceof Error ? error.message : String(error);
    testMessage.value = msg;
    actionError.value = msg || 'ทดสอบการเชื่อมต่อไม่สำเร็จ';
  } finally {
    testing.value = false;
  }
}

async function handleSaveConfig() {
  actionError.value = null;
  testMessage.value = null;

  if (!(formUrl.value.trim() && formKey.value.trim())) {
    actionError.value = 'กรอก Project URL และ Anon Key ก่อนบันทึก';
    return;
  }

  saving.value = true;
  try {
    await syncStore.saveConfig(formUrl.value.trim(), formKey.value.trim());
    formKey.value = '';
    testResult.value = true;
    testMessage.value = 'บันทึกการตั้งค่าสำเร็จ';
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    actionError.value = msg || 'บันทึกค่า Cloud Sync ไม่สำเร็จ';
  } finally {
    saving.value = false;
  }
}

async function handlePush() {
  actionError.value = null;
  try {
    await syncStore.push();
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    actionError.value = errorMsg || 'Push to cloud ไม่สำเร็จ';
  }
}

async function handlePull() {
  actionError.value = null;
  try {
    await syncStore.pull();
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    actionError.value = errorMsg || 'Pull from cloud ไม่สำเร็จ - ตรวจสอบว่ารัน SQL ใน Supabase แล้ว';
  }
}
</script>

<template>
  <div class="sync-layout">
    <section class="card-feature-pink-dark sync-hero">
      <div class="sync-title-row">
        <div>
          <div class="sync-kicker">
            <Cloud :size="16" />
            <span class="micro">Cloud Sync</span>
          </div>
          <h3 class="h4 sync-title">Supabase Backup & Restore</h3>
          <p class="body-sm sync-copy">
            SQLite ยังเป็นแหล่งข้อมูลหลักของเครื่องนี้เสมอ การ sync เป็นงานเสริมแบบ local-first และไม่บล็อกการใช้งานเดิม
          </p>
        </div>
        <span class="badge" :class="statusBadgeClass">{{ statusBadgeLabel }}</span>
      </div>

      <div class="sync-form-grid">
        <label class="form-field sync-field-wide">
          <span class="caption sync-label">Project URL</span>
          <input
            v-model="formUrl"
            class="input"
            placeholder="https://your-project.supabase.co"
            autocomplete="off"
          />
        </label>

        <label class="form-field sync-field-wide">
          <span class="caption sync-label">Anon Key</span>
          <div class="sync-secret-row">
            <input
              v-model="formKey"
              class="input"
              :type="showKey ? 'text' : 'password'"
              :placeholder="keyPlaceholder"
              autocomplete="off"
            />
            <button class="btn btn-secondary sync-icon-button" type="button" @click="showKey = !showKey">
              <Eye v-if="!showKey" :size="16" />
              <EyeOff v-else :size="16" />
            </button>
          </div>
          <span class="micro sync-hint">Anon key จะถูกเข้ารหัสก่อนบันทึกลง disk และ UI จะไม่อ่านค่าจริงกลับมาอีก</span>
        </label>
      </div>

      <div class="sync-action-row">
        <button class="btn btn-secondary" type="button" :disabled="testing" @click="handleTestConnection">
          <RefreshCw :size="16" :class="{ spinning: testing }" />
          {{ testing ? 'กำลังทดสอบ...' : 'Test Connection' }}
        </button>
        <button class="btn btn-primary" type="button" :disabled="saving" @click="handleSaveConfig">
          {{ saving ? 'กำลังบันทึก...' : 'Save Config' }}
        </button>
        <span v-if="testResult === true" class="badge badge-success">เชื่อมต่อสำเร็จ</span>
        <span v-else-if="testResult === false" class="badge badge-danger">เชื่อมต่อไม่สำเร็จ</span>
      </div>

      <p v-if="testMessage && testResult === true" class="micro sync-test-message success">
        {{ testMessage }}
      </p>
      <p v-else-if="testMessage && testResult === false" class="micro sync-test-message error">
        {{ testMessage }}
      </p>
    </section>

    <section class="card sync-status-card">
      <div class="sync-status-grid">
        <div class="sync-stat-block">
          <span class="caption sync-label">Pending upload</span>
          <strong class="sync-stat-value">{{ syncStore.info.pendingCount }}</strong>
          <span class="micro sync-hint">records ที่ยังไม่ได้ push หรือมีการแก้ไขหลัง sync ครั้งก่อน</span>
        </div>

        <div class="sync-stat-block">
          <span class="caption sync-label">Last sync</span>
          <strong class="body-md-medium">{{ lastSyncLabel }}</strong>
        </div>

        <div class="sync-stat-block">
          <span class="caption sync-label">Machine ID</span>
          <strong class="body-md-medium machine-id">{{ machineIdShort }}</strong>
          <span class="micro sync-hint">อ้างอิงเครื่องนี้สำหรับ encryption และ source tracking</span>
        </div>
      </div>

      <div class="sync-manual-row">
        <button class="btn btn-secondary" type="button" :disabled="!syncStore.info.configured || syncStore.status === 'pulling' || syncStore.status === 'syncing'" @click="handlePush">
          <Upload :size="16" />
          Push to Cloud
        </button>
        <button class="btn btn-secondary" type="button" :disabled="!syncStore.info.configured || syncStore.status === 'pushing' || syncStore.status === 'syncing'" @click="handlePull">
          <Download :size="16" />
          Pull from Cloud
        </button>
      </div>

      <div class="sync-toggle-row">
        <div>
          <p class="body-sm-medium">Auto-sync every 10 minutes</p>
          <p class="micro sync-hint">ระบบจะ push ก่อนแล้วค่อย pull เมื่อออนไลน์และตั้งค่า Supabase แล้ว</p>
        </div>
        <button
          type="button"
          class="sync-toggle"
          :class="{ active: syncStore.autoSyncEnabled }"
          role="switch"
          :aria-checked="syncStore.autoSyncEnabled"
          @click="syncStore.autoSyncEnabled = !syncStore.autoSyncEnabled"
        >
          <span class="sync-toggle-dot" />
          <span>{{ syncStore.autoSyncEnabled ? 'ON' : 'OFF' }}</span>
        </button>
      </div>

      <div v-if="actionError || syncStore.result?.errors.length" class="sync-feedback error-block">
        <p class="body-sm-medium">เกิดข้อผิดพลาด</p>
        <p v-if="actionError" class="body-sm">{{ actionError }}</p>
        <p v-for="error in syncStore.result?.errors ?? []" :key="error" class="body-sm">{{ error }}</p>
      </div>

      <div v-else-if="syncStore.result" class="sync-feedback success-block">
        <p class="body-sm-medium">ผลการ sync ล่าสุด</p>
        <p class="body-sm">
          Push {{ syncStore.result.pushed }} รายการ, Pull {{ syncStore.result.pulled }} รายการ,
          Conflict {{ syncStore.result.conflicts }} รายการ
        </p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.sync-layout {
  display: grid;
  gap: var(--spacing-lg);
}

.sync-hero {
  display: grid;
  gap: var(--spacing-xl);
  box-shadow: var(--elevation-2);
}

.sync-title-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--spacing-lg);
}

.sync-kicker {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  margin-bottom: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border-radius: var(--rounded-full);
  background: var(--color-canvas);
  color: var(--color-pink-600);
}

.sync-title {
  color: var(--color-ink-deep);
  margin-bottom: var(--spacing-xs);
}

.sync-copy,
.sync-label,
.sync-hint {
  color: var(--color-slate);
}

.sync-form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--spacing-md);
}

.sync-field-wide {
  grid-column: 1 / -1;
}

.sync-secret-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: var(--spacing-sm);
  align-items: center;
}

.sync-icon-button {
  min-height: var(--touch-target-min, 44px);
  min-width: var(--touch-target-min, 44px);
  padding: 0;
}

.sync-action-row,
.sync-manual-row,
.sync-toggle-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  flex-wrap: wrap;
}

.sync-test-message {
  margin-top: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--rounded-md);
  background: var(--color-surface-soft);
  white-space: pre-wrap;
  word-break: break-word;
}

.sync-test-message.success {
  background: var(--color-success-bg, var(--color-surface-soft));
  color: var(--color-success-accent, var(--color-ink));
}

.sync-test-message.error {
  background: var(--color-inr-high-bg);
  color: var(--color-inr-high);
}

.sync-status-card {
  display: grid;
  gap: var(--spacing-xl);
}

.sync-status-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--spacing-md);
}

.sync-stat-block {
  padding: var(--spacing-lg);
  border-radius: var(--rounded-xl);
  background: var(--color-surface-soft);
  border: 1px solid var(--color-hairline-soft);
  display: grid;
  gap: var(--spacing-xs);
}

.sync-stat-value {
  font-size: var(--typography-heading-3-size);
  font-weight: var(--typography-heading-3-weight);
  line-height: var(--typography-heading-3-line-height);
  color: var(--color-ink-deep);
}

.machine-id {
  letter-spacing: 0.04em;
}

.sync-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  border: 1px solid var(--color-hairline-strong);
  border-radius: var(--rounded-full);
  background: var(--color-canvas);
  color: var(--color-ink);
  min-height: var(--touch-target-min, 44px);
  padding: 0 var(--spacing-md);
  cursor: pointer;
  font: inherit;
}

.sync-toggle.active {
  background: var(--color-primary);
  color: var(--color-on-primary);
  border-color: var(--color-primary);
}

.sync-toggle-dot {
  width: var(--spacing-xs);
  height: var(--spacing-xs);
  border-radius: var(--rounded-full);
  background: currentColor;
}

.sync-feedback {
  padding: var(--spacing-lg);
  border-radius: var(--rounded-xl);
  display: grid;
  gap: var(--spacing-xs);
}

.success-block {
  background: var(--color-pink-100);
  color: var(--color-pink-600);
}

.error-block {
  background: var(--color-inr-high-bg);
  color: var(--color-inr-high);
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (max-width: 767px) {
  .sync-title-row,
  .sync-toggle-row {
    flex-direction: column;
    align-items: flex-start;
  }

  .sync-status-grid {
    grid-template-columns: 1fr;
  }
}
</style>