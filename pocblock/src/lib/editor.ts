import { effects as stdEffects } from '@blocksuite/block-std/effects';
import { effects as blocksEffects } from '@blocksuite/blocks/effects';
import { effects as presetsEffects } from '@blocksuite/presets/effects';
stdEffects();
blocksEffects();
presetsEffects();

import { createEmptyDoc, PageEditor } from '@blocksuite/presets';
import { Text } from '@blocksuite/store';

export interface EditorInstance {
  doc: ReturnType<ReturnType<typeof createEmptyDoc>['init']>;
  editor: PageEditor;
}

export function createEditor(container: HTMLElement): EditorInstance {
  // createEmptyDoc sets up Schema + DocCollection + Doc + default block tree
  const doc = createEmptyDoc().init();

  // Create the page editor web component and mount it
  const editor = new PageEditor();
  editor.doc = doc;
  container.appendChild(editor);

  // Set initial placeholder text
  const paragraphs = doc.getBlockByFlavour('affine:paragraph');
  if (paragraphs.length > 0) {
    doc.updateBlock(paragraphs[0], { text: new Text('Start writing here...') });
  }

  return { doc, editor };
}
