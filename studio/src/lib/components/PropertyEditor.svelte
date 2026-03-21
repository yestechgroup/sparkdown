<script lang="ts">
  import { addTriple } from '$lib/tauri/commands';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { entityColor } from '$lib/theme/colors';
  import type { EntityDto } from '$lib/tauri/commands';

  interface Props {
    entity: EntityDto;
    posX: number;
    posY: number;
    onClose: () => void;
  }

  let { entity, posX, posY, onClose }: Props = $props();

  let predicateInput = $state('');
  let valueInput = $state('');

  const commonPredicates = [
    { label: 'name', iri: 'http://schema.org/name' },
    { label: 'description', iri: 'http://schema.org/description' },
    { label: 'url', iri: 'http://schema.org/url' },
    { label: 'startDate', iri: 'http://schema.org/startDate' },
    { label: 'endDate', iri: 'http://schema.org/endDate' },
    { label: 'location', iri: 'http://schema.org/location' },
    { label: 'email', iri: 'http://schema.org/email' },
  ];

  async function handleSubmit() {
    const docId = getActiveDocId();
    if (!docId || !predicateInput.trim() || !valueInput.trim()) return;

    // Resolve predicate: if it looks like a CURIE or full IRI, use as-is
    let predIri = predicateInput.trim();
    const match = commonPredicates.find((p) => p.label === predIri);
    if (match) predIri = match.iri;
    else if (!predIri.includes(':') && !predIri.includes('/')) {
      predIri = `http://schema.org/${predIri}`;
    }

    try {
      await addTriple(docId, entity.id, predIri, valueInput.trim(), false);
      predicateInput = '';
      valueInput = '';
      onClose();
    } catch (e) {
      console.error('Failed to add triple:', e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function selectPredicate(iri: string, label: string) {
    predicateInput = label;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="property-editor-backdrop" onclick={onClose}>
  <div
    class="property-editor"
    style="left: {posX}px; top: {posY}px;"
    onclick|stopPropagation
  >
    <div class="editor-header">
      <span
        class="entity-dot"
        style="background: {entityColor(entity.type_prefix)}"
      ></span>
      <strong>{entity.label}</strong>
      <span class="entity-type">{entity.type_prefix}</span>
    </div>

    <div class="quick-predicates">
      {#each commonPredicates as pred}
        <button
          class="predicate-chip"
          class:active={predicateInput === pred.label}
          onclick={() => selectPredicate(pred.iri, pred.label)}
        >
          {pred.label}
        </button>
      {/each}
    </div>

    <div class="input-row">
      <input
        type="text"
        class="pred-input"
        placeholder="property"
        bind:value={predicateInput}
      />
      <input
        type="text"
        class="val-input"
        placeholder="value"
        bind:value={valueInput}
      />
      <button class="add-btn" onclick={handleSubmit}>+</button>
    </div>
  </div>
</div>

<style>
  .property-editor-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
  }

  .property-editor {
    position: fixed;
    background: #1E1E1E;
    border: 1px solid #333;
    border-radius: 8px;
    padding: 10px 12px;
    min-width: 300px;
    font-family: var(--font-ui, Inter, sans-serif);
    z-index: 1001;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  }

  .editor-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
    font-size: 13px;
    color: #E5E5E5;
  }

  .entity-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .entity-type {
    color: #737373;
    font-size: 11px;
    margin-left: auto;
  }

  .quick-predicates {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    margin-bottom: 8px;
  }

  .predicate-chip {
    background: #2A2A2A;
    border: 1px solid transparent;
    color: #A3A3A3;
    padding: 2px 8px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 10px;
    font-family: var(--font-ui, Inter, sans-serif);
  }

  .predicate-chip:hover {
    color: #E5E5E5;
    border-color: #444;
  }

  .predicate-chip.active {
    background: #333;
    color: #E5E5E5;
    border-color: #555;
  }

  .input-row {
    display: flex;
    gap: 4px;
  }

  .pred-input, .val-input {
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

  .pred-input { max-width: 100px; }

  .pred-input:focus, .val-input:focus {
    border-color: #666;
  }

  .add-btn {
    background: #333;
    border: 1px solid #444;
    color: #E5E5E5;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    font-family: var(--font-ui, Inter, sans-serif);
  }

  .add-btn:hover {
    background: #444;
  }
</style>
