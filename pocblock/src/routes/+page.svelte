<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { EditorInstance } from '$lib/editor';

  let editorEl: HTMLElement;
  let instance: EditorInstance | null = null;
  let syncStatus = $state('connecting...');
  let agentStatus = $state('idle');

  onMount(async () => {
    const { createEditor } = await import('$lib/editor');
    instance = createEditor(editorEl);

    instance.provider.on('status', (e: { status: string }) => {
      syncStatus = e.status === 'connected' ? 'connected' : 'disconnected';
    });

    // Poll agent server health
    const healthInterval = setInterval(async () => {
      try {
        const res = await fetch('http://localhost:3001/health');
        agentStatus = res.ok ? 'connected' : 'disconnected';
      } catch {
        agentStatus = 'disconnected';
      }
    }, 5000);

    return () => clearInterval(healthInterval);
  });

  onDestroy(() => {
    instance?.destroy();
    instance = null;
  });
</script>

<div class="page">
  <header>
    <h1>Sparkdown Agent PoC</h1>
    <div class="status-bar">
      <span class="status" class:connected={syncStatus === 'connected'}>
        Sync: {syncStatus}
      </span>
      <span class="status" class:connected={agentStatus === 'connected'}>
        Agents: {agentStatus}
      </span>
    </div>
  </header>
  <main bind:this={editorEl} class="editor-container"></main>
</div>

<style>
  .page {
    max-width: 900px;
    margin: 0 auto;
    padding: 1rem;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0;
    border-bottom: 1px solid #e0e0e0;
    margin-bottom: 1rem;
  }

  header h1 {
    font-size: 1.2rem;
    font-weight: 600;
    margin: 0;
  }

  .status-bar {
    display: flex;
    gap: 1rem;
  }

  .status {
    font-size: 0.8rem;
    color: #888;
  }

  .status.connected {
    color: #2da44e;
  }

  .editor-container {
    min-height: 80vh;
  }

  /* Tiptap editor styling */
  :global(.tiptap-editor) {
    outline: none;
    padding: 1rem;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    font-size: 1rem;
    line-height: 1.6;
  }

  :global(.tiptap-editor p) {
    margin: 0.5em 0;
  }

  :global(.tiptap-editor h1) {
    font-size: 1.8em;
    margin-top: 1em;
  }

  :global(.tiptap-editor h2) {
    font-size: 1.4em;
    margin-top: 0.8em;
  }

  :global(.tiptap-editor h3) {
    font-size: 1.2em;
    margin-top: 0.6em;
  }

  :global(.tiptap-editor code) {
    background: #f0f0f0;
    padding: 0.2em 0.4em;
    border-radius: 3px;
    font-size: 0.9em;
  }

  :global(.tiptap-editor pre) {
    background: #1e1e1e;
    color: #d4d4d4;
    padding: 1em;
    border-radius: 6px;
    overflow-x: auto;
  }

  :global(.tiptap-editor blockquote) {
    border-left: 3px solid #ccc;
    padding-left: 1em;
    color: #666;
    margin: 0.5em 0;
  }
</style>
