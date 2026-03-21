<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { markdown } from '@codemirror/lang-markdown';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { updateSource, saveDocument } from '$lib/tauri/commands';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { getEntities } from '$lib/stores/document.svelte';
  import { entitiesField, setEntitiesEffect, semanticGutter } from '$lib/editor/semantic-gutter';
  import { entityDecorations } from '$lib/editor/entity-decorations';
  import { whisperTooltip } from '$lib/editor/whisper-tooltip';

  interface Props {
    initialContent?: string;
  }

  let { initialContent = '' }: Props = $props();
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
        ]),
        history(),
        markdown(),
        oneDark,
        entitiesField,
        semanticGutter,
        entityDecorations,
        whisperTooltip,
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
</style>
