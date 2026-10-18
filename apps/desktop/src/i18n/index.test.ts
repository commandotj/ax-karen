import { beforeEach, describe, expect, it } from "vitest";
import {
  DICTIONARIES,
  SUPPORTED_LOCALES,
  detectSystemLocale,
  resolveLocaleFromTag,
  getNestedMessage,
  setLocale,
  t,
  translate,
  useI18n,
  STORAGE_KEY,
} from "./index";
import type { SupportedLocale } from "./types";

const store: Record<string, string> = {};
const mockStorage = {
  getItem: (k: string) => store[k] ?? null,
  setItem: (k: string, v: string) => {
    store[k] = String(v);
  },
  removeItem: (k: string) => {
    delete store[k];
  },
  clear: () => {
    Object.keys(store).forEach((k) => delete store[k]);
  },
};
Object.defineProperty(window, "localStorage", { value: mockStorage, writable: true });
Object.defineProperty(globalThis, "localStorage", { value: mockStorage, writable: true });

describe("i18n engine and dictionaries", () => {
  beforeEach(() => {
    mockStorage.clear();
    setLocale("en");
  });

  it("registers all 9 supported locales", () => {
    expect(SUPPORTED_LOCALES).toHaveLength(9);
    const codes = SUPPORTED_LOCALES.map((l) => l.code);
    expect(codes).toEqual(["en", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "ru"]);
  });

  it("verifies every dictionary contains complete non-empty navigation keys", () => {
    const navKeys = ["runtime", "cpu", "memory", "battery", "network", "space"] as const;
    for (const locale of SUPPORTED_LOCALES) {
      const dict = DICTIONARIES[locale.code];
      expect(dict).toBeDefined();
      for (const k of navKeys) {
        expect(dict.nav[k]).toBeTruthy();
      }
    }
  });

  it("verifies every dictionary contains valid masthead, network, space, and review keys", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const dict = DICTIONARIES[locale.code];
      expect(dict.masthead.edition).toBeTruthy();
      expect(dict.network.processLedger).toBeTruthy();
      expect(dict.space.scanRoot).toBeTruthy();
      expect(dict.review.title).toBeTruthy();
      expect(dict.dataTable.noData).toBeTruthy();
    }
  });

  it("translates keys in active locale and falls back to English when key is missing", () => {
    setLocale("en");
    expect(t("nav.runtime")).toBe("Runtime");

    setLocale("zh-CN");
    expect(t("nav.runtime")).toBe("运行状态");

    setLocale("ja");
    expect(t("nav.runtime")).toBe("稼働状況");

    setLocale("ru");
    expect(t("nav.runtime")).toBe("Состояние");

    // Non-existent key returns path
    expect(t("unknown.path.to.key")).toBe("unknown.path.to.key");
  });

  it("interpolates parameters into translated strings", () => {
    setLocale("en");
    expect(t("review.deleteConfirm", { count: 5 })).toContain("5");

    setLocale("zh-CN");
    expect(t("review.deleteConfirm", { count: 12 })).toContain("12");

    setLocale("ru");
    expect(t("review.deleteConfirm", { count: 3 })).toContain("3");
  });

  it("resolves language tags accurately across all supported languages", () => {
    expect(resolveLocaleFromTag("zh-TW")).toBe("zh-TW");
    expect(resolveLocaleFromTag("zh-HK")).toBe("zh-TW");
    expect(resolveLocaleFromTag("zh-Hant-TW")).toBe("zh-TW");
    expect(resolveLocaleFromTag("zh-CN")).toBe("zh-CN");
    expect(resolveLocaleFromTag("ja-JP")).toBe("ja");
    expect(resolveLocaleFromTag("ko-KR")).toBe("ko");
    expect(resolveLocaleFromTag("fr-FR")).toBe("fr");
    expect(resolveLocaleFromTag("de-DE")).toBe("de");
    expect(resolveLocaleFromTag("es-ES")).toBe("es");
    expect(resolveLocaleFromTag("ru-RU")).toBe("ru");
    expect(resolveLocaleFromTag("pt-BR")).toBe("en");
    expect(resolveLocaleFromTag(undefined)).toBe("en");
  });

  it("detects system locale from navigator.language or localStorage", () => {
    localStorage.setItem(STORAGE_KEY, "de");
    expect(detectSystemLocale()).toBe("de");

    localStorage.removeItem(STORAGE_KEY);
    expect(["en", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es", "ru"]).toContain(
      detectSystemLocale(),
    );
  });

  it("persists locale changes to localStorage and document lang", () => {
    setLocale("fr");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("fr");
    expect(document.documentElement.lang).toBe("fr");

    setLocale("zh-TW");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("zh-TW");
    expect(document.documentElement.lang).toBe("zh-TW");
  });

  it("safely handles getNestedMessage on non-object values", () => {
    expect(getNestedMessage(null, "foo.bar")).toBeUndefined();
    expect(getNestedMessage("string", "foo")).toBeUndefined();
    expect(getNestedMessage({ foo: { bar: "found" } }, "foo.bar")).toBe("found");
  });

  it("provides reactive useI18n composable", () => {
    const i18n = useI18n();
    expect(i18n.supportedLocales).toBe(SUPPORTED_LOCALES);
    i18n.setLocale("ko");
    expect(i18n.locale.value).toBe("ko");
    expect(i18n.t("nav.runtime")).toBe("실행 상태");
  });
});
