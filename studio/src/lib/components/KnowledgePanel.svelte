<script lang="ts">
  import {
    getEntities,
    getSelectedEntityId,
    getSelectedEntityDetail,
    setSelectedEntityId,
    setSelectedEntityDetail,
    setKnowledgePanelOpen,
    getSidecarStatus,
  } from '$lib/stores/document.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { getEntityDetails } from '$lib/tauri/commands';
  import { entityColor, ENTITY_COLORS } from '$lib/theme/colors';

  let entities = $derived(getEntities());
  let selectedId = $derived(getSelectedEntityId());
  let detail = $derived(getSelectedEntityDetail());
  let activeDocId = $derived(getActiveDocId());
  let status = $derived(getSidecarStatus());

  // Fetch details when selection changes
  $effect(() => {
    if (selectedId && activeDocId) {
      getEntityDetails(activeDocId, selectedId)
        .then((d) => setSelectedEntityDetail(d))
        .catch(() => setSelectedEntityDetail(null));
    } else {
      setSelectedEntityDetail(null);
    }
  });

  function handleClose() {
    setKnowledgePanelOpen(false);
    setSelectedEntityId(null);
  }

  function handleSelectEntity(id: string) {
    setSelectedEntityId(id);
  }
</script>

<aside class="knowledge-panel">
  <div class="panel-header">
    {#if detail}
      <span class="panel-title">{detail.label}</span>
    {:else}
      <span class="panel-title">Document Overview</span>
    {/if}
    <button class="close-btn" onclick={handleClose} title="Close panel">&times;</button>
  </div>

  <div class="panel-body">
    {#if detail}
      <!-- Entity Deep Dive -->
      <div class="entity-header">
        <span
          class="entity-dot"
          style="background: {entityColor(detail.type_prefix)}"
        ></span>
        <span class="entity-type">{detail.type_prefix}</span>
        <span class="entity-status" class:stale={detail.status === 'stale'} class:detached={detail.status === 'detached'}>
          {detail.status}
        </span>
      </div>

      {#if detail.properties.length > 0}
        <div class="section">
          <div class="section-header">Properties</div>
          {#each detail.properties as prop}
            <div class="property-row">
              <span class="prop-label">{prop.predicate_label}</span>
              <span class="prop-value">{prop.value}</span>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Outgoing relations from top_relations -->
      {#if entities.find(e => e.id === detail.id)?.top_relations?.length}
        <div class="section">
          <div class="section-header">Relations</div>
          {#each entities.find(e => e.id === detail.id)?.top_relations ?? [] as rel}
            <div class="relation-row">
              <span class="rel-predicate">{rel.predicate_label}</span>
              <span class="rel-arrow">&rarr;</span>
              {#if rel.target_id}
                <button class="rel-target clickable" onclick={() => handleSelectEntity(rel.target_id)}>
                  {rel.target_label}
                </button>
              {:else}
                <span class="rel-target">{rel.target_label}</span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if detail.incoming_relations.length > 0}
        <div class="section">
          <div class="section-header">Referenced by</div>
          {#each detail.incoming_relations as rel}
            <div class="relation-row">
              <button class="rel-target clickable" onclick={() => handleSelectEntity(rel.subject_id)}>
                {rel.subject_label}
              </button>
              <span class="rel-arrow">&rarr;</span>
              <span class="rel-predicate">{rel.predicate_label}</span>
            </div>
          {/each}
        </div>
      {/if}

    {:else}
      <!-- Document Overview -->
      <div class="section">
        <div class="section-header">Sidecar Status</div>
        <div class="status-grid">
          <span class="status-label">Synced</span><span class="status-value">{status.synced}</span>
          <span class="status-label">Stale</span><span class="status-value stale">{status.stale}</span>
          <span class="status-label">Detached</span><span class="status-value detached">{status.detached}</span>
          <span class="status-label">Triples</span><span class="status-value">{status.total_triples}</span>
        </div>
      </div>

      {#if entities.length > 0}
        <div class="section">
          <div class="section-header">Entities ({entities.length})</div>
          <div class="entity-list">
            {#each entities as entity}
              <button class="entity-item" onclick={() => handleSelectEntity(entity.id)}>
                <span
                  class="entity-dot"
                  style="background: {entityColor(entity.type_prefix)}"
                ></span>
                <span class="entity-label">{entity.label}</span>
                <span class="entity-type-small">{entity.type_prefix.split(':')[1] ?? entity.type_prefix}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</aside>

<style>
  .knowledge-panel {
    width: 280px;
    height: 100%;
    background: var(--bg-sidebar, #141414);
    border-left: 1px solid var(--border-subtle, #2A2A2A);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex-shrink: 0;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle, #2A2A2A);
  }

  .panel-title {
    font-weight: 500;
    font-size: 13px;
    color: var(--text-primary, #E5E5E5);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted, #737373);
    font-size: 18px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
  }

  .close-btn:hover {
    color: var(--text-primary, #E5E5E5);
  }

  .panel-body {
    padding: 8px 12px;
    flex: 1;
    overflow-y: auto;
  }

  .entity-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 12px;
  }

  .entity-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .entity-type {
    font-size: 11px;
    color: var(--text-secondary, #A3A3A3);
  }

  .entity-status {
    font-size: 10px;
    color: var(--text-muted, #737373);
    margin-left: auto;
    padding: 1px 6px;
    border-radius: 3px;
    background: #2A2A2A;
  }

  .entity-status.stale {
    color: #F59E0B;
    background: #F59E0B1a;
  }

  .entity-status.detached {
    color: #F43F5E;
    background: #F43F5E1a;
  }

  .section {
    margin-bottom: 12px;
  }

  .section-header {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted, #737373);
    margin-bottom: 6px;
    padding-bottom: 4px;
    border-bottom: 1px solid #222;
  }

  .property-row {
    display: flex;
    justify-content: space-between;
    padding: 3px 0;
    font-size: 11px;
  }

  .prop-label {
    color: var(--text-muted, #737373);
  }

  .prop-value {
    color: var(--text-secondary, #A3A3A3);
    text-align: right;
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .relation-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 0;
    font-size: 11px;
  }

  .rel-predicate {
    color: var(--text-muted, #737373);
  }

  .rel-arrow {
    color: #444;
  }

  .rel-target {
    color: var(--text-secondary, #A3A3A3);
  }

  .rel-target.clickable {
    background: none;
    border: none;
    color: #8B5CF6;
    cursor: pointer;
    padding: 0;
    font-size: 11px;
    font-family: inherit;
    text-decoration: underline;
    text-decoration-color: #8B5CF644;
  }

  .rel-target.clickable:hover {
    color: #A78BFA;
  }

  .status-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    font-size: 11px;
  }

  .status-label {
    color: var(--text-muted, #737373);
  }

  .status-value {
    color: var(--text-secondary, #A3A3A3);
    text-align: right;
  }

  .status-value.stale { color: #F59E0B; }
  .status-value.detached { color: #F43F5E; }

  .entity-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .entity-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
    border-radius: 4px;
    background: none;
    border: none;
    color: var(--text-secondary, #A3A3A3);
    cursor: pointer;
    font-size: 11px;
    font-family: inherit;
    text-align: left;
  }

  .entity-item:hover {
    background: #2A2A2A;
    color: var(--text-primary, #E5E5E5);
  }

  .entity-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .entity-type-small {
    color: var(--text-muted, #737373);
    font-size: 10px;
    flex-shrink: 0;
  }
</style>
