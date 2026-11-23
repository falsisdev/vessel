import { ref, shallowRef } from 'vue'
import enLocale from '@/locales/en.json'
import trLocale from '@/locales/tr.json'
import { isTauriRuntime, vesselApi } from '@/api/vessel'

type NestedDict = Record<string, unknown>

const BUNDLED_LOCALES: Record<string, NestedDict> = {
  en: enLocale,
  tr: trLocale,
}

const currentLang = shallowRef<string>(
  (typeof localStorage !== 'undefined' && localStorage.getItem('vessel_locale')) || 'en',
)
const availableLocales = ref<string[]>(Object.keys(BUNDLED_LOCALES))
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
    const bundled = Object.keys(BUNDLED_LOCALES)
    if (!isTauriRuntime()) {
      availableLocales.value = bundled
      return
    }
    const extra = await vesselApi.listCustomLocales()
    availableLocales.value = Array.from(new Set([...bundled, ...extra]))
  }

  async function setLocale(lang: string): Promise<void> {
    currentLang.value = lang
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('vessel_locale', lang)
    }
    const bundled = BUNDLED_LOCALES[lang] ?? null
    if (!isTauriRuntime()) {
      customDict.value = lang === 'en' ? null : bundled
      return
    }
    const custom = lang === 'en' ? null : await vesselApi.loadCustomLocale(lang)
    customDict.value = custom ?? (lang === 'en' ? null : bundled)
  }

  return {
    currentLang,
    availableLocales,
    t,
    refreshLocales,
    setLocale,
  }
}
