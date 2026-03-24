import { Node, mergeAttributes } from '@tiptap/core';

/**
 * Custom Tiptap node for AI agent notes injected via the Yjs document.
 * These are read-only blocks rendered inline in the editor showing
 * entity detections, summaries, and questions from the agent server.
 */
export const AgentNote = Node.create({
  name: 'agentNote',
  group: 'block',
  content: 'text*',
  defining: true,

  addAttributes() {
    return {
      agentId: { default: null },
      agentName: { default: null },
      noteType: { default: 'entity' },
      confidence: { default: 0 },
      accepted: { default: false },
    };
  },

  parseHTML() {
    return [{ tag: 'div[data-agent-note]' }];
  },

  renderHTML({ HTMLAttributes }) {
    const noteType = HTMLAttributes.noteType || 'entity';
    const confidence = HTMLAttributes.confidence || 0;
    const agentName = HTMLAttributes.agentName || 'Agent';
    const confidencePct = Math.round(confidence * 100);

    const colorMap: Record<string, string> = {
      entity: '#3b82f6',
      summary: '#22c55e',
      question: '#f59e0b',
    };
    const borderColor = colorMap[noteType] || '#888';

    return [
      'div',
      mergeAttributes(HTMLAttributes, {
        'data-agent-note': '',
        style: `border-left: 3px solid ${borderColor}; padding: 8px 12px; margin: 8px 0; background: ${borderColor}11; border-radius: 4px; font-size: 0.85em;`,
      }),
      [
        'div',
        {
          style: 'display: flex; justify-content: space-between; margin-bottom: 4px; font-size: 0.75em; color: #666;',
        },
        [
          'span',
          {},
          `${noteType.toUpperCase()} — ${agentName}`,
        ],
        [
          'span',
          {},
          `${confidencePct}%`,
        ],
      ],
      ['div', {}, 0], // 0 = content hole
    ];
  },
});
