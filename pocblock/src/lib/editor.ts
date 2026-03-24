import '@blocksuite/presets/effects';

import { AffineEditorContainer, createEmptyDoc } from '@blocksuite/presets';

export interface EditorInstance {
  doc: ReturnType<ReturnType<typeof createEmptyDoc>['init']>;
  editor: AffineEditorContainer;
}

export function createEditor(container: HTMLElement): EditorInstance {
  // createEmptyDoc sets up Schema + DocCollection + Doc
  const { doc, init } = createEmptyDoc();
  const loadedDoc = init();

  // Create the editor web component and mount it
  const editor = new AffineEditorContainer();
  editor.doc = loadedDoc;
  editor.mode = 'page';
  container.appendChild(editor);

  return { doc: loadedDoc, editor };
}
