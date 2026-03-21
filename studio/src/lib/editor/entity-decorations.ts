import { ViewPlugin, Decoration } from '@codemirror/view';
import type { DecorationSet, ViewUpdate } from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';
import { entitiesField, setEntitiesEffect } from './semantic-gutter';
import { entityColor } from '$lib/theme/colors';

function buildDecorations(view: import('@codemirror/view').EditorView): DecorationSet {
  const entities = view.state.field(entitiesField);
  const builder = new RangeSetBuilder<Decoration>();

  // Sort by span_start for RangeSetBuilder (requires sorted input)
  const sorted = [...entities].sort((a, b) => a.span_start - b.span_start);

  for (const entity of sorted) {
    const from = entity.span_start;
    const to = Math.min(entity.span_end, view.state.doc.length);

    if (from >= to || from < 0) continue;

    const color = entityColor(entity.type_prefix);
    const style = entity.status === 'synced'
      ? `text-decoration: underline; text-decoration-color: ${color}33; text-underline-offset: 3px;`
      : `text-decoration: underline dotted; text-decoration-color: ${color}26; text-underline-offset: 3px;`;

    builder.add(
      from,
      to,
      Decoration.mark({
        attributes: {
          style,
          'data-entity-id': entity.id,
        },
      })
    );
  }

  return builder.finish();
}

export const entityDecorations = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: import('@codemirror/view').EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: ViewUpdate) {
      if (
        update.docChanged ||
        update.transactions.some((tr) =>
          tr.effects.some((e) => e.is(setEntitiesEffect))
        )
      ) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);
