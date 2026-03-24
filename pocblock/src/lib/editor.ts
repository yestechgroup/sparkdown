import { Editor } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import Collaboration from '@tiptap/extension-collaboration';
import * as Y from 'yjs';
import { WebsocketProvider } from 'y-websocket';
import { AgentNote } from './extensions/agent-note';

const SYNC_URL = 'ws://localhost:4444';
const ROOM_NAME = 'sparkdown-poc';

export interface EditorInstance {
  editor: Editor;
  ydoc: Y.Doc;
  provider: WebsocketProvider;
  destroy: () => void;
}

export function createEditor(element: HTMLElement): EditorInstance {
  const ydoc = new Y.Doc();

  const provider = new WebsocketProvider(SYNC_URL, ROOM_NAME, ydoc, {
    connect: true,
  });

  const editor = new Editor({
    element,
    extensions: [
      StarterKit.configure({
        // Disable history — collaboration provides its own undo manager
        history: false,
      }),
      Collaboration.configure({
        document: ydoc,
      }),
      AgentNote,
    ],
    editorProps: {
      attributes: {
        class: 'tiptap-editor',
      },
    },
  });

  const destroy = () => {
    editor.destroy();
    provider.disconnect();
    ydoc.destroy();
  };

  return { editor, ydoc, provider, destroy };
}
