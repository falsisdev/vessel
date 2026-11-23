<script setup lang="ts">
import { ShieldAlert } from 'lucide-vue-next'
import { useI18n } from '@/composables/useI18n'
import type { PendingDomainPrompt } from '@/composables/useVesselEvents'

defineProps<{
  prompt: PendingDomainPrompt | null
}>()

const emit = defineEmits<{
  resolve: [allow: boolean, permanent: boolean]
}>()

const { t } = useI18n()
</script>

<template>
  <Teleport to="body">
    <Transition name="modal-pop">
      <div v-if="prompt" class="modal-backdrop">
        <div class="v-card modal-card">
          <div class="modal-head">
            <ShieldAlert :size="20" class="warn-icon" />
            <h3 class="modal-title">{{ t('overlay.domain_title') }}</h3>
          </div>
          <p class="modal-body">
            <strong>{{ prompt.pluginId }}</strong> requested outbound HTTPS access to
            <code>{{ prompt.domain }}</code>.
          </p>
          <div class="modal-actions">
            <button class="v-btn" @click="emit('resolve', false, false)">
              {{ t('overlay.reject') }}
            </button>
            <button class="v-btn" @click="emit('resolve', true, false)">
              {{ t('overlay.allow_once') }}
            </button>
            <button class="v-btn v-btn-primary" @click="emit('resolve', true, true)">
              {{ t('overlay.allow_always') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(17, 17, 27, 0.72);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  padding: 1rem;
}

.modal-card {
  width: min(460px, 100%);
  display: flex;
  flex-direction: column;
  gap: 0.95rem;
}

.modal-head {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.warn-icon {
  color: var(--vessel-warning);
}

.modal-title {
  margin: 0;
  font-size: 1.1rem;
}

.modal-body {
  margin: 0;
  line-height: 1.5;
  color: var(--vessel-text-muted);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  flex-wrap: wrap;
}
</style>
