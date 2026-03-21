<script lang="ts">
  import { getFileList, getActiveDocId } from '$lib/stores/workspace.svelte';

  interface Props {
    onSelect: (path: string) => void;
  }

  let { onSelect }: Props = $props();
  let fileList = $derived(getFileList());
  let activeDocId = $derived(getActiveDocId());
</script>

<ul class="file-tree">
  {#each fileList as file}
    <li class="file-entry" class:active={activeDocId?.endsWith(file.path)}>
      <button onclick={() => onSelect(file.path)}>
        <span class="file-name">{file.name}</span>
        {#if file.has_sidecar}
          <span class="sidecar-indicator" title="Has semantic sidecar">●</span>
        {/if}
      </button>
    </li>
  {/each}
</ul>

<style>
  .file-tree {
    list-style: none;
    padding: 0;
  }

  .file-entry button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 4px 12px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-ui);
    cursor: pointer;
    text-align: left;
  }

  .file-entry button:hover {
    background: var(--border-subtle);
    color: var(--text-primary);
  }

  .file-entry.active button {
    background: var(--border-subtle);
    color: var(--text-primary);
  }

  .sidecar-indicator {
    color: #8B5CF6;
    font-size: 8px;
  }
</style>
