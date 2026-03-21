<script lang="ts">
  import { getEntities, getSidecarStatus, getStaleAnchors, getEditorMode } from '$lib/stores/document.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { exportDocument } from '$lib/tauri/commands';

  interface Props {
    onShowRibbon?: () => void;
  }

  let { onShowRibbon }: Props = $props();

  let entities = $derived(getEntities());
  let status = $derived(getSidecarStatus());
  let staleAnchors = $derived(getStaleAnchors());
  let activeDocId = $derived(getActiveDocId());
  let editorMode = $derived(getEditorMode());
  let showExportMenu = $state(false);

  let statusText = $derived.by(() => {
    const stale = status.stale;
    const detached = status.detached;
    if (stale === 0 && detached === 0) return 'synced';
    const parts: string[] = [];
    if (stale > 0) parts.push(`${stale} stale`);
    if (detached > 0) parts.push(`${detached} detached`);
    return parts.join(', ');
  });

  let modeLabel = $derived.by(() => {
    switch (editorMode) {
      case 'deep-writing': return 'writing';
      case 'light-writing': return 'paused';
      case 'review': return 'review';
      case 'full-reading': return 'reading';
      default: return '';
    }
  });

  async function handleExport(format: 'html_rdfa' | 'json_ld' | 'turtle') {
    if (!activeDocId) return;
    try {
      const result = await exportDocument(activeDocId, format);
      console.log(`Exported ${format}:`, result.substring(0, 200));
    } catch (e) {
      console.error('Export failed:', e);
    }
    showExportMenu = false;
  }
</script>

<div class="suggestion-tray">
  <span class="tray-item">{entities.length} entities</span>
  <span class="tray-separator">&middot;</span>
  <span class="tray-item">sidecar: {statusText}</span>
  <span class="tray-separator">&middot;</span>
  <span class="tray-item">{status.total_triples} triples</span>

  {#if staleAnchors.length > 0 && onShowRibbon}
    <span class="tray-separator">&middot;</span>
    <button class="tray-item stale-link" onclick={onShowRibbon}>
      {staleAnchors.length} stale
    </button>
  {/if}

  <div class="tray-spacer"></div>

  <span class="mode-indicator">{modeLabel}</span>

  {#if activeDocId}
    <div class="export-wrapper">
      <button class="tray-button" onclick={() => showExportMenu = !showExportMenu}>
        Export
      </button>
      {#if showExportMenu}
        <div class="export-menu">
          <button onclick={() => handleExport('html_rdfa')}>HTML+RDFa</button>
          <button onclick={() => handleExport('json_ld')}>JSON-LD</button>
          <button onclick={() => handleExport('turtle')}>Turtle</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .suggestion-tray {
    height: var(--tray-height);
    background: var(--bg-tray);
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    padding: 0 12px;
    gap: 8px;
    font-size: var(--font-size-label);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .tray-separator {
    opacity: 0.4;
  }

  .tray-spacer {
    flex: 1;
  }

  .stale-link {
    background: none;
    border: none;
    color: #F59E0B;
    cursor: pointer;
    padding: 0;
    font-size: var(--font-size-label);
    font-family: var(--font-ui);
    text-decoration: underline;
    text-decoration-color: #F59E0B44;
  }

  .stale-link:hover {
    color: #FBBF24;
  }

  .mode-indicator {
    font-size: 10px;
    color: var(--text-muted);
    padding: 1px 6px;
    border-radius: 3px;
    background: #2A2A2A;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .export-wrapper {
    position: relative;
  }

  .tray-button {
    background: none;
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 1px 8px;
    border-radius: 3px;
    cursor: pointer;
    font-size: var(--font-size-label);
    font-family: var(--font-ui);
  }

  .tray-button:hover {
    color: var(--text-secondary);
    border-color: var(--text-muted);
  }

  .export-menu {
    position: absolute;
    bottom: 100%;
    right: 0;
    background: #1E1E1E;
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    padding: 4px 0;
    margin-bottom: 4px;
    min-width: 120px;
  }

  .export-menu button {
    display: block;
    width: 100%;
    padding: 4px 12px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: var(--font-size-label);
    font-family: var(--font-ui);
    cursor: pointer;
    text-align: left;
  }

  .export-menu button:hover {
    background: var(--border-subtle);
    color: var(--text-primary);
  }
</style>
