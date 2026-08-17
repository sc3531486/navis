import { For } from 'solid-js';
import type { Component } from 'solid-js';

export type UnifiedDiffLineKind = 'add' | 'delete' | 'hunk' | 'file' | 'context';

export interface UnifiedDiffLine {
  kind: UnifiedDiffLineKind;
  oldLine: string;
  newLine: string;
  text: string;
}

export function parseUnifiedDiff(value: string): UnifiedDiffLine[] {
  const lines = value.split('\n');
  let oldLine: number | null = null;
  let newLine: number | null = null;

  return lines.map((line) => {
    if (/^(diff --git|index |--- |\+\+\+ )/.test(line) || line.startsWith('...')) {
      return { kind: 'file', oldLine: '', newLine: '', text: line };
    }

    const hunk = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (hunk) {
      oldLine = Number(hunk[1]);
      newLine = Number(hunk[2]);
      return { kind: 'hunk', oldLine: '', newLine: '', text: line };
    }

    if (line.startsWith('+')) {
      const display = { kind: 'add' as const, oldLine: '', newLine: newLine == null ? '' : String(newLine), text: line };
      if (newLine != null) newLine += 1;
      return display;
    }

    if (line.startsWith('-')) {
      const display = { kind: 'delete' as const, oldLine: oldLine == null ? '' : String(oldLine), newLine: '', text: line };
      if (oldLine != null) oldLine += 1;
      return display;
    }

    const display = {
      kind: 'context' as const,
      oldLine: oldLine == null ? '' : String(oldLine),
      newLine: newLine == null ? '' : String(newLine),
      text: line,
    };
    if (oldLine != null) oldLine += 1;
    if (newLine != null) newLine += 1;
    return display;
  });
}

export interface UnifiedDiffViewerProps {
  diff: string;
  class?: string;
  ariaLabel?: string;
}

const UnifiedDiffViewer: Component<UnifiedDiffViewerProps> = (props) => (
  <div class={`navis-unified-diff ${props.class ?? ''}`.trim()} role="table" aria-label={props.ariaLabel ?? 'Diff'}>
    <For each={parseUnifiedDiff(props.diff)}>
      {(line) => (
        <div class={`navis-unified-diff-line is-${line.kind}`} role="row">
          <span class="navis-unified-diff-number" role="cell">{line.oldLine}</span>
          <span class="navis-unified-diff-number" role="cell">{line.newLine}</span>
          <code class="navis-unified-diff-code" role="cell">{line.text}</code>
        </div>
      )}
    </For>
  </div>
);

export default UnifiedDiffViewer;
