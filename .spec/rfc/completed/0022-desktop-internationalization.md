# RFC 0022: Desktop application multi-language internationalization

**Status:** Implemented

## Summary

Implement native multi-language internationalization (i18n) across the Karen desktop application with zero external runtime dependencies. Provide complete localization for CJK (Chinese Simplified `zh-CN`, Chinese Traditional `zh-TW`, Japanese `ja`, Korean `ko`), English (`en`), French (`fr`), German (`de`), Spanish (`es`), and Russian (`ru`). The system includes a type-safe reactive translation composable (`useI18n`), persistence via `localStorage` with browser/OS locale auto-detection, a steampunk brass language selector in the masthead, complete translation of all tabs and views (Runtime, CPU, Memory, Battery, Network, Space/Disk, Review, Help, and DataTable), and font-family fallbacks for crisp CJK and Cyrillic typography.

## Problem

Karen currently hardcodes English strings across its Vue components, templates, and data tables. While the numerical formatting (`formatBytes`, `formatDuration`, `formatPercent`) is language-agnostic, users in non-English speaking regions cannot navigate or read hardware telemetry, process descriptions, network state labels, and critical disk removal safety prompts in their native languages.

Specifically:
1. **Lack of Multi-Language Support**: All UI labels, table headers, buttons, tooltips, dialogs, and error messages are English-only.
2. **Missing CJK & Russian Support**: CJK (Chinese, Japanese, Korean) and Russian (Cyrillic) require appropriate typographic fallback stacks and localized domain terminology (e.g. system memory, socket states, battery health).
3. **No Dynamic Language Switching**: Users need to seamlessly switch languages at runtime from the masthead without requiring application restarts.
4. **Architectural Purity**: Introducing large runtime packages like `vue-i18n` would inflate bundle size and add unnecessary runtime complexity; Karen's lightweight reactive frontend architecture requires a typed, minimal, and fully-tested native solution.

## Goals

1. **Comprehensive Language Coverage**:
   - English (`en` - baseline).
   - Chinese Simplified (`zh-CN`) & Chinese Traditional (`zh-TW`).
   - Japanese (`ja`).
   - Korean (`ko`).
   - French (`fr`).
   - German (`de`).
   - Spanish (`es`).
   - Russian (`ru`).
2. **Zero-Dependency Lightweight i18n Engine**:
   - Provide a type-safe reactive composable `useI18n` in `apps/desktop/src/i18n/`.
   - Support nested key lookup (`t('runtime.hostTelemetry')`) and parameterized interpolation (`t('space.markedCount', { count: 3 })`).
3. **Persistence and Auto-Detection**:
   - Auto-detect user system language via `navigator.language` on initial launch.
   - Persist user-selected language to `localStorage` under `karen-locale`.
4. **Complete UI Localization**:
   - Localize all view components: `RuntimeView`, `CpuView`, `MemoryView`, `BatteryView`, `NetworkView`, `DiskTreeView`.
   - Localize shared components: `TabWheel`, `DataTable`, masthead, review drawer, removal confirmation modal, and keyboard shortcut help.
5. **Steampunk Aesthetic & Typographic Fallback**:
   - Integrate an elegant brass language selector widget in the desktop masthead matching the visual token system.
   - Ensure `style.css` font stacks provide clean, native fallbacks for CJK characters and Cyrillic glyphs.
6. **100% Test Coverage**:
   - Comprehensive unit tests for `useI18n` and dictionary integrity across all 8 locales.
   - Maintain 100% branch and line coverage with 0 lint warnings.

## Non-goals

- Dynamic runtime loading of external remote translation bundles (all translations are compiled statically into the application).
- Translating OS-provided process names, filesystem paths, or raw kernel error strings.

## Design

### 1. Dictionary Architecture (`apps/desktop/src/i18n/`)

Structured locale dictionaries implementing a common TypeScript interface:
```
apps/desktop/src/i18n/
├── index.ts          # useI18n composable, reactive state, locale switcher
├── types.ts          # LocaleMessageSchema definition
└── locales/
    ├── en.ts         # English (reference)
    ├── zh-CN.ts      # 简体中文
    ├── zh-TW.ts      # 繁體中文
    ├── ja.ts         # 日本語
    ├── ko.ts         # 한국어
    ├── fr.ts         # Français
    ├── de.ts         # Deutsch
    ├── es.ts         # Español
    └── ru.ts         # Русский
```

### 2. Message Schema (`types.ts`)

```typescript
export interface LocaleMessageSchema {
  nav: {
    runtime: string;
    cpu: string;
    memory: string;
    battery: string;
    network: string;
    space: string;
  };
  masthead: {
    title: string;
    edition: string;
    language: string;
    review: string;
    help: string;
  };
  runtime: {
    title: string;
    hostTelemetry: string;
    uptime: string;
    loadAverage: string;
    // ...
  };
  cpu: { ... };
  memory: { ... };
  battery: { ... };
  network: { ... };
  space: { ... };
  dataTable: { ... };
  dialogs: { ... };
}
```

### 3. Composable (`index.ts`)

```typescript
export function useI18n() {
  return {
    locale: currentLocale,
    t: (path: string, params?: Record<string, string | number>) => string,
    setLocale: (lang: SupportedLocale) => void,
    supportedLocales: [...]
  };
}
```

### 4. Language Selector in Masthead (`App.vue`)

Place an ornate brass select dropdown in `.masthead-right` with language codes/names:
- `EN` (English)
- `简` (简体中文)
- `繁` (繁體中文)
- `JA` (日本語)
- `KO` (한국어)
- `FR` (Français)
- `DE` (Deutsch)
- `ES` (Español)
- `RU` (Русский)

### 5. Font Fallback Stack (`style.css`)

Update root font stack to prioritize CJK and Cyrillic system typefaces smoothly:
```css
font-family: Manrope, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "PingFang SC", "Hiragino Sans", "Apple SD Gothic Neo", "Microsoft YaHei", sans-serif;
```

## Acceptance

1. All 8 target language locales (`en`, `zh-CN`, `zh-TW`, `ja`, `ko`, `fr`, `de`, `es`, `ru`) are fully authored with valid translation keys.
2. Changing the language selector immediately updates the UI across all tabs without reloading.
3. Language preference persists across application restarts in `localStorage`.
4. Typographic rendering remains crisp and properly aligned in CJK and Russian scripts.
5. `pnpm lint`, `pnpm test`, and `pnpm build` pass with 100% test coverage and 0 errors.
