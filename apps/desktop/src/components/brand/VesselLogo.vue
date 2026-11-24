<script setup lang="ts">
withDefaults(
  defineProps<{
    size?: number
  }>(),
  { size: 22 },
)
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <!-- Bow "V" Monolith cargo ship mark -->
    <rect x="8.5" y="3.5" width="7" height="3" rx="0.9" fill="var(--vessel-accent)" />
    <rect x="6" y="7.5" width="5.5" height="3.2" rx="0.9" fill="var(--vessel-accent-secondary)" />
    <rect x="12.5" y="7.5" width="5.5" height="3.2" rx="0.9" fill="currentColor" opacity="0.92" />
    <path d="M3.5 12L12 20.8L20.5 12H16.2L12 16.4L7.8 12H3.5Z" fill="currentColor" />
  </svg>
</template>
