<script setup lang="ts">
import { shallowRef } from 'vue'
import { useI18n } from '@/composables/useI18n'
import type { IptvKind } from '@/types/vessel'

const emit = defineEmits<{
  submit: [
    payload: {
      id: string
      name: string
      kind: IptvKind
      serverUrl: string
      username?: string
      password?: string
      epgUrl?: string
      userAgent?: string
    },
  ]
}>()

const { t } = useI18n()

const name = shallowRef('')
const kind = shallowRef<IptvKind>('m3u')
const serverUrl = shallowRef('')
const username = shallowRef('')
const password = shallowRef('')
const epgUrl = shallowRef('')
const userAgent = shallowRef('')

function handleSubmit(): void {
  if (!name.value.trim() || !serverUrl.value.trim()) return
  const id = `iptv-${Date.now()}`
  emit('submit', {
    id,
    name: name.value.trim(),
    kind: kind.value,
    serverUrl: serverUrl.value.trim(),
    username: username.value.trim() || undefined,
    password: password.value.trim() || undefined,
    epgUrl: epgUrl.value.trim() || undefined,
    userAgent: userAgent.value.trim() || undefined,
  })
  name.value = ''
  serverUrl.value = ''
  username.value = ''
  password.value = ''
  epgUrl.value = ''
  userAgent.value = ''
}
</script>

<template>
  <form class="v-card iptv-form" @submit.prevent="handleSubmit">
    <h3 class="form-title">{{ t('iptv.add_profile') }}</h3>
    <div class="form-grid">
      <input v-model="name" class="v-input" placeholder="Profile Name" required />
      <select v-model="kind" class="v-select">
        <option value="m3u">M3U / M3U-Plus URL</option>
        <option value="xtream">Xtream Codes</option>
      </select>
      <input v-model="serverUrl" class="v-input" placeholder="Playlist or Server URL" required />
      <input v-model="epgUrl" class="v-input" placeholder="XMLTV EPG URL (Optional)" />
      <input v-if="kind === 'xtream'" v-model="username" class="v-input" placeholder="Username" />
      <input
        v-if="kind === 'xtream'"
        v-model="password"
        type="password"
        class="v-input"
        placeholder="Password"
      />
      <input v-model="userAgent" class="v-input" placeholder="Custom User-Agent (Optional)" />
    </div>
    <button type="submit" class="v-btn v-btn-primary">{{ t('common.save') }}</button>
  </form>
</template>

<style scoped>
.iptv-form {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.form-title {
  margin: 0;
}

.form-grid {
  display: grid;
  gap: 0.6rem;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
}
</style>
