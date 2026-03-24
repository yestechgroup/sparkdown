import type { Doc } from '@blocksuite/store';
import { AffineEditorContainer, createEmptyDoc } from '@blocksuite/presets';
import { effects as presetEffects } from '@blocksuite/presets/effects';

// Register all web components (must be called once)
presetEffects();

export interface EditorInstance {
  doc: Doc;
  editor: AffineEditorContainer;
}

export function createEditor(container: HTMLElement): EditorInstance {
  // Use the built-in helper which handles Schema, DocCollection, etc.
  const { doc, init } = createEmptyDoc();
  init();

  // Create the editor web component
  const editor = new AffineEditorContainer();
  editor.doc = doc;
  container.appendChild(editor);

  return { doc, editor };
}
