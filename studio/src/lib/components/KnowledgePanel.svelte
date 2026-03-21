<script lang="ts">
    import { getDocumentOverview, getEntityDetail, type DocumentOverviewDto, type EntityDetailDto } from '$lib/tauri/commands';
    import { getActiveDocId } from '$lib/stores/workspace.svelte';
    import { getEntities, getSelectedEntityId, setSelectedEntityId, setEntityDetail, getEntityDetailState } from '$lib/stores/document.svelte';
    import EntityList from './EntityList.svelte';
    import EntityDetail from './EntityDetail.svelte';

    let collapsed = $state(false);
    let overview = $state<DocumentOverviewDto | null>(null);
    let detail = $derived(getEntityDetailState());
    let selectedId = $derived(getSelectedEntityId());
    let entities = $derived(getEntities());

    async function loadOverview() {
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            overview = await getDocumentOverview(docId);
        } catch (e) {
            console.error('Failed to load overview:', e);
        }
    }

    async function selectEntity(id: string) {
        setSelectedEntityId(id);
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            const detailData = await getEntityDetail(docId, id);
            setEntityDetail(detailData);
        } catch (e) {
            console.error('Failed to load entity detail:', e);
        }
    }

    function goBack() {
        setSelectedEntityId(null);
        setEntityDetail(null);
    }

    // Refresh when entities change
    $effect(() => {
        entities; // subscribe to entity changes
        loadOverview();
    });
</script>

<div class="knowledge-panel" class:collapsed>
    <button class="collapse-toggle" onclick={() => collapsed = !collapsed}>
        {collapsed ? '\u25C0' : '\u25B6'}
    </button>

    {#if !collapsed}
        <div class="panel-content">
            <div class="panel-header">Knowledge</div>

            {#if selectedId && detail}
                <EntityDetail
                    {detail}
                    onBack={goBack}
                    onNavigate={selectEntity}
                />
            {:else}
                <EntityList
                    entities={overview?.entities ?? []}
                    onSelect={selectEntity}
                />
                {#if overview}
                    <div class="summary">
                        {overview.sidecar_status.total_triples} triples &middot;
                        {overview.sidecar_status.synced} synced
                        {#if overview.sidecar_status.stale > 0}
                            &middot; {overview.sidecar_status.stale} stale
                        {/if}
                    </div>
                {/if}
            {/if}
        </div>
    {:else}
        <div class="collapsed-label">
            {entities.length}
        </div>
    {/if}
</div>

<style>
    .knowledge-panel {
        width: 280px; min-width: 280px;
        background: #1a1a1a; border-left: 1px solid #333;
        display: flex; flex-direction: column;
        position: relative;
    }
    .knowledge-panel.collapsed { width: 24px; min-width: 24px; }
    .collapse-toggle {
        position: absolute; left: -12px; top: 8px;
        width: 24px; height: 24px; border-radius: 50%;
        background: #2a2a2a; border: 1px solid #444;
        color: #888; cursor: pointer; font-size: 10px;
        display: flex; align-items: center; justify-content: center;
        z-index: 10;
    }
    .collapse-toggle:hover { background: #333; color: #ddd; }
    .panel-content { flex: 1; overflow-y: auto; }
    .panel-header {
        padding: 12px; color: #888; font-size: 11px;
        text-transform: uppercase; letter-spacing: 1px;
        border-bottom: 1px solid #333;
    }
    .summary {
        padding: 12px; color: #555; font-size: 11px;
        border-top: 1px solid #333;
    }
    .collapsed-label {
        writing-mode: vertical-lr; text-align: center;
        padding: 12px 4px; color: #666; font-size: 12px;
    }
</style>
