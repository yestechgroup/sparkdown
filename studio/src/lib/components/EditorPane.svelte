<script lang="ts">
  import CodeMirrorEditor from './CodeMirrorEditor.svelte';
  import SuggestionTray from './SuggestionTray.svelte';
  import SuggestionRibbon from './SuggestionRibbon.svelte';
  import KnowledgePanel from './KnowledgePanel.svelte';
  import EntityCreator from './EntityCreator.svelte';
  import PropertyEditor from './PropertyEditor.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { getKnowledgePanelOpen, getEditorMode } from '$lib/stores/document.svelte';
  import type { EntityDto } from '$lib/tauri/commands';

  interface Props {
    initialContent?: string;
  }

  let { initialContent = '' }: Props = $props();
  let activeDocId = $derived(getActiveDocId());
  let knowledgePanelOpen = $derived(getKnowledgePanelOpen());
  let editorMode = $derived(getEditorMode());

  // Entity creator popup state
  let entityCreatorInfo = $state<{
    text: string;
    from: number;
    to: number;
    x: number;
    y: number;
  } | null>(null);

  // Property editor popup state
  let propertyEditorInfo = $state<{
    entity: EntityDto;
    x: number;
    y: number;
  } | null>(null);

  // Suggestion ribbon visibility
  let showRibbon = $state(false);

  function handleRequestEntityCreator(info: typeof entityCreatorInfo) {
    entityCreatorInfo = info;
  }

  function handleRequestPropertyEditor(info: typeof propertyEditorInfo) {
    propertyEditorInfo = info;
  }
</script>

{#if activeDocId}
  <div class="editor-pane" class:with-panel={knowledgePanelOpen}>
    <div class="editor-main">
      <CodeMirrorEditor
        {initialContent}
        onRequestEntityCreator={handleRequestEntityCreator}
        onRequestPropertyEditor={handleRequestPropertyEditor}
      />
      {#if showRibbon}
        <SuggestionRibbon onClose={() => (showRibbon = false)} />
      {/if}
      <SuggestionTray onShowRibbon={() => (showRibbon = !showRibbon)} />
    </div>

    {#if knowledgePanelOpen}
      <KnowledgePanel />
    {/if}
  </div>

  <!-- Floating popups -->
  {#if entityCreatorInfo}
    <EntityCreator
      selectedText={entityCreatorInfo.text}
      spanStart={entityCreatorInfo.from}
      spanEnd={entityCreatorInfo.to}
      posX={entityCreatorInfo.x}
      posY={entityCreatorInfo.y}
      onClose={() => (entityCreatorInfo = null)}
      onCreated={() => {}}
    />
  {/if}

  {#if propertyEditorInfo}
    <PropertyEditor
      entity={propertyEditorInfo.entity}
      posX={propertyEditorInfo.x}
      posY={propertyEditorInfo.y}
      onClose={() => (propertyEditorInfo = null)}
    />
  {/if}
{:else}
  <div class="empty-state">
    <p>Select a file to start editing</p>
  </div>
{/if}

<style>
  .editor-pane {
    flex: 1;
    display: flex;
    flex-direction: row;
    background: var(--bg-editor);
    overflow: hidden;
  }

  .editor-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
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
