<script lang="ts">
  import CodeMirrorEditor from './CodeMirrorEditor.svelte';
  import SuggestionTray from './SuggestionTray.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';

  interface Props {
    initialContent?: string;
  }

  let { initialContent = '' }: Props = $props();
  let activeDocId = $derived(getActiveDocId());
</script>

{#if activeDocId}
  <div class="editor-pane">
    <CodeMirrorEditor {initialContent} />
    <SuggestionTray />
  </div>
{:else}
  <div class="empty-state">
    <p>Select a file to start editing</p>
  </div>
{/if}

<style>
  .editor-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--bg-editor);
    overflow: hidden;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-editor);
    color: var(--text-muted);
  }
</style>
