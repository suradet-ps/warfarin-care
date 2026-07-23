<script setup lang="ts">
import {
  AlertTriangle,
  Pill,
  Plus,
  Search,
  ShieldAlert,
  ShieldCheck,
  ShieldX,
  Trash2,
  X,
} from 'lucide-vue-next';
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
const searchResults = ref<{ icode: string; name: string; strength: string; units: string }[]>([]);
const selectedDrug = ref<{ icode: string; name: string; strength: string; units: string } | null>(
  null,
);
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

function selectDrug(drug: { icode: string; name: string; strength: string; units: string }) {
  selectedDrug.value = drug;
  searchResults.value = [];
  searchKeyword.value = `${drug.name} ${drug.strength}`.trim();
}

function openAddModal() {
  interactionModalOpen.value = true;
  selectedDrug.value = null;
  searchKeyword.value = '';
  searchResults.value = [];
  interactionType.value = 'increase';
  severity.value = 'moderate';
  clinicalEffect.value = '';
  management.value = '';
  evidenceLevel.value = '';
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

function severityConfig(severity: string) {
  switch (severity) {
    case 'contraindicated':
      return { class: 'severity-contraindicated', label: 'ห้ามใช้ร่วม', icon: ShieldX };
    case 'major':
      return { class: 'severity-major', label: 'หลีกเลี่ยง', icon: ShieldAlert };
    case 'moderate':
      return { class: 'severity-moderate', label: 'ระวัง', icon: AlertTriangle };
    case 'minor':
      return { class: 'severity-minor', label: 'ทราบ', icon: ShieldCheck };
    default:
      return { class: 'severity-minor', label: severity, icon: ShieldCheck };
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

    <!-- Connection -->
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

    <!-- Hospital -->
    <div v-else-if="activeSection === 'hospital'" class="settings-section card">
      <h3 class="h4" style="margin-bottom: var(--spacing-xl)">ข้อมูลโรงพยาบาล</h3>
      <label class="form-field">
        <span class="caption" style="color:var(--color-slate)">ชื่อโรงพยาบาล</span>
        <input class="input" v-model="store.hospitalName" />
      </label>
    </div>

    <!-- Sync -->
    <div v-else-if="activeSection === 'sync'" class="settings-section">
      <SyncPanel />
    </div>

    <!-- Drug Interactions -->
    <div v-else-if="activeSection === 'interactions'" class="settings-section">
      <div class="interaction-header">
        <div class="interaction-header-text">
          <h3 class="h4">Drug interaction ที่มีผลต่อ Warfarin</h3>
          <p class="caption" style="color: var(--color-slate)">
            กำหนดยายาที่มีปฏิกิริยากับ Warfarin จากฐานข้อมูล HOSxP
          </p>
        </div>
        <button class="btn btn-primary" @click="openAddModal">
          <Plus :size="16" /> เพิ่มยา
        </button>
      </div>

      <!-- Empty state -->
      <div v-if="store.drugInteractions.length === 0" class="empty-state card">
        <div class="empty-state-icon">
          <Pill :size="32" />
        </div>
        <p class="body-sm" style="color: var(--color-slate); margin: 0">
          ยังไม่มีการตั้งค่า Drug interaction
        </p>
        <p class="caption" style="color: var(--color-stone); margin: 0">
          คลิก "เพิ่มยา" เพื่อค้นหายาจาก HOSxP และเพิ่มปฏิกิริยา
        </p>
      </div>

      <!-- Interaction cards -->
      <div v-else class="interaction-list">
        <div
          v-for="drug in store.drugInteractions"
          :key="drug.id"
          class="interaction-card"
        >
          <div class="interaction-card-left">
            <div :class="['severity-indicator', severityConfig(drug.severity).class]">
              <component :is="severityConfig(drug.severity).icon" :size="16" />
            </div>
            <div class="interaction-card-content">
              <div class="interaction-card-title">
                <span class="drug-name">{{ drug.drugName }}</span>
                <span v-if="drug.strength" class="drug-strength">{{ drug.strength }}</span>
                <span class="drug-icode">{{ drug.icode }}</span>
              </div>
              <div class="interaction-card-badges">
                <span :class="['badge', drug.interactionType === 'increase' ? 'badge-danger' : 'badge-warning']">
                  {{ drug.interactionType === 'increase' ? 'เพิ่มฤทธิ์' : 'ลดฤทธิ์' }}
                </span>
                <span :class="['badge', severityConfig(drug.severity).class]">
                  {{ severityConfig(drug.severity).label }}
                </span>
                <span v-if="drug.evidenceLevel" class="badge badge-info">
                  Evidence {{ drug.evidenceLevel }}
                </span>
              </div>
              <p v-if="drug.clinicalEffect" class="interaction-card-detail">
                {{ drug.clinicalEffect }}
              </p>
              <p v-if="drug.management" class="interaction-card-detail interaction-card-management">
                แนะนำ: {{ drug.management }}
              </p>
            </div>
          </div>
          <button class="btn btn-ghost btn-icon btn-delete" @click="handleDeleteInteraction(drug.id)">
            <Trash2 :size="16" />
          </button>
        </div>
      </div>
    </div>

    <!-- Add Drug Interaction Modal -->
    <Teleport to="body">
      <div v-if="interactionModalOpen" class="modal-overlay" @click.self="interactionModalOpen = false">
        <div class="modal-card modal-card-lg">
          <div class="modal-header">
            <div>
              <h3 class="h4">เพิ่ม Drug interaction</h3>
              <p class="caption" style="color: var(--color-slate); margin-top: 2px">
                ค้นหายาจาก HOSxP และกำหนดปฏิกิริยา
              </p>
            </div>
            <button class="btn btn-ghost btn-icon" @click="interactionModalOpen = false">
              <X :size="20" />
            </button>
          </div>

          <div class="modal-body">
            <!-- Search section -->
            <div class="form-section">
              <label class="form-field">
                <span class="form-label">ค้นหายาใน HOSxP</span>
                <div class="search-input-group">
                  <div class="search-input-wrapper">
                    <Search :size="16" class="search-icon" />
                    <input
                      class="input search-input"
                      v-model="searchKeyword"
                      placeholder="พิมพ์ชื่อยาหรือรหัสยา..."
                      @keyup.enter="onSearchKeyword"
                    />
                  </div>
                  <button class="btn btn-primary" @click="onSearchKeyword" :disabled="searchingDrug">
                    {{ searchingDrug ? 'ค้นหา...' : 'ค้นหา' }}
                  </button>
                </div>
              </label>

              <!-- Search results -->
              <div v-if="searchResults.length > 0" class="search-results">
                <div class="search-results-header">
                  <span class="caption-bold">พบ {{ searchResults.length }} รายการ</span>
                </div>
                <div class="search-results-list">
                  <button
                    v-for="drug in searchResults"
                    :key="drug.icode"
                    class="search-result-item"
                    @click="selectDrug(drug)"
                  >
                    <div class="search-result-info">
                      <span class="search-result-name">{{ drug.name }}</span>
                      <span class="search-result-meta">
                        <span v-if="drug.strength">{{ drug.strength }}</span>
                        <span v-if="drug.strength && drug.units"> · </span>
                        <span v-if="drug.units">{{ drug.units }}</span>
                      </span>
                    </div>
                    <span class="search-result-code">{{ drug.icode }}</span>
                  </button>
                </div>
              </div>
            </div>

            <!-- Selected drug + config -->
            <div v-if="selectedDrug" class="form-section selected-drug-section">
              <div class="selected-drug-header">
                <Pill :size="16" style="color: var(--color-primary)" />
                <span class="form-label">ยาที่เลือก</span>
              </div>
              <div class="selected-drug-card">
                <div class="selected-drug-info">
                  <span class="selected-drug-name">{{ selectedDrug.name }}</span>
                  <span class="selected-drug-meta">
                    <span v-if="selectedDrug.strength">{{ selectedDrug.strength }}</span>
                    <span class="selected-drug-code">{{ selectedDrug.icode }}</span>
                  </span>
                </div>
              </div>

              <div class="config-grid">
                <label class="form-field">
                  <span class="form-label">ประเภทปฏิกิริยา</span>
                  <select class="input" v-model="interactionType">
                    <option value="increase">เพิ่มฤทธิ์ Warfarin (Increase)</option>
                    <option value="decrease">ลดฤทธิ์ Warfarin (Decrease)</option>
                  </select>
                </label>

                <label class="form-field">
                  <span class="form-label">ความรุนแรง (Severity)</span>
                  <select class="input" v-model="severity">
                    <option value="minor">Minor - ทราบ</option>
                    <option value="moderate">Moderate - ระวัง</option>
                    <option value="major">Major - หลีกเลี่ยง</option>
                    <option value="contraindicated">Contraindicated - ห้ามใช้ร่วม</option>
                  </select>
                </label>

                <label class="form-field full-width">
                  <span class="form-label">ผลทางคลินิก (Clinical Effect)</span>
                  <textarea
                    class="input textarea"
                    v-model="clinicalEffect"
                    placeholder="เช่น เพิ่มฤทธิ์ผ่านการยับยั้ง CYP2C9, ทำให้ INR สูงขึ้น"
                    rows="2"
                  />
                </label>

                <label class="form-field full-width">
                  <span class="form-label">การจัดการ (Management)</span>
                  <textarea
                    class="input textarea"
                    v-model="management"
                    placeholder="เช่น ลดขนาดยา 25-50%, ติดตาม INR ทุกสัปดาห์"
                    rows="2"
                  />
                </label>

                <label class="form-field">
                  <span class="form-label">ระดับหลักฐาน (Evidence)</span>
                  <select class="input" v-model="evidenceLevel">
                    <option value="">ไม่ระบุ</option>
                    <option value="A">A - หลักฐานแข็ง</option>
                    <option value="B">B - หลักฐานปานกลาง</option>
                    <option value="C">C - หลักฐานจำกัด</option>
                  </select>
                </label>
              </div>
            </div>
          </div>

          <div class="modal-footer">
            <button class="btn btn-secondary" @click="interactionModalOpen = false">ยกเลิก</button>
            <button
              class="btn btn-primary"
              @click="saveDrugInteraction"
              :disabled="!selectedDrug"
            >
              <Plus :size="16" />
              บันทึก
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.settings-view { display: flex; flex-direction: column; gap: var(--spacing-lg); max-width: 900px; }
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
.text-right { text-align: right; }
.btn-icon { padding: var(--spacing-xs); }

/* Interaction Header */
.interaction-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--spacing-xl);
}
.interaction-header-text { display: flex; flex-direction: column; gap: var(--spacing-xxs); }

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xxl) var(--spacing-xl);
  text-align: center;
}
.empty-state-icon {
  width: 56px;
  height: 56px;
  border-radius: var(--rounded-full);
  background: var(--color-pink-50);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-primary);
}

/* Interaction List */
.interaction-list { display: flex; flex-direction: column; gap: var(--spacing-sm); }

.interaction-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-md);
  padding: var(--spacing-md);
  background: var(--color-canvas);
  border: 1px solid var(--color-hairline);
  border-radius: var(--rounded-lg);
  transition: border-color 150ms;
}
.interaction-card:hover { border-color: var(--color-hairline-strong); }

.interaction-card-left {
  display: flex;
  gap: var(--spacing-md);
  flex: 1;
  min-width: 0;
}

.severity-indicator {
  width: 36px;
  height: 36px;
  min-width: 36px;
  border-radius: var(--rounded-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
}
.severity-indicator.severity-contraindicated { background: var(--color-danger); }
.severity-indicator.severity-major { background: var(--color-coral-500); }
.severity-indicator.severity-moderate { background: var(--color-warning); }
.severity-indicator.severity-minor { background: var(--color-stone); }

.interaction-card-content {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xxs);
  flex: 1;
  min-width: 0;
}

.interaction-card-title {
  display: flex;
  align-items: baseline;
  gap: var(--spacing-xs);
  flex-wrap: wrap;
}

.drug-name {
  font-size: var(--typography-body-md-size);
  font-weight: var(--typography-body-sm-medium-weight);
  color: var(--color-ink);
}

.drug-strength {
  font-size: var(--typography-body-sm-size);
  color: var(--color-slate);
}

.drug-icode {
  font-family: var(--font-family-mono);
  font-size: var(--typography-micro-size);
  color: var(--color-stone);
  background: var(--color-surface);
  padding: 1px var(--spacing-xxs);
  border-radius: var(--rounded-xs);
}

.interaction-card-badges {
  display: flex;
  gap: var(--spacing-xxs);
  flex-wrap: wrap;
  margin-top: 2px;
}

.interaction-card-detail {
  font-size: var(--typography-caption-size);
  color: var(--color-slate);
  line-height: 1.4;
  margin: 0;
}

.interaction-card-management {
  font-weight: var(--typography-body-sm-medium-weight);
  color: var(--color-charcoal);
}

.btn-delete {
  color: var(--color-stone);
  transition: color 150ms;
}
.btn-delete:hover { color: var(--color-danger); }

/* Modal */
.modal-overlay {
  position: fixed; inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex; align-items: center; justify-content: center;
  z-index: 100;
  padding: var(--spacing-xl);
}
.modal-card {
  background: var(--color-canvas);
  border-radius: var(--rounded-xl);
  width: 100%; max-width: 520px;
  max-height: 90vh;
  overflow: hidden;
  display: flex; flex-direction: column;
  box-shadow: var(--elevation-4);
}
.modal-card-lg { max-width: 640px; }
.modal-header {
  display: flex; justify-content: space-between; align-items: flex-start;
  padding: var(--spacing-xl);
  border-bottom: 1px solid var(--color-hairline);
}
.modal-body {
  padding: var(--spacing-xl);
  overflow-y: auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xl);
}
.modal-footer {
  display: flex; justify-content: flex-end; gap: var(--spacing-md);
  padding: var(--spacing-lg) var(--spacing-xl);
  border-top: 1px solid var(--color-hairline);
  background: var(--color-surface);
}

/* Form sections in modal */
.form-section {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
}

.form-label {
  font-size: var(--typography-caption-bold-size);
  font-weight: var(--typography-caption-bold-weight);
  color: var(--color-charcoal);
}

.textarea {
  resize: vertical;
  min-height: 48px;
  line-height: 1.5;
}

/* Search */
.search-input-group { display: flex; gap: var(--spacing-xs); }
.search-input-wrapper {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
}
.search-icon {
  position: absolute;
  left: var(--spacing-sm);
  color: var(--color-stone);
  pointer-events: none;
}
.search-input {
  padding-left: calc(var(--spacing-sm) + 16px + var(--spacing-xs));
  width: 100%;
}

.search-results {
  border: 1px solid var(--color-hairline);
  border-radius: var(--rounded-md);
  overflow: hidden;
}
.search-results-header {
  padding: var(--spacing-xs) var(--spacing-sm);
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-hairline-soft);
}
.search-results-list {
  max-height: 220px;
  overflow-y: auto;
}
.search-result-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-md);
  padding: var(--spacing-sm) var(--spacing-md);
  border: none;
  border-bottom: 1px solid var(--color-hairline-soft);
  cursor: pointer;
  text-align: left;
  background: var(--color-canvas);
  width: 100%;
  transition: background 150ms;
}
.search-result-item:last-child { border-bottom: none; }
.search-result-item:hover { background: var(--color-pink-50); }

.search-result-info { display: flex; flex-direction: column; gap: 1px; flex: 1; min-width: 0; }
.search-result-name {
  font-size: var(--typography-body-sm-size);
  font-weight: var(--typography-body-sm-medium-weight);
  color: var(--color-ink);
}
.search-result-meta {
  font-size: var(--typography-caption-size);
  color: var(--color-slate);
}
.search-result-code {
  font-family: var(--font-family-mono);
  font-size: var(--typography-micro-size);
  color: var(--color-stone);
  white-space: nowrap;
}

/* Selected drug section */
.selected-drug-section {
  padding: var(--spacing-md);
  background: var(--color-pink-50);
  border-radius: var(--rounded-lg);
  border: 1px solid var(--color-pink-100);
}
.selected-drug-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  margin-bottom: var(--spacing-sm);
}
.selected-drug-card {
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-canvas);
  border-radius: var(--rounded-md);
  border: 1px solid var(--color-hairline);
}
.selected-drug-info { display: flex; flex-direction: column; gap: 2px; }
.selected-drug-name {
  font-size: var(--typography-body-md-size);
  font-weight: var(--typography-body-sm-medium-weight);
  color: var(--color-ink);
}
.selected-drug-meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: var(--typography-caption-size);
  color: var(--color-slate);
}
.selected-drug-code {
  font-family: var(--font-family-mono);
  font-size: var(--typography-micro-size);
  color: var(--color-stone);
  background: var(--color-surface);
  padding: 1px var(--spacing-xxs);
  border-radius: var(--rounded-xs);
}

.config-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--spacing-md);
  margin-top: var(--spacing-md);
}
.full-width { grid-column: 1 / -1; }
</style>
