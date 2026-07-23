<script setup lang="ts">
import { Plus, Search, Trash2, X } from 'lucide-vue-next';
import { onMounted, ref } from 'vue';
import SyncPanel from '#/components/settings/SyncPanel.vue';
import { useSettingsStore } from '#/stores/settings.ts';

const store = useSettingsStore();
const testResult = ref<boolean | null>(null);
const testing = ref(false);
const saving = ref(false);
const saveResult = ref<'success' | 'error' | null>(null);
const saveError = ref<string | null>(null);

const activeSection = ref<'connection' | 'hospital' | 'interactions' | 'sync'>('connection');

const sections = [
  { key: 'connection', label: 'การเชื่อมต่อ' },
  { key: 'hospital', label: 'ข้อมูลโรงพยาบาล' },
  { key: 'sync', label: 'Cloud Sync' },
  { key: 'interactions', label: 'Drug interaction' },
] as const;

onMounted(() => {
  void store.loadMysqlConfig();
  void store.loadSettings();
  void store.loadDrugInteractions();
});

async function handleTestConnection() {
  testing.value = true;
  saveResult.value = null;
  saveError.value = null;
  const result = await store.testConnection();
  testResult.value = result;
  testing.value = false;
}

async function handleSaveConnection() {
  saving.value = true;
  testResult.value = null;
  saveResult.value = null;
  saveError.value = null;
  try {
    await store.saveMysqlConfig();
    saveResult.value = 'success';
  } catch (e) {
    saveResult.value = 'error';
    saveError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

const interactionModalOpen = ref(false);
const searchingDrug = ref(false);
const searchResults = ref<{ icode: string; name: string; strength: string }[]>([]);
const selectedDrug = ref<{ icode: string; name: string; strength: string } | null>(null);
const interactionType = ref('increase');
const severity = ref('moderate');
const clinicalEffect = ref('');
const management = ref('');
const evidenceLevel = ref('');
const searchKeyword = ref('');

async function onSearchKeyword() {
  if (!searchKeyword.value.trim()) {
    return;
  }

  if (!store.isConnected) {
    alert('กรุณาเชื่อมต่อ HOSxP MySQL ก่อนค้นหายา\n(ไปที่แท็บ "การเชื่อมต่อ" และกด "ทดสอบการเชื่อมต่อ")');
    return;
  }

  searchingDrug.value = true;
  try {
    searchResults.value = await store.searchHosxpDrugs(searchKeyword.value.trim());
  } catch {
    searchResults.value = [];
  } finally {
    searchingDrug.value = false;
  }
}

function selectDrug(drug: { icode: string; name: string; strength: string }) {
  selectedDrug.value = drug;
  searchResults.value = [];
  searchKeyword.value = `${drug.name} ${drug.strength}`.trim();
}

async function saveDrugInteraction() {
  if (!selectedDrug.value) {
    return;
  }
  await store.addDrugInteraction({
    icode: selectedDrug.value.icode,
    drugName: selectedDrug.value.name,
    strength: selectedDrug.value.strength || null,
    interactionType: interactionType.value,
    severity: severity.value,
    clinicalEffect: clinicalEffect.value.trim() || null,
    management: management.value.trim() || null,
    evidenceLevel: evidenceLevel.value.trim() || null,
  });
  interactionModalOpen.value = false;
  selectedDrug.value = null;
  searchKeyword.value = '';
  interactionType.value = 'increase';
  severity.value = 'moderate';
  clinicalEffect.value = '';
  management.value = '';
  evidenceLevel.value = '';
}

async function handleDeleteInteraction(id: number) {
  if (confirm('ต้องการลบรายการนี้?')) {
    await store.deleteDrugInteraction(id);
  }
}

function severityBadgeClass(severity: string): string {
  switch (severity) {
    case 'contraindicated': return 'badge-danger';
    case 'major': return 'badge-danger';
    case 'moderate': return 'badge-warning';
    case 'minor': return 'badge-info';
    default: return 'badge-warning';
  }
}

function severityLabel(severity: string): string {
  switch (severity) {
    case 'contraindicated': return 'ห้ามใช้ร่วม';
    case 'major': return 'หลีกเลี่ยง';
    case 'moderate': return 'ระวัง';
    case 'minor': return 'ทราบ';
    default: return severity;
  }
}
</script>

<template>
  <div class="settings-view">
    <div class="section-tabs">
      <button
        v-for="section in sections"
        :key="section.key"
        :class="['section-tab', { active: activeSection === section.key }]"
        @click="activeSection = section.key"
      >
        {{ section.label }}
      </button>
    </div>

    <div v-if="activeSection === 'connection'" class="settings-section card">
      <h3 class="h4" style="margin-bottom: var(--spacing-xl)">การเชื่อมต่อ HOSxP MySQL</h3>
      <div class="form-grid">
        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">Host</span>
          <input class="input" v-model="store.mysqlConfig.host" placeholder="localhost" />
        </label>
        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">Port</span>
          <input class="input" type="number" v-model.number="store.mysqlConfig.port" />
        </label>
        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">Database</span>
          <input class="input" v-model="store.mysqlConfig.database" />
        </label>
        <label class="form-field">
          <span class="caption" style="color:var(--color-slate)">Username</span>
          <input class="input" v-model="store.mysqlConfig.username" />
        </label>
        <label class="form-field" style="grid-column: 1 / -1">
          <span class="caption" style="color:var(--color-slate)">Password</span>
          <input
            class="input"
            type="password"
            v-model="store.mysqlConfig.password"
            :placeholder="store.hasStoredConfig ? '•••••••• (พิมพ์ใหม่เพื่อเปลี่ยน)' : 'กรอกรหัสผ่าน'"
            autocomplete="off"
          />
        </label>
      </div>
      <div class="settings-actions">
        <button class="btn btn-secondary" @click="handleTestConnection" :disabled="testing || saving">
          {{ testing ? 'กำลังทดสอบ...' : 'ทดสอบการเชื่อมต่อ' }}
        </button>
        <button class="btn btn-primary" @click="handleSaveConnection" :disabled="testing || saving">
          {{ saving ? 'กำลังบันทึก...' : 'บันทึกการเชื่อมต่อ' }}
        </button>
        <span v-if="testResult === true" class="badge badge-success">✓ เชื่อมต่อสำเร็จ (บันทึกแล้ว)</span>
        <span v-else-if="testResult === false" class="badge badge-danger">✗ เชื่อมต่อไม่ได้</span>
        <span v-if="saveResult === 'success'" class="badge badge-success">✓ บันทึกการเชื่อมต่อแล้ว</span>
        <span v-else-if="saveResult === 'error'" class="badge badge-danger">✗ บันทึกไม่สำเร็จ</span>
      </div>
      <p v-if="saveError" class="caption" style="color: var(--color-brand-red); margin-top: var(--spacing-xs)">
        {{ saveError }}
      </p>
    </div>

    <div v-else-if="activeSection === 'hospital'" class="settings-section card">
      <h3 class="h4" style="margin-bottom: var(--spacing-xl)">ข้อมูลโรงพยาบาล</h3>
      <label class="form-field">
        <span class="caption" style="color:var(--color-slate)">ชื่อโรงพยาบาล</span>
        <input class="input" v-model="store.hospitalName" />
      </label>
    </div>

    <div v-else-if="activeSection === 'sync'" class="settings-section">
      <SyncPanel />
    </div>

    <div v-else-if="activeSection === 'interactions'" class="settings-section">
      <div class="section-header">
        <h3 class="h4">Drug interaction ที่มีผลต่อ Warfarin</h3>
        <button class="btn btn-primary" @click="interactionModalOpen = true">
          <Plus :size="16" /> เพิ่มยา
        </button>
      </div>

      <div v-if="store.drugInteractions.length === 0" class="card" style="padding: var(--spacing-lg); text-align: center;">
        <p class="body-sm" style="color: var(--color-slate)">
          ยังไม่มีการตั้งค่า Drug interaction คลิก "เพิ่มยา" เพื่อเพิ่มรายการ
        </p>
      </div>

      <div v-else class="table-card">
        <table class="comparison-table">
          <thead>
            <tr class="comparison-row">
              <th>ICode</th>
              <th>ชื่อยา</th>
              <th>ความแรง</th>
              <th>ผลต่อ Warfarin</th>
              <th>ความรุนแรง</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="drug in store.drugInteractions" :key="drug.id" class="comparison-row">
              <td class="font-mono">{{ drug.icode }}</td>
              <td>{{ drug.drugName }}</td>
              <td>{{ drug.strength || '-' }}</td>
              <td>
                <span :class="['badge', drug.interactionType === 'increase' ? 'badge-danger' : 'badge-warning']">
                  {{ drug.interactionType === 'increase' ? 'เพิ่มฤทธิ์' : 'ลดฤทธิ์' }}
                </span>
              </td>
              <td>
                <span :class="['badge', severityBadgeClass(drug.severity)]">
                  {{ severityLabel(drug.severity) }}
                </span>
              </td>
              <td class="text-right">
                <button class="btn btn-ghost btn-icon" @click="handleDeleteInteraction(drug.id)">
                  <Trash2 :size="16" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <Teleport to="body">
      <div v-if="interactionModalOpen" class="modal-overlay" @click.self="interactionModalOpen = false">
        <div class="modal-card">
          <div class="modal-header">
            <h3 class="h4">เพิ่ม Drug interaction</h3>
            <button class="btn btn-ghost btn-icon" @click="interactionModalOpen = false">
              <X :size="20" />
            </button>
          </div>

          <div class="modal-body">
            <label class="form-field">
              <span class="caption" style="color:var(--color-slate)">ค้นหายาใน HOSxP</span>
              <div class="search-input-group">
                <input
                  class="input"
                  v-model="searchKeyword"
                  placeholder="พิมพ์ชื่อยาหรือรหัสยา..."
                  @keyup.enter="onSearchKeyword"
                />
                <button class="btn btn-secondary" @click="onSearchKeyword" :disabled="searchingDrug">
                  <Search :size="16" />
                </button>
              </div>
            </label>

            <div v-if="searchResults.length > 0" class="search-results">
              <button
                v-for="drug in searchResults"
                :key="drug.icode"
                class="search-result-item"
                @click="selectDrug(drug)"
              >
                <span class="drug-name">{{ drug.name }}</span>
                <span class="drug-strength">{{ drug.strength }}</span>
                <span class="drug-code">{{ drug.icode }}</span>
              </button>
            </div>

            <div v-if="selectedDrug" class="selected-drug">
              <div class="selected-drug-info">
                <span class="caption">ยาที่เลือก</span>
                <span class="drug-name">{{ selectedDrug.name }} {{ selectedDrug.strength }}</span>
                <span class="drug-code">{{ selectedDrug.icode }}</span>
              </div>

              <label class="form-field">
                <span class="caption" style="color:var(--color-slate)">ประเภทปฏิกิริยา</span>
                <select class="input" v-model="interactionType">
                  <option value="increase">เพิ่มฤทธิ์ Warfarin (Increase)</option>
                  <option value="decrease">ลดฤทธิ์ Warfarin (Decrease)</option>
                </select>
              </label>

              <label class="form-field">
                <span class="caption" style="color:var(--color-slate)">ความรุนแรง (Severity)</span>
                <select class="input" v-model="severity">
                  <option value="minor">Minor - ทราบ</option>
                  <option value="moderate">Moderate - ระวัง</option>
                  <option value="major">Major - หลีกเลี่ยง</option>
                  <option value="contraindicated">Contraindicated - ห้ามใช้ร่วม</option>
                </select>
              </label>

              <label class="form-field" style="grid-column: 1 / -1">
                <span class="caption" style="color:var(--color-slate)">ผลทางคลินิก (Clinical Effect)</span>
                <input class="input" v-model="clinicalEffect" placeholder="เช่น เพิ่มฤทธิ์ผ่าน CYP2C9" />
              </label>

              <label class="form-field" style="grid-column: 1 / -1">
                <span class="caption" style="color:var(--color-slate)">การจัดการ (Management)</span>
                <input class="input" v-model="management" placeholder="เช่น ลดขนาดยา 25-50%, ติดตาม INR" />
              </label>

              <label class="form-field">
                <span class="caption" style="color:var(--color-slate)">ระดับหลักฐาน (Evidence)</span>
                <select class="input" v-model="evidenceLevel">
                  <option value="">ไม่ระบุ</option>
                  <option value="A">A - หลักฐานแข็ง</option>
                  <option value="B">B - หลักฐานปานกลาง</option>
                  <option value="C">C - หลักฐานจำกัด</option>
                </select>
              </label>
            </div>
          </div>

          <div class="modal-footer">
            <button class="btn btn-secondary" @click="interactionModalOpen = false">ยกเลิก</button>
            <button
              class="btn btn-primary"
              @click="saveDrugInteraction"
              :disabled="!selectedDrug"
            >
              บันทึก
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.settings-view { display: flex; flex-direction: column; gap: var(--spacing-lg); max-width: 800px; }
.section-tabs { display: flex; gap: var(--spacing-xs); border-bottom: 1px solid var(--color-hairline); padding-bottom: var(--spacing-xs); }
.section-tab {
  padding: var(--spacing-sm) var(--spacing-md);
  cursor: pointer;
  border: none;
  background: none;
  font-size: var(--typography-body-sm-size);
  color: var(--color-slate);
  border-bottom: 2px solid transparent;
  transition: color 150ms;
}
.section-tab:hover { background: var(--color-surface-soft); }
.section-tab.active { background: var(--color-primary); color: var(--color-on-primary); }

.settings-section { font-size: var(--typography-body-sm-size); }
.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-md); margin-bottom: var(--spacing-xl); }
.form-field { display: flex; flex-direction: column; gap: var(--spacing-xs); }
.settings-actions { display: flex; align-items: center; gap: var(--spacing-md); }
.section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--spacing-lg); }
.table-card { border: 1px solid var(--color-hairline); border-radius: var(--rounded-lg); overflow: hidden; }
.comparison-table { width: 100%; border-collapse: collapse; }
.font-mono { font-family: monospace; font-size: var(--typography-micro-size); }
.search-input-group { display: flex; gap: var(--spacing-xs); }
.search-input-group .input { flex: 1; }
.search-results { display: flex; flex-direction: column; gap: var(--spacing-xs); max-height: 200px; overflow-y: auto; margin-top: var(--spacing-sm); }
.search-result-item { display: flex; align-items: center; gap: var(--spacing-md); padding: var(--spacing-sm) var(--spacing-md); border: 1px solid var(--color-hairline); border-radius: var(--rounded-md); cursor: pointer; text-align: left; background: var(--color-surface); transition: background 150ms; }
.search-result-item:hover { background: var(--color-surface-soft); }
.selected-drug { margin-top: var(--spacing-md); padding: var(--spacing-md); background: var(--color-surface); border-radius: var(--rounded-md); display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-md); }
.selected-drug-info { display: flex; flex-direction: column; gap: 2px; margin-bottom: var(--spacing-md); }
.comparison-row th { padding: var(--spacing-sm) var(--spacing-md); text-align: left; font-weight: 600; font-size: var(--typography-micro-uppercase-size); color: var(--color-slate); background: var(--color-surface); }
.text-right { text-align: right; }
.btn-icon { padding: var(--spacing-xs); }
</style>

<style>
.modal-overlay {
  position: fixed; inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex; align-items: center; justify-content: center;
  z-index: 100;
}
.modal-card {
  background: var(--color-canvas);
  border-radius: var(--rounded-xl);
  width: 100%; max-width: 480px;
  max-height: 90vh;
  overflow: hidden;
  display: flex; flex-direction: column;
  box-shadow: var(--elevation-4);
}
.modal-header { display: flex; justify-content: space-between; align-items: center; padding: var(--spacing-lg); border-bottom: 1px solid var(--color-hairline); }
.modal-body { padding: var(--spacing-lg); overflow-y: auto; flex: 1; }
.modal-footer { display: flex; justify-content: flex-end; gap: var(--spacing-md); padding: var(--spacing-lg); border-top: 1px solid var(--color-hairline); }
</style>