import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';
import { getLocale, setLocale, SUPPORTED_LOCALES, type SupportedLocale } from '@/i18n';

export interface LanguageOption {
  value: SupportedLocale;
  label: string;
}

export interface LanguageState {
  language: SupportedLocale;
  builtinLanguages: LanguageOption[];
  loaded: boolean;
  loading: boolean;
  error: string | null;
}

interface UiLanguageState {
  language: SupportedLocale;
  builtinLanguages: LanguageOption[];
}

export const [languageState, setLanguageState] = createStore<LanguageState>({
  language: getLocale(),
  builtinLanguages: SUPPORTED_LOCALES.map((locale) => ({
    value: locale,
    label: locale === 'zh-CN' ? '中文（简体）' : 'English',
  })),
  loaded: false,
  loading: false,
  error: null,
});

function applyLanguageState(state: UiLanguageState): void {
  setLanguageState({
    language: state.language,
    builtinLanguages: state.builtinLanguages,
    loaded: true,
    loading: false,
    error: null,
  });
}

export async function loadLanguage(): Promise<void> {
  setLanguageState('loading', true);
  setLanguageState('error', null);

  try {
    const state = await invoke<UiLanguageState>('ui_get_language');
    await setLocale(state.language);
    applyLanguageState(state);
  } catch (error) {
    setLanguageState('loaded', true);
    setLanguageState('loading', false);
    setLanguageState('error', error instanceof Error ? error.message : String(error));
  }
}

export async function setAppLanguage(language: SupportedLocale): Promise<void> {
  setLanguageState('loading', true);
  setLanguageState('error', null);

  try {
    const state = await invoke<UiLanguageState>('ui_set_language', {
      payload: { language },
    });
    await setLocale(state.language);
    applyLanguageState(state);
  } catch (error) {
    setLanguageState('loading', false);
    setLanguageState('error', error instanceof Error ? error.message : String(error));
  }
}
