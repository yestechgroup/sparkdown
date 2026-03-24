import { effects as blocksEffects } from '@blocksuite/blocks/effects';
import { effects as presetsEffects } from '@blocksuite/presets/effects';
blocksEffects();
presetsEffects();

import { AffineEditorContainer, createEmptyDoc } from '@blocksuite/presets';
import { Text } from '@blocksuite/store';

export interface EditorInstance {
  doc: ReturnType<ReturnType<typeof createEmptyDoc>['init']>;
  editor: AffineEditorContainer;
}

export function createEditor(container: HTMLElement): EditorInstance {
  // createEmptyDoc sets up Schema + DocCollection + Doc + default block tree
  const doc = createEmptyDoc().init();

  // AffineEditorContainer wraps PageEditor with the viewport element
  // that BlockSuite components require for scroll/position tracking
  const editor = new AffineEditorContainer();
  editor.doc = doc;
  container.appendChild(editor);

  // Set initial placeholder text
  const paragraphs = doc.getBlockByFlavour('affine:paragraph');
  if (paragraphs.length > 0) {
    doc.updateBlock(paragraphs[0], { text: new Text('Start writing here...') });
  }

  return { doc, editor };
}
