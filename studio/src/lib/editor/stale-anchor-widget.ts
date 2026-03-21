import { EditorView, Decoration, WidgetType } from '@codemirror/view';
import { StateField, StateEffect } from '@codemirror/state';
import type { StaleAnchor } from '$lib/tauri/commands';

export const setStaleAnchorsEffect = StateEffect.define<StaleAnchor[]>();

class StaleAnchorWidget extends WidgetType {
  constructor(
    private staleAnchor: StaleAnchor,
    private onAccept: (entityId: string) => void,
    private onDismiss: (entityId: string) => void,
  ) {
    super();
  }

  toDOM() {
    const wrap = document.createElement('div');
    wrap.className = 'stale-anchor-widget';

    const label = document.createElement('span');
    label.className = 'stale-label';
    label.textContent = `\u2191 "${this.staleAnchor.old_snippet}" \u2192 "${this.staleAnchor.new_text}" \u2014 update?`;
    wrap.appendChild(label);

    const yBtn = document.createElement('button');
    yBtn.textContent = 'y';
    yBtn.className = 'stale-btn stale-accept';
    yBtn.onclick = () => this.onAccept(this.staleAnchor.entity_id);
    wrap.appendChild(yBtn);

    const nBtn = document.createElement('button');
    nBtn.textContent = 'n';
    nBtn.className = 'stale-btn stale-dismiss';
    nBtn.onclick = () => this.onDismiss(this.staleAnchor.entity_id);
    wrap.appendChild(nBtn);

    // Keyboard support
    wrap.addEventListener('keydown', (e) => {
      if (e.key === 'y') this.onAccept(this.staleAnchor.entity_id);
      if (e.key === 'n') this.onDismiss(this.staleAnchor.entity_id);
    });

    return wrap;
  }

  ignoreEvent() {
    return false;
  }
}

export function staleAnchorWidgets(
  onAccept: (entityId: string) => void,
  onDismiss: (entityId: string) => void,
) {
  return StateField.define({
    create() {
      return Decoration.none;
    },
    update(decos, tr) {
      for (const effect of tr.effects) {
        if (effect.is(setStaleAnchorsEffect)) {
          const anchors: StaleAnchor[] = effect.value;
          if (anchors.length === 0) return Decoration.none;
          const widgets = anchors
            .filter((sa) => sa.span_end <= tr.state.doc.length)
            .map((sa) =>
              Decoration.widget({
                widget: new StaleAnchorWidget(sa, onAccept, onDismiss),
                block: true,
              }).range(sa.span_end),
            );
          return Decoration.set(widgets, true);
        }
      }
      return decos.map(tr.changes);
    },
    provide: (f) => EditorView.decorations.from(f),
  });
}
