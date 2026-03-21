<script lang="ts">
  import { createEntity } from '$lib/tauri/commands';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { ENTITY_COLORS } from '$lib/theme/colors';

  interface Props {
    selectedText: string;
    spanStart: number;
    spanEnd: number;
    posX: number;
    posY: number;
    onClose: () => void;
    onCreated: () => void;
  }

  let { selectedText, spanStart, spanEnd, posX, posY, onClose, onCreated }: Props = $props();

  const entityTypes = [
    { label: 'Person', iri: 'http://schema.org/Person', prefix: 'schema:Person' },
    { label: 'Place', iri: 'http://schema.org/Place', prefix: 'schema:Place' },
    { label: 'Event', iri: 'http://schema.org/Event', prefix: 'schema:Event' },
    { label: 'Org', iri: 'http://schema.org/Organization', prefix: 'schema:Organization' },
    { label: 'Article', iri: 'http://schema.org/Article', prefix: 'schema:Article' },
    { label: 'Custom...', iri: '', prefix: '' },
  ];

  let customIri = $state('');
  let showCustomInput = $state(false);

  async function handleSelect(type: typeof entityTypes[0]) {
    if (type.iri === '') {
      showCustomInput = true;
      return;
    }

    const docId = getActiveDocId();
    if (!docId) return;

    try {
      await createEntity(docId, spanStart, spanEnd, type.iri);
      onCreated();
      onClose();
    } catch (e) {
      console.error('Failed to create entity:', e);
    }
  }

  async function handleCustomSubmit() {
    if (!customIri.trim()) return;
    const docId = getActiveDocId();
    if (!docId) return;

    try {
      await createEntity(docId, spanStart, spanEnd, customIri.trim());
      onCreated();
      onClose();
    } catch (e) {
      console.error('Failed to create entity:', e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
    if (showCustomInput && e.key === 'Enter') {
      handleCustomSubmit();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="entity-creator-backdrop" onclick={onClose}>
  <div
    class="entity-creator"
    style="left: {posX}px; top: {posY}px;"
    onclick|stopPropagation
  >
    <div class="selected-text">"{selectedText.length > 30 ? selectedText.slice(0, 30) + '...' : selectedText}"</div>

    {#if showCustomInput}
      <div class="custom-input-row">
        <input
          type="text"
          class="custom-iri-input"
          placeholder="http://schema.org/Thing"
          bind:value={customIri}
        />
        <button class="confirm-btn" onclick={handleCustomSubmit}>Create</button>
      </div>
    {:else}
      <div class="type-grid">
        {#each entityTypes as type}
          <button
            class="type-option"
            class:custom={type.iri === ''}
            onclick={() => handleSelect(type)}
          >
            {#if type.prefix}
              <span
                class="type-dot"
                style="background: {ENTITY_COLORS[type.prefix] ?? '#6B7280'}"
              ></span>
            {/if}
            {type.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .entity-creator-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
  }

  .entity-creator {
    position: fixed;
    background: #1E1E1E;
    border: 1px solid #333;
    border-radius: 8px;
    padding: 10px 12px;
    min-width: 240px;
    font-family: var(--font-ui, Inter, sans-serif);
    z-index: 1001;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  }

  .selected-text {
    color: #E5E5E5;
    font-size: 13px;
    margin-bottom: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid #333;
  }

  .type-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }

  .type-option {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: 1px solid transparent;
    color: #A3A3A3;
    padding: 5px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-family: var(--font-ui, Inter, sans-serif);
    text-align: left;
  }

  .type-option:hover {
    background: #2A2A2A;
    color: #E5E5E5;
    border-color: #444;
  }

  .type-option.custom {
    grid-column: span 2;
    justify-content: center;
    color: #737373;
    border-color: #333;
    margin-top: 4px;
  }

  .type-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .custom-input-row {
    display: flex;
    gap: 6px;
  }

  .custom-iri-input {
    flex: 1;
    background: #141414;
    border: 1px solid #444;
    color: #E5E5E5;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-family: var(--font-editor, monospace);
    outline: none;
  }

  .custom-iri-input:focus {
    border-color: #666;
  }

  .confirm-btn {
    background: #333;
    border: 1px solid #444;
    color: #E5E5E5;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-family: var(--font-ui, Inter, sans-serif);
  }

  .confirm-btn:hover {
    background: #444;
  }
</style>
