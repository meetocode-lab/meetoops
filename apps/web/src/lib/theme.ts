export type ThemePreference = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'meetoops-theme-preference';

export function loadThemePreference(): ThemePreference {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored;
  }
  return 'system';
}

export function resolveTheme(pref: ThemePreference): 'light' | 'dark' {
  if (pref === 'light' || pref === 'dark') return pref;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function applyThemePreference(pref: ThemePreference): void {
  localStorage.setItem(STORAGE_KEY, pref);
  document.documentElement.setAttribute('data-theme', resolveTheme(pref));
}

export function watchSystemTheme(getPref: () => ThemePreference): () => void {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  const listener = () => {
    if (getPref() === 'system') {
      document.documentElement.setAttribute('data-theme', mq.matches ? 'dark' : 'light');
    }
  };
  mq.addEventListener('change', listener);
  return () => mq.removeEventListener('change', listener);
}
