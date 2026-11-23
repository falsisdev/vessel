import { ref, shallowRef } from 'vue'
import enLocale from '@/locales/en.json'
import { isTauriRuntime, vesselApi } from '@/api/vessel'

type NestedDict = Record<string, unknown>

const currentLang = shallowRef<string>(
  (typeof localStorage !== 'undefined' && localStorage.getItem('vessel_locale')) || 'en',
)
const availableLocales = ref<string[]>(['en'])
const customDict = ref<NestedDict | null>(null)

function lookupPath(source: NestedDict | null, dottedKey: string): string | null {
  if (!source) return null
  const parts = dottedKey.split('.')
  let cur: unknown = source
  for (const part of parts) {
    if (cur && typeof cur === 'object' && part in (cur as NestedDict)) {
      cur = (cur as NestedDict)[part]
    } else {
      return null
    }
  }
  return typeof cur === 'string' ? cur : null
}

export function useI18n() {
  function t(key: string): string {
    return lookupPath(customDict.value, key) ?? lookupPath(enLocale, key) ?? key
  }

  async function refreshLocales(): Promise<void> {
    if (!isTauriRuntime()) {
      availableLocales.value = ['en']
      return
    }
    const extra = await vesselApi.listCustomLocales()
    const merged = new Set(['en', ...extra])
    availableLocales.value = Array.from(merged)
  }

  async function setLocale(lang: string): Promise<void> {
    currentLang.value = lang
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('vessel_locale', lang)
    }
    if (lang === 'en' || !isTauriRuntime()) {
      customDict.value = null
      return
    }
    customDict.value = await vesselApi.loadCustomLocale(lang)
  }

  return {
    currentLang,
    availableLocales,
    t,
    refreshLocales,
    setLocale,
  }
}
