<script lang="ts">
  import FileTree from './FileTree.svelte';
  import { getWorkspacePath } from '$lib/stores/workspace.svelte';
  import { openWorkspace, listWorkspaceFiles } from '$lib/tauri/commands';
  import { setWorkspacePath, setFileList } from '$lib/stores/workspace.svelte';

  interface Props {
    onFileSelect: (path: string) => void;
  }

  let { onFileSelect }: Props = $props();
  let workspacePath = $derived(getWorkspacePath());

  async function handleOpenWorkspace() {
    try {
      const info = await openWorkspace();
      setWorkspacePath(info.path);
      setFileList(info.files);
    } catch (e) {
      console.error('Failed to open workspace:', e);
    }
  }

  async function handleRefresh() {
    if (workspacePath) {
      try {
        const files = await listWorkspaceFiles(workspacePath);
        setFileList(files);
      } catch (e) {
        console.error('Failed to refresh:', e);
      }
    }
  }
</script>

<aside class="sidebar">
  <div class="workspace-header">
    {#if workspacePath}
      <span class="workspace-name" title={workspacePath}>
        {workspacePath.split('/').pop()}
      </span>
      <button class="refresh-btn" onclick={handleRefresh} title="Refresh">↻</button>
    {:else}
      <button class="open-btn" onclick={handleOpenWorkspace}>Open Folder</button>
    {/if}
  </div>

  {#if workspacePath}
    <FileTree onSelect={onFileSelect} />
  {:else}
    <p class="empty-message">Open a folder to start</p>
  {/if}
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width);
    height: 100vh;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .workspace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .workspace-name {
    font-weight: 500;
    font-size: var(--font-size-ui);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .open-btn, .refresh-btn {
    background: none;
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: var(--font-size-ui);
  }

  .refresh-btn {
    border: none;
    padding: 4px;
    font-size: 16px;
  }

  .open-btn:hover, .refresh-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }

  .empty-message {
    padding: 16px 12px;
    color: var(--text-muted);
    font-size: var(--font-size-ui);
  }
</style>
