<script lang="ts">
    import { createEntity, listAvailableTypes, type TypeCategoryDto, type TypeOptionDto } from '$lib/tauri/commands';
    import { getActiveDocId } from '$lib/stores/workspace.svelte';
    import { entityColor } from '$lib/theme/colors';

    let { show = $bindable(false), charStart = 0, charEnd = 0, selectedText = '' } = $props();

    let categories = $state<TypeCategoryDto[]>([]);
    let searchQuery = $state('');
    let selectedIndex = $state(0);
    let loading = $state(false);
    let loaded = $state(false);

    let filteredTypes = $derived.by(() => {
        const query = searchQuery.toLowerCase();
        if (!query) return categories;
        return categories.map(cat => ({
            ...cat,
            types: cat.types.filter(t =>
                t.label.toLowerCase().includes(query) ||
                t.curie.toLowerCase().includes(query)
            )
        })).filter(cat => cat.types.length > 0);
    });

    let flatTypes = $derived(filteredTypes.flatMap(c => c.types));

    async function loadTypes() {
        if (loaded) return;
        loading = true;
        try {
            categories = await listAvailableTypes();
            loaded = true;
        } finally {
            loading = false;
        }
    }

    async function confirm(type_option: TypeOptionDto) {
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            await createEntity(docId, charStart, charEnd, type_option.iri);
            show = false;
        } catch (e) {
            console.error('Failed to create entity:', e);
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            show = false;
        } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            selectedIndex = Math.min(selectedIndex + 1, flatTypes.length - 1);
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            selectedIndex = Math.max(selectedIndex - 1, 0);
        } else if (e.key === 'Enter' && flatTypes[selectedIndex]) {
            e.preventDefault();
            confirm(flatTypes[selectedIndex]);
        }
    }

    $effect(() => {
        if (show) {
            loadTypes();
            searchQuery = '';
            selectedIndex = 0;
        }
    });
</script>

{#if show}
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="popup-overlay" onkeydown={handleKeydown}>
    <div class="popup">
        <div class="popup-header">"{selectedText}"</div>
        <input
            class="search-input"
            bind:value={searchQuery}
            placeholder="Search types..."
            autofocus
        />
        {#if loading}
            <div class="loading">Loading types...</div>
        {:else}
            <div class="type-list">
                {#each filteredTypes as category}
                    <div class="category-label">{category.pack_name}</div>
                    {#each category.types as typeOpt}
                        {@const globalIdx = flatTypes.indexOf(typeOpt)}
                        <button
                            class="type-option"
                            class:selected={globalIdx === selectedIndex}
                            onclick={() => confirm(typeOpt)}
                        >
                            <span class="type-dot" style="background: {entityColor(typeOpt.curie)}"></span>
                            <span class="type-label">{typeOpt.label}</span>
                            <span class="type-curie">{typeOpt.curie}</span>
                        </button>
                    {/each}
                {/each}
            </div>
        {/if}
    </div>
</div>
{/if}

<style>
    .popup-overlay {
        position: fixed; inset: 0; z-index: 100;
    }
    .popup {
        position: absolute; top: 50%; left: 50%;
        transform: translate(-50%, -50%);
        background: #1a1a1a; border: 1px solid #333; border-radius: 8px;
        width: 320px; max-height: 400px; overflow: hidden;
        display: flex; flex-direction: column;
    }
    .popup-header {
        padding: 12px 16px 4px; color: #ccc; font-size: 13px;
        font-style: italic;
    }
    .search-input {
        margin: 8px 16px; padding: 6px 10px;
        background: #0f0f0f; border: 1px solid #444; border-radius: 4px;
        color: #eee; font-size: 13px; outline: none;
    }
    .search-input:focus { border-color: #666; }
    .loading { padding: 16px; color: #888; text-align: center; }
    .type-list { overflow-y: auto; max-height: 280px; padding-bottom: 8px; }
    .category-label {
        padding: 8px 16px 4px; color: #666; font-size: 11px;
        text-transform: uppercase; letter-spacing: 0.5px;
    }
    .type-option {
        display: flex; align-items: center; gap: 8px;
        width: 100%; padding: 6px 16px;
        background: none; border: none; color: #ddd;
        font-size: 13px; cursor: pointer; text-align: left;
    }
    .type-option:hover, .type-option.selected { background: #2a2a2a; }
    .type-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
    .type-curie { color: #666; font-size: 11px; margin-left: auto; }
</style>
