import { ref, computed } from "vue";
import type { SupportedLocale, LocaleOption, LocaleMessageSchema } from "./types";
import { en } from "./locales/en";
import { zhCN } from "./locales/zh-CN";
import { zhTW } from "./locales/zh-TW";
import { ja } from "./locales/ja";
import { ko } from "./locales/ko";
import { fr } from "./locales/fr";
import { de } from "./locales/de";
import { es } from "./locales/es";
import { ru } from "./locales/ru";

export type * from "./types";

export const SUPPORTED_LOCALES: LocaleOption[] = [
  { code: "en", label: "English", nativeName: "English" },
  { code: "zh-CN", label: "Chinese (Simplified)", nativeName: "简体中文" },
  { code: "zh-TW", label: "Chinese (Traditional)", nativeName: "繁體中文" },
  { code: "ja", label: "Japanese", nativeName: "日本語" },
  { code: "ko", label: "Korean", nativeName: "한국어" },
  { code: "fr", label: "French", nativeName: "Français" },
  { code: "de", label: "German", nativeName: "Deutsch" },
  { code: "es", label: "Spanish", nativeName: "Español" },
  { code: "ru", label: "Russian", nativeName: "Русский" },
];

export const DICTIONARIES: Record<SupportedLocale, LocaleMessageSchema> = {
  en,
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  ja,
  ko,
  fr,
  de,
  es,
  ru,
};

export const STORAGE_KEY = "karen-locale";

export function resolveLocaleFromTag(tag?: string): SupportedLocale {
  if (!tag) return "en";
  const lang = tag.toLowerCase();
  if (lang.startsWith("zh-tw") || lang.startsWith("zh-hk") || lang.startsWith("zh-mo") || lang.includes("hant")) {
    return "zh-TW";
  }
  if (lang.startsWith("zh")) return "zh-CN";
  if (lang.startsWith("ja")) return "ja";
  if (lang.startsWith("ko")) return "ko";
  if (lang.startsWith("fr")) return "fr";
  if (lang.startsWith("de")) return "de";
  if (lang.startsWith("es")) return "es";
  if (lang.startsWith("ru")) return "ru";
  return "en";
}

export function detectSystemLocale(): SupportedLocale {
  try {
    if (typeof localStorage !== "undefined") {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved && saved in DICTIONARIES) {
        return saved as SupportedLocale;
      }
    }
  } catch {
    // localStorage may be restricted in sandbox/private mode
  }

  const navLang = typeof navigator !== "undefined" ? navigator.language : undefined;
  return resolveLocaleFromTag(navLang);
}

const currentLocale = ref<SupportedLocale>(detectSystemLocale());

export function setLocale(lang: SupportedLocale): void {
  if (lang in DICTIONARIES) {
    currentLocale.value = lang;
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(STORAGE_KEY, lang);
      }
    } catch {
      // Ignore storage errors
    }
    if (typeof document !== "undefined" && document.documentElement) {
      document.documentElement.lang = lang;
    }
  }
}

export function getNestedMessage(obj: unknown, path: string): string | undefined {
  const parts = path.split(".");
  let cur: unknown = obj;
  for (const part of parts) {
    if (!cur || typeof cur !== "object" || !(part in cur)) {
      return undefined;
    }
    cur = (cur as Record<string, unknown>)[part];
  }
  return typeof cur === "string" ? cur : undefined;
}

export function translate(
  locale: SupportedLocale,
  path: string,
  params?: Record<string, string | number>,
): string {
  const dict = DICTIONARIES[locale] || en;
  let raw = getNestedMessage(dict, path);
  if (raw === undefined && locale !== "en") {
    raw = getNestedMessage(en, path);
  }
  if (raw === undefined) {
    return path;
  }
  if (!params) {
    return raw;
  }
  return raw.replace(/\{(\w+)\}/g, (_, key) => {
    return key in params ? String(params[key]) : `{${key}}`;
  });
}

export function t(path: string, params?: Record<string, string | number>): string {
  return translate(currentLocale.value, path, params);
}

export function translateInterfaceText(
  locale: SupportedLocale,
  source: string,
  params?: Record<string, string | number>,
): string {
  const message = DICTIONARIES[locale]?.interfaceText?.[source]
    ?? DICTIONARIES.en.interfaceText[source]
    ?? source;
  if (!params) return message;
  return message.replace(/\{(\w+)\}/g, (_, key) =>
    key in params ? String(params[key]) : `{${key}}`,
  );
}

export function translateBatteryState(locale: SupportedLocale, state?: string | null): string {
  const normalized = state?.toLowerCase();
  const knownStates = ["charging", "discharging", "full", "empty", "unknown"] as const;
  if (!normalized || !knownStates.includes(normalized as typeof knownStates[number])) {
    return translate(locale, "battery.unknown");
  }
  return translate(locale, `battery.${normalized}`);
}

export function useI18n() {
  return {
    locale: currentLocale,
    supportedLocales: SUPPORTED_LOCALES,
    setLocale,
    t: (path: string, params?: Record<string, string | number>) =>
      translate(currentLocale.value, path, params),
    text: (source: string, params?: Record<string, string | number>) =>
      translateInterfaceText(currentLocale.value, source, params),
    batteryState: (state?: string | null) => translateBatteryState(currentLocale.value, state),
    number: (value: number) => new Intl.NumberFormat(currentLocale.value).format(value),
    decimal: (value: number, fractionDigits = 1) => new Intl.NumberFormat(currentLocale.value, {
      minimumFractionDigits: fractionDigits,
      maximumFractionDigits: fractionDigits,
    }).format(value),
    date: (value: number | Date) => new Intl.DateTimeFormat(currentLocale.value).format(value),
    currentDictionary: computed(() => DICTIONARIES[currentLocale.value]),
  };
}
