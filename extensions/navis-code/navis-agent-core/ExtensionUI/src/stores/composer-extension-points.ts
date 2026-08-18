import type { UiExtensionPointRegistration } from '@/lib/extension-ui';
import {
  extensionPointsByKind,
  inlinePointPosition,
} from '@/stores/extension-points';

/** Navis Code Agent Composer 消费的产品级 inline 扩展点。 */
export function composerInlineExtensionPoints(): UiExtensionPointRegistration[] {
  return extensionPointsByKind('inline').filter((point) => {
    if (point.target !== 'Chat') return false;
    const position = inlinePointPosition(point);
    return position === null || position === 'BeforeInput';
  });
}
