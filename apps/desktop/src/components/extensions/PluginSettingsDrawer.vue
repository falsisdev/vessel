<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { vesselApi } from '@/api/vessel'
import { useI18n } from '@/composables/useI18n'
import { pushToast } from '@/composables/useVesselEvents'
import type { PluginManifest } from '@/types/vessel'

const props = defineProps<{
  manifest: PluginManifest
}>()

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()
const values = ref<Record<string, unknown>>({})

onMounted(async () => {
  try {
    const loaded = await vesselApi.getPluginSettings(props.manifest.id)
    const next: Record<string, unknown> = { ...loaded }
    for (const field of props.manifest.settingsSchema) {
      if (!(field.key in next) && field.default !== undefined) {
        next[field.key] = field.default
      }
    }
    values.value = next
  } catch (err) {
    pushToast('error', String(err))
  }
})

async function save(): Promise<void> {
  try {
    await vesselApi.setPluginSettings(props.manifest.id, values.value)
    pushToast('info', `Saved settings for ${props.manifest.name}`)
    emit('close')
  } catch (err) {
    pushToast('error', String(err))
  }
}
</script>

<template>
  <div class="v-card drawer-card">
    <div class="drawer-head">
      <h3 class="drawer-title">{{ manifest.name }} — {{ t('extensions.settings') }}</h3>
      <button class="v-btn" @click="emit('close')">{{ t('common.cancel') }}</button>
    </div>

    <div v-for="field in manifest.settingsSchema" :key="field.key" class="field-row">
      <label class="field-label">{{ field.label }}</label>

      <input
        v-if="field.type === 'text' || field.type === 'secret'"
        v-model="values[field.key]"
        :type="field.type === 'secret' ? 'password' : 'text'"
        :placeholder="field.placeholder"
        class="v-input"
      />

      <label v-else-if="field.type === 'toggle'" class="v-switch">
        <input v-model="values[field.key]" type="checkbox" class="v-switch-input" />
        <span class="v-switch-track" aria-hidden="true" />
        <span>{{ values[field.key] ? t('common.enabled') : t('common.disabled') }}</span>
      </label>

      <select v-else-if="field.type === 'select'" v-model="values[field.key]" class="v-select">
        <option v-for="opt in field.options" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
    </div>

    <div class="drawer-actions">
      <button class="v-btn v-btn-primary" @click="save">{{ t('common.save') }}</button>
    </div>
  </div>
</template>

<style scoped>
.drawer-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  border-color: var(--vessel-accent);
}

.drawer-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.drawer-title {
  margin: 0;
}

.field-row {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.field-label {
  font-weight: 600;
  font-size: 0.88rem;
}

.toggle-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.drawer-actions {
  display: flex;
  justify-content: flex-end;
}
</style>
