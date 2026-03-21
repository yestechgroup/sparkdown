<script lang="ts">
    import type { EntityDetailDto } from '$lib/tauri/commands';
    import { deleteEntity } from '$lib/tauri/commands';
    import { getActiveDocId } from '$lib/stores/workspace.svelte';
    import { entityColor } from '$lib/theme/colors';

    let { detail, onBack, onNavigate }:
        { detail: EntityDetailDto, onBack: () => void, onNavigate: (entityId: string) => void } = $props();

    async function handleDelete() {
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            await deleteEntity(docId, detail.entity.id);
            onBack();
        } catch (e) {
            console.error('Failed to delete entity:', e);
        }
    }
</script>

<div class="entity-detail">
    <button class="back-btn" onclick={onBack}>&larr; Back</button>

    <div class="header">
        <span class="dot" style="background: {entityColor(detail.entity.type_prefix)}"></span>
        <div>
            <div class="name">{detail.entity.label}</div>
            <div class="type">{detail.entity.type_prefix}</div>
        </div>
        <span class="status status-{detail.entity.status}">{detail.entity.status}</span>
    </div>

    <div class="section">
        <div class="section-label">Anchor</div>
        <div class="snippet">"{detail.anchor_snippet}" &mdash; line {detail.anchor_line}</div>
    </div>

    {#if detail.all_relations.length > 0}
        <div class="section">
            <div class="section-label">Relations</div>
            {#each detail.all_relations as rel}
                <div class="relation">
                    <span class="pred">{rel.predicate_label}</span> &rarr;
                    {#if rel.target_id}
                        <button class="link" onclick={() => onNavigate(rel.target_id)}>{rel.target_label}</button>
                    {:else}
                        <span class="literal">{rel.target_label}</span>
                    {/if}
                </div>
            {/each}
        </div>
    {/if}

    {#if detail.incoming_relations.length > 0}
        <div class="section">
            <div class="section-label">Referenced by</div>
            {#each detail.incoming_relations as rel}
                <div class="relation">
                    <span class="pred">{rel.predicate_label}</span> &larr;
                    <button class="link" onclick={() => onNavigate(rel.target_id)}>{rel.target_label}</button>
                </div>
            {/each}
        </div>
    {/if}

    <div class="actions">
        <button class="delete-btn" onclick={handleDelete}>Delete entity</button>
    </div>
</div>

<style>
    .entity-detail { display: flex; flex-direction: column; gap: 12px; padding: 12px; }
    .back-btn { background: none; border: none; color: #888; cursor: pointer; text-align: left; padding: 0; font-size: 12px; }
    .back-btn:hover { color: #ddd; }
    .header { display: flex; align-items: center; gap: 8px; }
    .dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
    .name { color: #eee; font-size: 14px; font-weight: 500; }
    .type { color: #888; font-size: 12px; }
    .status { font-size: 10px; color: #666; margin-left: auto; }
    .status-stale { color: #F59E0B; }
    .status-detached { color: #F43F5E; }
    .section-label { color: #555; font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px; }
    .snippet { color: #aaa; font-size: 12px; font-style: italic; }
    .relation { font-size: 13px; color: #ccc; padding: 2px 0; }
    .pred { color: #888; }
    .link { background: none; border: none; color: #8B5CF6; cursor: pointer; padding: 0; font-size: 13px; }
    .link:hover { text-decoration: underline; }
    .literal { color: #aaa; }
    .actions { margin-top: 12px; padding-top: 12px; border-top: 1px solid #333; }
    .delete-btn {
        background: none; border: 1px solid #F43F5E33; color: #F43F5E; border-radius: 4px;
        padding: 4px 12px; font-size: 12px; cursor: pointer;
    }
    .delete-btn:hover { background: #F43F5E11; }
</style>
