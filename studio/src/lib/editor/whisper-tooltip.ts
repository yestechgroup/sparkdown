import { hoverTooltip } from '@codemirror/view';
import type { EditorView, Tooltip } from '@codemirror/view';
import { entitiesField } from './semantic-gutter';
import { entityColor } from '$lib/theme/colors';

export const whisperTooltip = hoverTooltip(
  (view: EditorView, pos: number): Tooltip | null => {
    const entities = view.state.field(entitiesField);

    // Find entity at position
    const entity = entities.find(
      (e) => pos >= e.span_start && pos < e.span_end
    );

    if (!entity) return null;

    return {
      pos: entity.span_start,
      end: entity.span_end,
      above: false,
      create() {
        const dom = document.createElement('div');
        dom.className = 'whisper-card';

        const color = entityColor(entity.type_prefix);

        dom.innerHTML = `
          <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
            <span style="width: 6px; height: 6px; border-radius: 50%; background: ${color}; flex-shrink: 0;"></span>
            <strong style="color: #E5E5E5; font-size: 13px;">${escapeHtml(entity.label)}</strong>
          </div>
          <div style="color: #A3A3A3; font-size: 11px; margin-bottom: 2px;">${escapeHtml(entity.type_prefix)}</div>
          ${entity.top_relations
            .map(
              (r) =>
                `<div style="color: #737373; font-size: 11px;">${escapeHtml(r.predicate_label)} → ${escapeHtml(r.target_label)}</div>`
            )
            .join('')}
        `;

        dom.style.cssText = `
          background: #1E1E1E;
          border: 1px solid #333;
          border-radius: 6px;
          padding: 8px 10px;
          max-width: 260px;
          font-family: var(--font-ui, Inter, sans-serif);
        `;

        return { dom };
      },
    };
  },
  { hoverTime: 300 }
);

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
