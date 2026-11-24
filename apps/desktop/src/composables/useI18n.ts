import { ref, shallowRef } from 'vue'
import arLocale from '@/locales/ar.json'
import deLocale from '@/locales/de.json'
import enLocale from '@/locales/en.json'
import esLocale from '@/locales/es.json'
import faLocale from '@/locales/fa.json'
import frLocale from '@/locales/fr.json'
import itLocale from '@/locales/it.json'
import jaLocale from '@/locales/ja.json'
import ptLocale from '@/locales/pt.json'
import ruLocale from '@/locales/ru.json'
import trLocale from '@/locales/tr.json'
import zhLocale from '@/locales/zh.json'
import { isTauriRuntime, vesselApi } from '@/api/vessel'

type NestedDict = Record<string, unknown>

const BUNDLED_LOCALES: Record<string, NestedDict> = {
  en: enLocale,
  tr: trLocale,
  es: esLocale,
  de: deLocale,
  fr: frLocale,
  it: itLocale,
  ru: ruLocale,
  ja: jaLocale,
  pt: ptLocale,
  zh: zhLocale,
  ar: arLocale,
  fa: faLocale,
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

function applyDocumentLang(lang: string): void {
  if (typeof document === 'undefined') return
  document.documentElement.lang = lang
}

export function useI18n() {
  function t(
    key: string,
    params?: Record<string, string | number | null | undefined>,
  ): string {
    let text = lookupPath(customDict.value, key) ?? lookupPath(enLocale, key) ?? key
    if (params) {
      for (const [name, value] of Object.entries(params)) {
        text = text.replaceAll(`{${name}}`, value == null ? '' : String(value))
      }
    }
    return text
  }

  function getLocaleLabel(code: string): string {
    const key = `languages.${code}`
    const translated = lookupPath(customDict.value, key) ?? lookupPath(enLocale, key)
    if (translated) return translated
    try {
      if (typeof Intl !== 'undefined' && 'DisplayNames' in Intl) {
        const dn = new Intl.DisplayNames([currentLang.value, 'en'], { type: 'language' })
        const name = dn.of(code)
        if (name) return name.charAt(0).toUpperCase() + name.slice(1)
      }
    } catch {
      // Fallback to raw code if custom tag isn't valid BCP-47
    }
    return code
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
    applyDocumentLang(lang)
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
    getLocaleLabel,
    refreshLocales,
    setLocale,
  }
}
