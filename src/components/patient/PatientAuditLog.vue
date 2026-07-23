<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMounted, ref } from 'vue';
import { AUDIT_ACTIONS, type AuditLogEntry } from '#/types/audit.ts';

const props = defineProps<{ hn: string }>();

const loading = ref(false);
const error = ref<string | null>(null);
const entries = ref<AuditLogEntry[]>([]);

async function loadPatientAudit() {
  loading.value = true;
  error.value = null;
  try {
    entries.value = await invoke<AuditLogEntry[]>('get_patient_audit_log', {
      hn: props.hn,
      limit: 100,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts);
    return d.toLocaleString('th-TH', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return ts;
  }
}

function actionLabel(action: string): string {
  return AUDIT_ACTIONS[action] ?? action;
}

function actionClass(action: string): string {
  if (action.includes('dose')) {
    return 'action-dose';
  }
  if (action.includes('visit')) {
    return 'action-visit';
  }
  if (action.includes('login') || action.includes('logout')) {
    return 'action-auth';
  }
  if (action.includes('status')) {
    return 'action-status';
  }
  if (action.includes('adverse')) {
    return 'action-adverse';
  }
  return 'action-default';
}

onMounted(() => {
  void loadPatientAudit();
});
</script>

<template>
  <div class="patient-audit">
    <div v-if="loading" class="body-sm" style="color: var(--color-slate)">กำลังโหลด...</div>

    <div v-else-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">
      {{ error }}
    </div>

    <div v-else-if="entries.length === 0" class="body-sm" style="color: var(--color-slate); padding: var(--spacing-lg); text-align: center">
      ไม่มีประวัติการกระทำสำหรับผู้ป่วยรายนี้
    </div>

    <div v-else class="audit-table-container">
      <table class="comparison-table">
        <thead>
          <tr class="comparison-row">
            <th>วันที่/เวลา</th>
            <th>ผู้ทำ</th>
            <th>ประเภท</th>
            <th>รายละเอียด</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="entry in entries" :key="entry.id" class="comparison-row">
            <td class="timestamp-cell">{{ formatTimestamp(entry.timestamp) }}</td>
            <td>{{ entry.actor }}</td>
            <td>
              <span :class="['action-badge', actionClass(entry.action)]">
                {{ actionLabel(entry.action) }}
              </span>
            </td>
            <td class="detail-cell">
              <template v-if="entry.oldValue && entry.newValue">
                <span class="old-value">{{ entry.oldValue }}</span>
                <span class="arrow">→</span>
                <span class="new-value">{{ entry.newValue }}</span>
              </template>
              <span v-else-if="entry.newValue" class="new-value">{{ entry.newValue }}</span>
              <span v-else-if="entry.detail" class="caption">{{ entry.detail }}</span>
              <span v-else class="caption" style="color: var(--color-slate)">-</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.patient-audit {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.audit-table-container {
  overflow-x: auto;
}

.comparison-table {
  width: 100%;
  border-collapse: collapse;
}

.comparison-row {
  border-bottom: 1px solid var(--color-hairline-soft);
}

.comparison-row th {
  padding: var(--spacing-sm) var(--spacing-md);
  text-align: left;
  font-weight: var(--typography-caption-bold-weight);
  font-size: var(--typography-micro-uppercase-size);
  color: var(--color-slate);
  background: var(--color-surface);
}

.comparison-row td {
  padding: var(--spacing-sm) var(--spacing-md);
  font-size: var(--typography-body-sm-size);
}

.timestamp-cell {
  white-space: nowrap;
}

.detail-cell {
  max-width: 300px;
}

.old-value {
  color: var(--color-slate);
  text-decoration: line-through;
}

.arrow {
  margin: 0 var(--spacing-xxs);
  color: var(--color-slate);
}

.new-value {
  font-weight: var(--typography-body-sm-medium-weight);
}

.action-badge {
  display: inline-block;
  font-size: var(--typography-micro-size);
  font-weight: var(--typography-caption-bold-weight);
  padding: 1px var(--spacing-xs);
  border-radius: var(--rounded-full);
  white-space: nowrap;
}

.action-badge.action-dose {
  background: var(--color-pink-100);
  color: var(--color-pink-600);
}

.action-badge.action-visit {
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
}

.action-badge.action-auth {
  background: rgba(100, 116, 139, 0.1);
  color: var(--color-slate);
}

.action-badge.action-status {
  background: rgba(245, 158, 11, 0.1);
  color: #d97706;
}

.action-badge.action-adverse {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}

.action-badge.action-default {
  background: var(--color-pink-50);
  color: var(--color-ink);
}
</style>
