import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface DrugInteraction {
  id: number
  icode: string
  drugName: string
  strength: string | null
  interactionType: string
  createdAt: string
  updatedAt: string
}

export interface HosxpDrugItem {
  icode: string
  name: string
  strength: string
  units: string
}

export interface MysqlConfig {
  host: string
  port: number
  database: string
  username: string
  password: string
}

export interface MysqlConfigStatus {
  hasConfig: boolean
  host: string
  port: number
  database: string
  username: string
}

export const useSettingsStore = defineStore('settings', () => {
  const mysqlConfig = ref<MysqlConfig>({
    host: 'localhost',
    port: 3306,
    database: 'hosxp',
    username: '',
    password: '',
  })
  const hasStoredConfig = ref(false)
  const hospitalName = ref('Warfarin Care')
  const staffList = ref<string[]>([])
  const isConnected = ref(false)
  const drugInteractions = ref<DrugInteraction[]>([])

  async function loadMysqlConfig() {
    try {
      // Fetch non-secret metadata only. The password is never round-tripped
      // through the frontend; the user re-enters it to update.
      const status = await invoke<MysqlConfigStatus>('get_mysql_config_status')
      if (status.hasConfig) {
        mysqlConfig.value = {
          host: status.host,
          port: status.port,
          database: status.database,
          username: status.username,
          password: '',
        }
        hasStoredConfig.value = true
        isConnected.value = true
      }
    } catch (e) {
      console.error('Failed to load MySQL config status:', e)
    }
  }

  async function testConnection() {
    try {
      isConnected.value = await invoke<boolean>('test_mysql_connection', {
        config: mysqlConfig.value,
      })
      if (isConnected.value) {
        hasStoredConfig.value = true
        // Clear the password from the in-memory store once it has been
        // persisted encrypted on the backend. Don't keep it in Vue state.
        mysqlConfig.value.password = ''
      }
      return isConnected.value
    } catch (e) {
      console.error('Connection test failed:', e)
      isConnected.value = false
      return false
    }
  }

  async function saveMysqlConfig() {
    // Persists the current form values via the backend. The backend merges
    // in the stored password if the in-memory password is empty, so users
    // can save edits without re-typing the password.
    await invoke('save_mysql_config', { config: mysqlConfig.value })
    hasStoredConfig.value = true
    mysqlConfig.value.password = ''
  }

  async function loadSettings() {
    try {
      const settings = await invoke<Record<string, string>>('get_settings')
      if (settings.hospital_name) hospitalName.value = settings.hospital_name
      if (settings.staff_list) staffList.value = JSON.parse(settings.staff_list)
    } catch (e) {
      console.error('Failed to load settings:', e)
    }
  }

  async function loadDrugInteractions() {
    try {
      drugInteractions.value = await invoke<DrugInteraction[]>('get_all_drug_interactions')
    } catch (e) {
      console.error('Failed to load drug interactions:', e)
    }
  }

  async function addDrugInteraction(input: {
    icode: string
    drugName: string
    strength: string | null
    interactionType: string
  }) {
    const id = await invoke<number>('add_drug_interaction', { input })
    await loadDrugInteractions()
    return id
  }

  async function deleteDrugInteraction(id: number) {
    await invoke('delete_drug_interaction', { id })
    await loadDrugInteractions()
  }

  async function searchHosxpDrugs(keyword: string): Promise<HosxpDrugItem[]> {
    return invoke<HosxpDrugItem[]>('search_hosxp_drugs', {
      mysqlConfig: mysqlConfig.value,
      keyword,
    })
  }

  return {
    mysqlConfig,
    hasStoredConfig,
    hospitalName,
    staffList,
    isConnected,
    drugInteractions,
    loadMysqlConfig,
    testConnection,
    saveMysqlConfig,
    loadSettings,
    loadDrugInteractions,
    addDrugInteraction,
    deleteDrugInteraction,
    searchHosxpDrugs,
  }
})