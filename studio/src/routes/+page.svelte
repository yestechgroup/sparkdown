<script lang="ts">
  import '$lib/theme/tokens.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import EditorPane from '$lib/components/EditorPane.svelte';
  import KnowledgePanel from '$lib/components/KnowledgePanel.svelte';
  import { openDocument } from '$lib/tauri/commands';
  import { getActiveDocId, setActiveDocId } from '$lib/stores/workspace.svelte';
  import { clearDocumentState } from '$lib/stores/document.svelte';
  import { setupEventListeners, teardownEventListeners } from '$lib/stores/events';
  import { onMount } from 'svelte';
  import { readTextFile } from '@tauri-apps/plugin-fs';

  let fileContent = $state('');
  let activeDocId = $derived(getActiveDocId());

  onMount(() => {
    setupEventListeners();
    return teardownEventListeners;
  });

  async function handleFileSelect(path: string) {
    try {
      clearDocumentState();
      fileContent = await readTextFile(path);
      const docId = await openDocument(path);
      setActiveDocId(docId);
    } catch (e) {
      console.error('Failed to open document:', e);
    }
  }
</script>

<div class="app-layout">
  <Sidebar onFileSelect={handleFileSelect} />
  <EditorPane initialContent={fileContent} />
  {#if activeDocId}
    <KnowledgePanel />
  {/if}
</div>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
</style>
