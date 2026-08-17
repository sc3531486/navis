/**
 * ============================================================
 * navis-settings 扩展前端 — 迁移过渡期 re-export 桥
 * ============================================================
 *
 * 实际文件仍保留在 src/components/Settings/、
 * src/stores/settings.ts、src/styles/settings/
 * 后续 Phase 执行物理搬迁时，此处的 re-export 路径将同步更新。
 *
 * 归属说明：
 *   - Settings 组件 → 扩展 navis-settings
 *   - settings store → 扩展 navis-settings
 *   - settings CSS → 扩展 navis-settings
 * ============================================================
 */

// ── Settings 组件 ────────────────────────────────────────
export { default as SettingsDialogContent } from '@/components/Settings/SettingsDialogContent';
export { default as CodingSettingsEditor } from '@/components/Settings/CodingSettingsEditor';
export { default as ExtensionConfigurationEditor } from '@/components/Settings/ExtensionConfigurationEditor';
export { default as ExtensionsManager } from '@/components/Settings/ExtensionsManager';
export { default as GatewayConfigEditor } from '@/components/Settings/GatewayConfigEditor';
export { default as GatewayModelList } from '@/components/Settings/GatewayModelList';
export { default as GatewayProviderRail } from '@/components/Settings/GatewayProviderRail';
export { openSettingsDialog } from '@/components/Settings/openSettingsDialog';
export type { GatewayConfigModel } from '@/components/Settings/gateway-config-model';

// ── Settings Store ───────────────────────────────────────
export {
  settingsState,
  setSettingsState,
  loadEditorSettings,
  updateEditorSettings,
  resetEditorSettingsDraft,
  saveEditorSettings,
  resetSettingsState,
} from '@/stores/settings';
