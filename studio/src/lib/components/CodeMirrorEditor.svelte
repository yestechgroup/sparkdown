<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { markdown } from '@codemirror/lang-markdown';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { updateSource, saveDocument, updateStaleAnchor } from '$lib/tauri/commands';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { getEntities, getVisibleStaleAnchors, dismissStaleAnchor } from '$lib/stores/document.svelte';
  import { entitiesField, setEntitiesEffect, semanticGutter } from '$lib/editor/semantic-gutter';
  import { entityDecorations } from '$lib/editor/entity-decorations';
  import { whisperTooltip } from '$lib/editor/whisper-tooltip';
  import { staleAnchorWidgets, setStaleAnchorsEffect } from '$lib/editor/stale-anchor-widget';

  interface Props {
    initialContent?: string;
    onCreateEntity?: (from: number, to: number, text: string) => void;
  }

  let { initialContent = '', onCreateEntity }: Props = $props();
  let editorContainer: HTMLDivElement;
  let view: EditorView;
  let debounceTimer: ReturnType<typeof setTimeout>;

  // Push entity updates from Svelte state into CodeMirror
  $effect(() => {
    const current = getEntities();
    if (view) {
      view.dispatch({
        effects: setEntitiesEffect.of(current),
      });
    }
  });

  // Push stale anchor updates into CodeMirror
  $effect(() => {
    const anchors = getVisibleStaleAnchors();
    if (view) {
      view.dispatch({
        effects: setStaleAnchorsEffect.of(anchors),
      });
    }
  });

  onMount(() => {
    const state = EditorState.create({
      doc: initialContent,
      extensions: [
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
          {
            key: 'Mod-s',
            run: () => {
              const docId = getActiveDocId();
              if (docId) {
                saveDocument(docId).catch(console.error);
              }
              return true;
            },
          },
          {
            key: 'Mod-e',
            run: (view) => {
              const sel = view.state.selection.main;
              if (sel.from === sel.to) return false;
              const text = view.state.sliceDoc(sel.from, sel.to);
              onCreateEntity?.(sel.from, sel.to, text);
              return true;
            },
          },
        ]),
        history(),
        markdown(),
        oneDark,
        entitiesField,
        semanticGutter,
        entityDecorations,
        whisperTooltip,
        staleAnchorWidgets(
          async (entityId) => {
            const docId = getActiveDocId();
            if (docId) await updateStaleAnchor(docId, entityId);
          },
          (entityId) => {
            dismissStaleAnchor(entityId);
          },
        ),
        EditorView.theme({
          '&': {
            height: '100%',
            fontSize: 'var(--font-size-editor)',
            fontFamily: 'var(--font-editor)',
          },
          '.cm-content': {
            padding: '16px',
          },
          '.cm-scroller': {
            overflow: 'auto',
          },
          '.cm-semantic-gutter': {
            width: '12px',
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => {
              const docId = getActiveDocId();
              if (docId) {
                const source = update.state.doc.toString();
                updateSource(docId, source).catch(console.error);
              }
            }, 150);
          }
        }),
      ],
    });

    view = new EditorView({
      state,
      parent: editorContainer,
    });

    return () => {
      clearTimeout(debounceTimer);
      view.destroy();
    };
  });

  export function setContent(content: string) {
    if (view) {
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: content,
        },
      });
    }
  }
</script>

<div class="editor-wrapper" bind:this={editorContainer}></div>

<style>
  .editor-wrapper {
    flex: 1;
    overflow: hidden;
  }

  .editor-wrapper :global(.cm-editor) {
    height: 100%;
  }

  .editor-wrapper :global(.stale-anchor-widget) {
    display: flex; align-items: center; gap: 8px;
    padding: 4px 12px; margin: 2px 0;
    background: rgba(245, 158, 11, 0.1);
    border-left: 2px solid #F59E0B;
    font-size: 12px; color: #aaa;
  }
  .editor-wrapper :global(.stale-btn) {
    padding: 2px 8px; border: 1px solid #444;
    border-radius: 3px; background: #222; color: #ddd;
    cursor: pointer; font-size: 11px;
  }
  .editor-wrapper :global(.stale-btn:hover) { background: #333; }
  .editor-wrapper :global(.stale-accept:hover) { border-color: #22C55E; }
  .editor-wrapper :global(.stale-dismiss:hover) { border-color: #F43F5E; }
</style>
