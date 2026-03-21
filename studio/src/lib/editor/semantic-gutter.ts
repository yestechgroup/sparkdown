import { gutter, GutterMarker } from '@codemirror/view';
import type { EditorView } from '@codemirror/view';
import { StateField, StateEffect } from '@codemirror/state';
import type { EntityDto } from '$lib/tauri/commands';
import { entityColor } from '$lib/theme/colors';

// Effect to update entities from outside
export const setEntitiesEffect = StateEffect.define<EntityDto[]>();

// State field that holds current entity list
export const entitiesField = StateField.define<EntityDto[]>({
  create: () => [],
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setEntitiesEffect)) return effect.value;
    }
    return value;
  },
});

class EntityDotMarker extends GutterMarker {
  constructor(private colors: string[]) {
    super();
  }

  toDOM() {
    const container = document.createElement('div');
    container.style.display = 'flex';
    container.style.flexDirection = 'column';
    container.style.gap = '1px';
    container.style.padding = '2px 0';

    for (const color of this.colors) {
      const dot = document.createElement('div');
      dot.style.width = '4px';
      dot.style.height = '4px';
      dot.style.borderRadius = '50%';
      dot.style.backgroundColor = color;
      container.appendChild(dot);
    }

    return container;
  }
}

export const semanticGutter = gutter({
  class: 'cm-semantic-gutter',
  lineMarker(view: EditorView, line) {
    const entities = view.state.field(entitiesField);
    const lineFrom = line.from;
    const lineTo = line.to;

    const colors: string[] = [];
    for (const entity of entities) {
      if (entity.span_end > lineFrom && entity.span_start < lineTo) {
        colors.push(entityColor(entity.type_prefix));
      }
    }

    if (colors.length === 0) return null;

    // Deduplicate colors
    const unique = [...new Set(colors)];
    return new EntityDotMarker(unique);
  },
  lineMarkerChange(update) {
    return update.transactions.some((tr) =>
      tr.effects.some((e) => e.is(setEntitiesEffect))
    );
  },
});
