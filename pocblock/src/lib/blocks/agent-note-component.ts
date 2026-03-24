import { ShadowlessElement } from '@blocksuite/block-std';
import { html, css, type TemplateResult } from 'lit';
import { customElement, property } from 'lit/decorators.js';

@customElement('sparkdown-agent-note')
export class AgentNoteComponent extends ShadowlessElement {
  static override styles = css`
    .agent-note {
      border-left: 3px solid var(--border-color, #4a9eff);
      background: var(--bg-color, #f0f7ff);
      border-radius: 4px;
      padding: 12px 16px;
      font-size: 0.9em;
      margin: 8px 0;
      position: relative;
    }
    .agent-note[data-type="entity"] {
      --border-color: #4a9eff;
      --bg-color: #f0f7ff;
    }
    .agent-note[data-type="summary"] {
      --border-color: #2da44e;
      --bg-color: #f0fff4;
    }
    .agent-note[data-type="question"] {
      --border-color: #e16f24;
      --bg-color: #fff8f0;
    }
    .agent-note.accepted {
      opacity: 0.6;
      border-left-style: dashed;
    }
    .agent-note-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 6px;
      font-size: 0.8em;
      color: #666;
    }
    .agent-name {
      font-weight: 600;
    }
    .confidence {
      background: #e8e8e8;
      border-radius: 8px;
      padding: 1px 6px;
      font-size: 0.85em;
    }
    .agent-note-content {
      line-height: 1.5;
    }
    .agent-note-actions {
      margin-top: 8px;
      display: flex;
      gap: 8px;
    }
    .agent-note-actions button {
      font-size: 0.8em;
      padding: 2px 10px;
      border-radius: 4px;
      border: 1px solid #ddd;
      background: white;
      cursor: pointer;
    }
    .agent-note-actions button:hover {
      background: #f0f0f0;
    }
    .agent-note-actions button.accept {
      border-color: #2da44e;
      color: #2da44e;
    }
    .agent-note-actions button.dismiss {
      border-color: #cf222e;
      color: #cf222e;
    }
  `;

  @property({ type: String }) agentId = '';
  @property({ type: String }) agentName = 'Agent';
  @property({ type: String }) noteType: 'entity' | 'summary' | 'question' = 'entity';
  @property({ type: String }) content = '';
  @property({ type: Number }) confidence = 0;
  @property({ type: Boolean }) accepted = false;

  // Callbacks to be set by parent
  onAccept?: () => void;
  onDismiss?: () => void;

  override render(): TemplateResult {
    const icon = this.noteType === 'entity' ? '\u{1F50D}'
      : this.noteType === 'summary' ? '\u{1F4DD}'
      : '\u{2753}';

    const confidencePct = Math.round(this.confidence * 100);

    return html`
      <div class="agent-note ${this.accepted ? 'accepted' : ''}"
           data-type="${this.noteType}">
        <div class="agent-note-header">
          <span class="agent-name">${icon} ${this.agentName}</span>
          <span class="confidence">${confidencePct}%</span>
        </div>
        <div class="agent-note-content">${this.content}</div>
        ${!this.accepted ? html`
          <div class="agent-note-actions">
            <button class="accept" @click=${this._accept}>Accept</button>
            <button class="dismiss" @click=${this._dismiss}>Dismiss</button>
          </div>
        ` : html`<div class="agent-note-actions"><em>Accepted</em></div>`}
      </div>
    `;
  }

  private _accept() {
    this.onAccept?.();
  }

  private _dismiss() {
    this.onDismiss?.();
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'sparkdown-agent-note': AgentNoteComponent;
  }
}
