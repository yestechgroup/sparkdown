<script lang="ts">
    import type { EntityDto } from '$lib/tauri/commands';
    import { entityColor } from '$lib/theme/colors';

    let { entities = [], onSelect }: { entities: EntityDto[], onSelect: (id: string) => void } = $props();

    // Group entities by type_prefix
    let grouped = $derived.by(() => {
        const groups = new Map<string, EntityDto[]>();
        for (const e of entities) {
            const key = e.type_prefix || 'Unknown';
            if (!groups.has(key)) groups.set(key, []);
            groups.get(key)!.push(e);
        }
        return groups;
    });
</script>

<div class="entity-list">
    {#each [...grouped.entries()] as [type_prefix, group]}
        <div class="type-group-label">{type_prefix}</div>
        {#each group as entity}
            <button class="entity-row" onclick={() => onSelect(entity.id)}>
                <span class="dot" style="background: {entityColor(entity.type_prefix)}"></span>
                <span class="label">{entity.label}</span>
                <span class="status status-{entity.status}">{entity.status}</span>
            </button>
        {/each}
    {/each}
    {#if entities.length === 0}
        <div class="empty">No entities yet. Select text and press Cmd+E to create one.</div>
    {/if}
</div>

<style>
    .entity-list { display: flex; flex-direction: column; }
    .type-group-label {
        padding: 8px 12px 4px; color: #666; font-size: 11px;
        text-transform: uppercase; letter-spacing: 0.5px;
    }
    .entity-row {
        display: flex; align-items: center; gap: 8px;
        padding: 6px 12px; background: none; border: none;
        color: #ddd; font-size: 13px; cursor: pointer; text-align: left;
    }
    .entity-row:hover { background: #2a2a2a; }
    .dot { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
    .label { flex: 1; }
    .status { font-size: 10px; color: #666; }
    .status-stale { color: #F59E0B; }
    .status-detached { color: #F43F5E; }
    .empty { padding: 16px 12px; color: #555; font-size: 13px; }
</style>
