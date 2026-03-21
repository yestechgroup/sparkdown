<script lang="ts">
  import CodeMirrorEditor from './CodeMirrorEditor.svelte';
  import SuggestionTray from './SuggestionTray.svelte';
  import EntityCreationPopup from './EntityCreationPopup.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';

  interface Props {
    initialContent?: string;
  }

  let { initialContent = '' }: Props = $props();
  let activeDocId = $derived(getActiveDocId());

  let showEntityPopup = $state(false);
  let entityPopupStart = $state(0);
  let entityPopupEnd = $state(0);
  let entityPopupText = $state('');

  function handleCreateEntity(from: number, to: number, text: string) {
    entityPopupStart = from;
    entityPopupEnd = to;
    entityPopupText = text;
    showEntityPopup = true;
  }
</script>

{#if activeDocId}
  <div class="editor-pane">
    <CodeMirrorEditor {initialContent} onCreateEntity={handleCreateEntity} />
    <SuggestionTray />
  </div>

  <EntityCreationPopup
    bind:show={showEntityPopup}
    charStart={entityPopupStart}
    charEnd={entityPopupEnd}
    selectedText={entityPopupText}
  />
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
