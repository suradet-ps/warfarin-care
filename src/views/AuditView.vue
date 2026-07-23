<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { ArrowLeft, Filter, Search } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { AUDIT_ACTIONS, type AuditLogEntry, type AuditLogFilter } from '#/types/audit.ts';
import { formatThaiDate } from '#/utils/clinic.ts';

const router = useRouter();
const loading = ref(false);
const error = ref<string | null>(null);
const entries = ref<AuditLogEntry[]>([]);

const filter = ref<AuditLogFilter>({
  hn: undefined,
  action: undefined,
  dateFrom: undefined,
  dateTo: undefined,
  page: 1,
  pageSize: 50,
});

const filterHn = ref('');
const filterAction = ref('');
const filterDateFrom = ref('');
const filterDateTo = ref('');

async function loadAuditLog() {
  loading.value = true;
  error.value = null;
  try {
    const f: AuditLogFilter = {
      page: filter.value.page,
      pageSize: filter.value.pageSize,
    };
    if (filterHn.value.trim()) f.hn = filterHn.value.trim();
    if (filterAction.value) f.action = filterAction.value;
    if (filterDateFrom.value) f.dateFrom = filterDateFrom.value;
    if (filterDateTo.value) f.dateTo = filterDateTo.value;
    entries.value = await invoke<AuditLogEntry[]>('get_audit_log', { filter: f });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function applyFilter() {
  filter.value.page = 1;
  void loadAuditLog();
}

function clearFilter() {
  filterHn.value = '';
  filterAction.value = '';
  filterDateFrom.value = '';
  filterDateTo.value = '';
  filter.value.page = 1;
  void loadAuditLog();
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
  if (action.includes('dose')) return 'action-dose';
  if (action.includes('visit')) return 'action-visit';
  if (action.includes('login') || action.includes('logout')) return 'action-auth';
  if (action.includes('status')) return 'action-status';
  if (action.includes('adverse')) return 'action-adverse';
  return 'action-default';
}

onMounted(() => {
  void loadAuditLog();
});
</script>

<template>
  <div class="audit-view">
    <div class="audit-header">
      <button class="btn btn-ghost" @click="router.back()" aria-label="กลับ">
        <ArrowLeft :size="18" />
      </button>
      <h2 class="h4">Audit Trail</h2>
    </div>

    <div class="filter-bar card">
      <div class="filter-fields">
        <label class="form-field">
          <span class="caption label">HN</span>
          <input class="input input-sm" type="text" v-model="filterHn" placeholder="กรอก HN..." />
        </label>
        <label class="form-field">
          <span class="caption label">ประเภท</span>
          <select class="input input-sm" v-model="filterAction">
            <option value="">ทั้งหมด</option>
            <option v-for="(label, key) in AUDIT_ACTIONS" :key="key" :value="key">{{ label }}</option>
          </select>
        </label>
        <label class="form-field">
          <span class="caption label">จากวันที่</span>
          <input class="input input-sm" type="date" v-model="filterDateFrom" />
        </label>
        <label class="form-field">
          <span class="caption label">ถึงวันที่</span>
          <input class="input input-sm" type="date" v-model="filterDateTo" />
        </label>
      </div>
      <div class="filter-actions">
        <button class="btn btn-primary btn-compact" @click="applyFilter">
          <Search :size="14" /> ค้นหา
        </button>
        <button class="btn btn-ghost btn-compact" @click="clearFilter">ล้างตัวกรอง</button>
      </div>
    </div>

    <div v-if="loading" class="loading-state">กำลังโหลด...</div>
    <div v-else-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">
      {{ error }}
    </div>
    <div v-else-if="entries.length === 0" class="empty-state body-sm" style="color: var(--color-slate)">
      ไม่มีข้อมูล Audit Trail
    </div>

    <div v-else class="audit-table-container">
      <table class="comparison-table">
        <thead>
          <tr class="comparison-row">
            <th>วันที่/เวลา</th>
            <th>ผู้ทำ</th>
            <th>ประเภท</th>
            <th>HN</th>
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
            <td>
              <RouterLink
                v-if="entry.hn"
                :to="`/patient/${entry.hn}`"
                class="hn-link"
              >
                {{ entry.hn }}
              </RouterLink>
              <span v-else class="caption" style="color: var(--color-slate)">-</span>
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
.audit-view {
  padding: var(--spacing-xl);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
  max-width: 1200px;
}

.audit-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.filter-bar {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  padding: var(--spacing-lg);
}

.filter-fields {
  display: flex;
  gap: var(--spacing-md);
  flex-wrap: wrap;
}

.filter-fields .form-field {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xxs);
  min-width: 150px;
}

.input-sm {
  height: 36px;
  padding-inline: var(--spacing-sm);
  font-size: var(--typography-body-sm-size);
}

.filter-actions {
  display: flex;
  gap: var(--spacing-sm);
}

.btn-compact {
  padding-inline: var(--spacing-md);
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

.hn-link {
  color: var(--color-ink);
  text-decoration: underline;
  cursor: pointer;
}

.hn-link:hover {
  color: var(--color-pink-600);
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

.loading-state,
.empty-state {
  padding: var(--spacing-xxl);
  text-align: center;
}
</style>
