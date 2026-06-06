import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface SyncResult {
  pushed: number
  pulled: number
  conflicts: number
  errors: string[]
}

export interface SyncStatus {
  pendingCount: number
  lastSyncAt: string | null
  configured: boolean
  machineId: string
}

export interface SyncSummary {
  hasAnonKey: boolean
  supabaseUrl: string | null
}

export interface ConnectionTestResult {
  ok: boolean
  message: string
  statusCode: number | null
}

type SyncPhase = 'idle' | 'pushing' | 'pulling' | 'syncing' | 'success' | 'error'

export const useSyncStore = defineStore('sync', () => {
  const status = ref<SyncPhase>('idle')
  const result = ref<SyncResult | null>(null)
  const info = ref<SyncStatus>({
    pendingCount: 0,
    lastSyncAt: null,
    configured: false,
    machineId: '',
  })
  const summary = ref<SyncSummary>({
    hasAnonKey: false,
    supabaseUrl: null,
  })
  const autoSyncEnabled = ref(true)

  let intervalId: number | null = null

  async function loadSummary() {
    summary.value = await invoke<SyncSummary>('get_sync_summary')
  }

  async function refreshStatus() {
    info.value = await invoke<SyncStatus>('get_sync_status')
  }

  async function refreshAll() {
    await Promise.all([loadSummary(), refreshStatus()])
  }

  async function saveConfig(url: string, anonKey: string) {
    await invoke('save_supabase_config', { url, anonKey })
    summary.value = {
      hasAnonKey: true,
      supabaseUrl: url.trim(),
    }
    await refreshStatus()
  }

  async function testConnection(url: string, anonKey: string): Promise<ConnectionTestResult> {
    return invoke<ConnectionTestResult>('test_supabase_connection', { url, anonKey })
  }

  async function push() {
    status.value = 'pushing'
    try {
      result.value = await invoke<SyncResult>('push_to_supabase')
      status.value = result.value.errors.length > 0 ? 'error' : 'success'
    } catch (error) {
      status.value = 'error'
      throw error
    } finally {
      await refreshStatus()
    }
  }

  async function pull() {
    status.value = 'pulling'
    try {
      result.value = await invoke<SyncResult>('pull_from_supabase')
      status.value = result.value.errors.length > 0 ? 'error' : 'success'
    } catch (error) {
      status.value = 'error'
      throw error
    } finally {
      await refreshStatus()
    }
  }

  async function sync() {
    status.value = 'syncing'
    try {
      const pushResult = await invoke<SyncResult>('push_to_supabase')
      const pullResult = await invoke<SyncResult>('pull_from_supabase')
      result.value = {
        pushed: pushResult.pushed,
        pulled: pullResult.pulled,
        conflicts: pushResult.conflicts + pullResult.conflicts,
        errors: [...pushResult.errors, ...pullResult.errors],
      }
      status.value = result.value.errors.length > 0 ? 'error' : 'success'
    } catch (error) {
      status.value = 'error'
      throw error
    } finally {
      await refreshStatus()
    }
  }

  function startAutoSync(intervalMinutes = 10) {
    if (intervalId !== null) {
      return
    }

    intervalId = window.setInterval(() => {
      if (!autoSyncEnabled.value || !navigator.onLine || !info.value.configured) {
        return
      }

      if (status.value === 'pushing' || status.value === 'pulling' || status.value === 'syncing') {
        return
      }

      void sync().catch((error) => {
        console.error('Auto sync failed:', error)
      })
    }, intervalMinutes * 60 * 1000)
  }

  return {
    status,
    result,
    info,
    summary,
    autoSyncEnabled,
    loadSummary,
    refreshStatus,
    refreshAll,
    saveConfig,
    testConnection,
    push,
    pull,
    sync,
    startAutoSync,
  }
})