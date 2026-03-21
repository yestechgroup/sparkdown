<script lang="ts">
  import { getStaleAnchors } from '$lib/stores/document.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { updateStaleAnchor, dismissSuggestion } from '$lib/tauri/commands';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();
  let staleAnchors = $derived(getStaleAnchors());
  let activeDocId = $derived(getActiveDocId());

  async function handleAccept(entityId: string) {
    if (!activeDocId) return;
    try {
      await updateStaleAnchor(activeDocId, entityId);
    } catch (e) {
      console.error('Failed to update stale anchor:', e);
    }
  }

  async function handleDismiss(entityId: string) {
    if (!activeDocId) return;
    try {
      await dismissSuggestion(activeDocId, entityId);
    } catch (e) {
      console.error('Failed to dismiss suggestion:', e);
    }
  }
</script>

{#if staleAnchors.length > 0}
  <div class="suggestion-ribbon">
    <div class="ribbon-header">
      <span class="ribbon-title">{staleAnchors.length} stale anchor{staleAnchors.length > 1 ? 's' : ''}</span>
      <button class="ribbon-close" onclick={onClose}>&times;</button>
    </div>
    <div class="ribbon-items">
      {#each staleAnchors as anchor}
        <div class="ribbon-item">
          <span class="suggestion-text">
            "{anchor.old_snippet}" &rarr; "{anchor.new_text}"
          </span>
          <div class="suggestion-actions">
            <button class="action-accept" onclick={() => handleAccept(anchor.entity_id)} title="Accept update">&#10003;</button>
            <button class="action-dismiss" onclick={() => handleDismiss(anchor.entity_id)} title="Dismiss">&times;</button>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .suggestion-ribbon {
    background: #1A1A1A;
    border-top: 1px solid #333;
    padding: 6px 12px;
    font-family: var(--font-ui, Inter, sans-serif);
    max-height: 120px;
    overflow-y: auto;
  }

  .ribbon-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 4px;
  }

  .ribbon-title {
    font-size: 11px;
    color: #F59E0B;
    font-weight: 500;
  }

  .ribbon-close {
    background: none;
    border: none;
    color: var(--text-muted, #737373);
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px;
  }

  .ribbon-close:hover {
    color: var(--text-primary, #E5E5E5);
  }

  .ribbon-items {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .ribbon-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 3px 8px;
    background: #222;
    border-radius: 4px;
    font-size: 11px;
  }

  .suggestion-text {
    color: var(--text-secondary, #A3A3A3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 280px;
  }

  .suggestion-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .action-accept, .action-dismiss {
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 12px;
    padding: 0 4px;
    border-radius: 2px;
    font-family: var(--font-ui, Inter, sans-serif);
  }

  .action-accept {
    color: #22C55E;
    border-color: #22C55E44;
  }

  .action-accept:hover {
    background: #22C55E1a;
  }

  .action-dismiss {
    color: #737373;
    border-color: #73737344;
  }

  .action-dismiss:hover {
    background: #7373731a;
    color: #F43F5E;
  }
</style>
