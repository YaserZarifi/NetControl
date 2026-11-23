import { invoke, isTauri } from '@tauri-apps/api/core';
export type Settings = { theme: 'system' | 'light' | 'dark'; units: 'decimal' | 'binary'; retentionDays: number };
export type Foundation = { version: string; databaseReady: boolean; settings: Settings };
export const desktop = isTauri();
const defaults: Settings = { theme: 'system', units: 'decimal', retentionDays: 90 };
function previewSettings(): Settings {
  try {
    const s = JSON.parse(localStorage.getItem('netcontrol-preview') ?? 'null');
    if (s && ['system', 'light', 'dark'].includes(s.theme) && ['decimal', 'binary'].includes(s.units) && [30, 90, 365].includes(s.retentionDays)) return s;
  } catch { /* Fall back when browser storage is unavailable or invalid. */ }
  return defaults;
}
export async function loadFoundation(): Promise<Foundation> {
  return desktop ? invoke('get_foundation') : { version: '0.1.0', databaseReady: false, settings: previewSettings() };
}
export async function saveSettings(settings: Settings): Promise<void> {
  if (desktop) await invoke('save_settings', { settings });
  else localStorage.setItem('netcontrol-preview', JSON.stringify(settings));
}
