<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { AlertTriangle, ShieldAlert, ShieldCheck, ShieldX } from 'lucide-vue-next';
import { onMounted, ref, watch } from 'vue';
import { useSettingsStore } from '#/stores/settings.ts';
import type { Interaction, Severity } from '#/types/interaction.ts';

const props = defineProps<{
  hn: string;
}>();

const emit = defineEmits<(e: 'blocked', blocked: boolean) => void>();

const store = useSettingsStore();
const loading = ref(false);
const error = ref<string | null>(null);
const interactions = ref<Interaction[]>([]);

async function checkInteractions() {
  loading.value = true;
  error.value = null;
  try {
    interactions.value = await invoke<Interaction[]>('check_patient_interactions', {
      hn: props.hn,
    });
    emit('blocked', hasContraindicated());
  } catch (e) {
    error.value = String(e);
    emit('blocked', false);
  } finally {
    loading.value = false;
  }
}

function severityClass(severity: Severity): string {
  switch (severity) {
    case 'contraindicated':
      return 'severity-contraindicated';
    case 'major':
      return 'severity-major';
    case 'moderate':
      return 'severity-moderate';
    case 'minor':
      return 'severity-minor';
    default:
      return 'severity-minor';
  }
}

function severityIcon(severity: Severity) {
  switch (severity) {
    case 'contraindicated':
      return ShieldX;
    case 'major':
      return ShieldAlert;
    case 'moderate':
      return AlertTriangle;
    case 'minor':
      return ShieldCheck;
    default:
      return ShieldCheck;
  }
}

function severityLabel(severity: Severity): string {
  switch (severity) {
    case 'contraindicated':
      return 'ห้ามใช้ร่วม';
    case 'major':
      return 'หลีกเลี่ยง';
    case 'moderate':
      return 'ระวัง';
    case 'minor':
      return 'ทราบ';
    default:
      return severity;
  }
}

const hasContraindicated = () => interactions.value.some((i) => i.severity === 'contraindicated');

watch(
  () => props.hn,
  () => {
    if (props.hn) {
      void checkInteractions();
    }
  },
);

onMounted(() => {
  if (props.hn) {
    void checkInteractions();
  }
});
</script>

<template>
  <div class="interaction-checker">
    <div v-if="loading" class="body-sm" style="color: var(--color-slate)">
      กำลังตรวจสอบปฏิกิริยา...
    </div>

    <div v-else-if="error" class="card card-feature-coral body-sm" style="padding: var(--spacing-md)">
      {{ error }}
    </div>

    <template v-else-if="interactions.length > 0">
      <div
        v-if="hasContraindicated()"
        class="card card-feature-coral interaction-banner"
        role="alert"
        aria-live="assertive"
      >
        <ShieldX :size="20" />
        <span class="body-sm-medium">มียาที่ห้ามใช้ร่วมกับ Warfarin — ตรวจสอบก่อนบันทึก</span>
      </div>

      <div class="interaction-list">
        <div
          v-for="(item, idx) in interactions"
          :key="`${item.icode}-${idx}`"
          :class="['interaction-item', severityClass(item.severity)]"
        >
          <div class="interaction-header">
            <component :is="severityIcon(item.severity)" :size="16" />
            <span class="body-sm-medium">{{ item.drugName }}</span>
            <span v-if="item.strength" class="caption">{{ item.strength }}</span>
            <span :class="['severity-badge', severityClass(item.severity)]">
              {{ severityLabel(item.severity) }}
            </span>
            <span class="interaction-type-badge">
              {{ item.interactionType === 'increase' ? 'เพิ่มฤทธิ์' : 'ลดฤทธิ์' }}
            </span>
          </div>
          <div v-if="item.clinicalEffect" class="caption interaction-detail">
            {{ item.clinicalEffect }}
          </div>
          <div v-if="item.management" class="caption interaction-detail interaction-management">
            แนะนำ: {{ item.management }}
          </div>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="interaction-empty">
        <ShieldCheck :size="16" style="color: var(--color-inr-safe)" />
        <span class="caption" style="color: var(--color-slate)">ไม่พบปฏิกิริยากับยา Warfarin</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.interaction-checker {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.interaction-banner {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-md);
}

.interaction-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.interaction-item {
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--rounded-md);
  border-left: 3px solid transparent;
}

.interaction-item.severity-contraindicated {
  border-left-color: var(--color-brand-red);
  background: rgba(239, 68, 68, 0.05);
}

.interaction-item.severity-major {
  border-left-color: var(--color-coral-500);
  background: rgba(244, 63, 94, 0.05);
}

.interaction-item.severity-moderate {
  border-left-color: var(--color-brand-orange-light);
  background: rgba(251, 146, 60, 0.05);
}

.interaction-item.severity-minor {
  border-left-color: var(--color-slate);
  background: rgba(100, 116, 139, 0.03);
}

.interaction-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  flex-wrap: wrap;
}

.severity-badge {
  font-size: var(--typography-micro-size);
  font-weight: 600;
  padding: 1px var(--spacing-xs);
  border-radius: var(--rounded-full);
}

.severity-badge.severity-contraindicated {
  background: var(--color-brand-red);
  color: white;
}

.severity-badge.severity-major {
  background: var(--color-coral-500);
  color: white;
}

.severity-badge.severity-moderate {
  background: var(--color-brand-orange-light);
  color: white;
}

.severity-badge.severity-minor {
  background: var(--color-stone);
  color: white;
}

.interaction-type-badge {
  font-size: var(--typography-micro-size);
  padding: 1px var(--spacing-xs);
  border-radius: var(--rounded-full);
  background: var(--color-pink-100);
  color: var(--color-ink);
}

.interaction-detail {
  margin-top: var(--spacing-xxs);
  color: var(--color-slate);
}

.interaction-management {
  font-weight: 500;
  color: var(--color-ink);
}

.interaction-empty {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}
</style>
