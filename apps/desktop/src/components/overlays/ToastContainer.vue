<script setup lang="ts">
import type { ToastItem } from '@/composables/useVesselEvents'

defineProps<{
  items: ToastItem[]
}>()
</script>

<template>
  <Teleport to="body">
    <TransitionGroup
      name="toast-slide"
      tag="div"
      class="toast-stack"
      aria-live="polite"
      aria-atomic="false"
    >
      <div
        v-for="item in items"
        :key="item.id"
        class="v-card toast-item"
        :class="`tone-${item.tone}`"
        role="status"
      >
        {{ item.message }}
      </div>
    </TransitionGroup>
  </Teleport>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  right: 1.15rem;
  bottom: 1.15rem;
  z-index: 300;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  max-width: 380px;
}

.toast-item {
  font-size: 0.88rem;
  background: var(--vessel-surface-overlay);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.28);
}

.tone-warn {
  border-color: var(--vessel-warning);
}

.tone-error {
  border-color: var(--vessel-danger);
}
</style>
