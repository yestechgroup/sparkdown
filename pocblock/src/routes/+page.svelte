<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { EditorInstance } from '$lib/editor';

  let container: HTMLElement;
  let editorInstance: EditorInstance | null = null;
  let syncStatus = $state('connecting...');
  let agentStatus = $state('idle');

  onMount(async () => {
    // Dynamic import — BlockSuite needs the DOM
    const { createEditor } = await import('$lib/editor');
    const { connectSync } = await import('$lib/sync');

    editorInstance = createEditor(container);

    // Wait for the editor to finish its initial Lit render before connecting
    // Yjs sync — otherwise incoming updates trigger connectedCallback on
    // block components whose host isn't initialised yet.
    await editorInstance.editor.updateComplete;
    const provider = connectSync(editorInstance.doc);

    // Update status indicator
    provider.on('status', (e: { status: string }) => {
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
    editorInstance = null;
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
  <main bind:this={container} class="editor-container"></main>
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
</style>
