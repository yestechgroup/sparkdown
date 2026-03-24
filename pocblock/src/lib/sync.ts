import { WebsocketProvider } from 'y-websocket';
import type { Doc } from '@blocksuite/store';

const SYNC_URL = 'ws://localhost:4444';
const ROOM_NAME = 'sparkdown-poc';

export function connectSync(doc: Doc): WebsocketProvider {
  // BlockSuite's Doc wraps a Yjs subdocument accessible via spaceDoc
  const ydoc = doc.spaceDoc;

  const provider = new WebsocketProvider(SYNC_URL, ROOM_NAME, ydoc);

  provider.on('status', (event: { status: string }) => {
    console.log(`[sync] ${event.status}`);
  });

  provider.on('sync', (isSynced: boolean) => {
    console.log(`[sync] synced: ${isSynced}`);
  });

  return provider;
}
