import { ViewPlugin, Decoration, WidgetType } from '@codemirror/view';
import type { DecorationSet, ViewUpdate, EditorView } from '@codemirror/view';
import { StateField, StateEffect, RangeSetBuilder } from '@codemirror/state';
import type { StaleAnchor } from '$lib/tauri/commands';

// Effect to push stale anchors into CM state
export const setStaleAnchorsEffect = StateEffect.define<StaleAnchor[]>();

export const staleAnchorsField = StateField.define<StaleAnchor[]>({
  create: () => [],
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setStaleAnchorsEffect)) return effect.value;
    }
    return value;
  },
});

class StaleNudgeWidget extends WidgetType {
  constructor(
    private anchor: StaleAnchor,
    private onAccept: (entityId: string) => void,
    private onDismiss: (entityId: string) => void,
  ) {
    super();
  }

  toDOM() {
    const container = document.createElement('span');
    container.className = 'stale-nudge';
    container.style.cssText = `
      display: inline-flex;
      align-items: center;
      gap: 4px;
      margin-left: 8px;
      font-size: 11px;
      font-family: var(--font-ui, Inter, sans-serif);
      color: #F59E0B;
      opacity: 0.8;
    `;

    const text = document.createElement('span');
    text.textContent = `"${this.anchor.old_snippet}" → "${this.anchor.new_text}" — update?`;
    text.style.cssText = 'max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;';

    const yBtn = document.createElement('button');
    yBtn.textContent = 'y';
    yBtn.style.cssText = `
      background: none; border: 1px solid #F59E0B44; color: #F59E0B;
      padding: 0 4px; border-radius: 2px; cursor: pointer; font-size: 10px;
      font-family: var(--font-ui, Inter, sans-serif);
    `;
    yBtn.onclick = (e) => {
      e.stopPropagation();
      this.onAccept(this.anchor.entity_id);
    };

    const nBtn = document.createElement('button');
    nBtn.textContent = 'n';
    nBtn.style.cssText = `
      background: none; border: 1px solid #73737344; color: #737373;
      padding: 0 4px; border-radius: 2px; cursor: pointer; font-size: 10px;
      font-family: var(--font-ui, Inter, sans-serif);
    `;
    nBtn.onclick = (e) => {
      e.stopPropagation();
      this.onDismiss(this.anchor.entity_id);
    };

    container.append(text, yBtn, nBtn);
    return container;
  }

  ignoreEvent() {
    return false;
  }
}

export function createStaleNudgePlugin(
  onAccept: (entityId: string) => void,
  onDismiss: (entityId: string) => void,
) {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = this.build(view);
      }

      build(view: EditorView): DecorationSet {
        const anchors = view.state.field(staleAnchorsField);
        const builder = new RangeSetBuilder<Decoration>();

        const sorted = [...anchors].sort((a, b) => a.span_end - b.span_end);
        for (const anchor of sorted) {
          const pos = Math.min(anchor.span_end, view.state.doc.length);
          if (pos < 0) continue;
          builder.add(
            pos,
            pos,
            Decoration.widget({
              widget: new StaleNudgeWidget(anchor, onAccept, onDismiss),
              side: 1,
            }),
          );
        }

        return builder.finish();
      }

      update(update: ViewUpdate) {
        if (
          update.docChanged ||
          update.transactions.some((tr) =>
            tr.effects.some((e) => e.is(setStaleAnchorsEffect)),
          )
        ) {
          this.decorations = this.build(update.view);
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    },
  );
}
