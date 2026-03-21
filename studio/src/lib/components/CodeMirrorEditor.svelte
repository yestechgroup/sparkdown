<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { markdown } from '@codemirror/lang-markdown';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { updateSource, saveDocument, checkFileModified } from '$lib/tauri/commands';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import {
    getEntities,
    getStaleAnchors,
    getEditorMode,
    setSelectedEntityId,
    setKnowledgePanelOpen,
    setEditorMode,
  } from '$lib/stores/document.svelte';
  import { entitiesField, setEntitiesEffect, semanticGutter } from '$lib/editor/semantic-gutter';
  import { entityDecorations } from '$lib/editor/entity-decorations';
  import { createWhisperTooltip } from '$lib/editor/whisper-tooltip';
  import {
    staleAnchorsField,
    setStaleAnchorsEffect,
    createStaleNudgePlugin,
  } from '$lib/editor/stale-anchor-nudge';
  import { createModeTransitionPlugin } from '$lib/editor/mode-transitions';
  import type { EntityDto } from '$lib/tauri/commands';

  interface Props {
    initialContent?: string;
    onRequestEntityCreator?: (info: {
      text: string;
      from: number;
      to: number;
      x: number;
      y: number;
    }) => void;
    onRequestPropertyEditor?: (info: {
      entity: EntityDto;
      x: number;
      y: number;
    }) => void;
  }

  let {
    initialContent = '',
    onRequestEntityCreator,
    onRequestPropertyEditor,
  }: Props = $props();

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
    const stale = getStaleAnchors();
    if (view) {
      view.dispatch({
        effects: setStaleAnchorsEffect.of(stale),
      });
    }
  });

  function handleOpenEntity(entityId: string) {
    setSelectedEntityId(entityId);
    setKnowledgePanelOpen(true);
  }

  function handleStaleAccept(entityId: string) {
    const docId = getActiveDocId();
    if (docId) {
      import('$lib/tauri/commands').then(({ updateStaleAnchor }) => {
        updateStaleAnchor(docId, entityId).catch(console.error);
      });
    }
  }

  function handleStaleDismiss(entityId: string) {
    const docId = getActiveDocId();
    if (docId) {
      import('$lib/tauri/commands').then(({ dismissSuggestion }) => {
        dismissSuggestion(docId, entityId).catch(console.error);
      });
    }
  }

  onMount(() => {
    const state = EditorState.create({
      doc: initialContent,
      extensions: [
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
          {
            key: 'Mod-s',
            run: (v) => {
              const docId = getActiveDocId();
              if (docId) {
                // Phase 2: check for external modifications before saving
                checkFileModified(docId)
                  .then((modified) => {
                    if (modified) {
                      console.warn('[Sparkdown] File was externally modified since last save');
                      // Still save, but warn via console (future: show UI notification)
                    }
                    return saveDocument(docId);
                  })
                  .catch(console.error);
              }
              return true;
            },
          },
          {
            // Cmd+E: Quick entity creation
            key: 'Mod-e',
            run: (v) => {
              const sel = v.state.selection.main;
              if (sel.empty || !onRequestEntityCreator) return false;

              const text = v.state.sliceDoc(sel.from, sel.to);
              const coords = v.coordsAtPos(sel.from);

              if (coords) {
                onRequestEntityCreator({
                  text,
                  from: sel.from,
                  to: sel.to,
                  x: coords.left,
                  y: coords.bottom + 4,
                });
              }
              return true;
            },
          },
          {
            // Cmd+K: Inline property editor
            key: 'Mod-k',
            run: (v) => {
              if (!onRequestPropertyEditor) return false;

              const pos = v.state.selection.main.head;
              const entities = v.state.field(entitiesField);
              const entity = entities.find(
                (e) => pos >= e.span_start && pos < e.span_end,
              );

              if (!entity) return false;

              const coords = v.coordsAtPos(pos);
              if (coords) {
                onRequestPropertyEditor({
                  entity,
                  x: coords.left,
                  y: coords.bottom + 4,
                });
              }
              return true;
            },
          },
          {
            // Cmd+Shift+R: Toggle full reading mode
            key: 'Mod-Shift-r',
            run: () => {
              // getEditorMode is already imported at top level
              const current = getEditorMode();
              if (current === 'full-reading') {
                setEditorMode('deep-writing');
                setKnowledgePanelOpen(false);
              } else {
                setEditorMode('full-reading');
                setKnowledgePanelOpen(true);
              }
              return true;
            },
          },
        ]),
        history(),
        markdown(),
        oneDark,
        entitiesField,
        staleAnchorsField,
        semanticGutter,
        entityDecorations,
        createWhisperTooltip(handleOpenEntity),
        createStaleNudgePlugin(handleStaleAccept, handleStaleDismiss),
        createModeTransitionPlugin((mode) => {
          setEditorMode(mode);
          if (mode === 'review') {
            setKnowledgePanelOpen(true);
          } else if (mode === 'deep-writing') {
            // Don't auto-close panel on typing — let user control it
          }
        }),
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
