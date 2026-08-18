import { Component } from 'solid-js';
import { dialog } from '@navis-code/components/Dialog';
import SettingsDialogContent, { type SettingsTab } from './SettingsDialogContent';
import type { ExtensionsFilter } from './ExtensionsManager';

interface OpenSettingsDialogOptions {
  extensionsFilter?: ExtensionsFilter;
}

export async function openSettingsDialog(
  initialTab: SettingsTab = 'gateway',
  message = '',
  options: OpenSettingsDialogOptions = {},
): Promise<void> {
  const Content: Component = () => <SettingsDialogContent initialTab={initialTab} extensionsFilter={options.extensionsFilter} />;
  await dialog.custom('Settings', Content, message);
}
