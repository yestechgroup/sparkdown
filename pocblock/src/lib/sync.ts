import { WebsocketProvider } from 'y-websocket';
import type { Doc } from '@blocksuite/store';

const SYNC_URL = 'ws://localhost:4444';
const ROOM_NAME = 'sparkdown-poc';

export function createSyncProvider(doc: Doc): WebsocketProvider {
  // BlockSuite's Doc wraps a Yjs subdocument accessible via spaceDoc
  const ydoc = doc.spaceDoc;

  // Create with connect: false so we control when the WebSocket opens.
  // This prevents incoming Yjs updates from hitting the editor before
  // the full component tree is mounted.
  const provider = new WebsocketProvider(SYNC_URL, ROOM_NAME, ydoc, {
    connect: false,
  });

  provider.on('status', (event: { status: string }) => {
    console.log(`[sync] ${event.status}`);
  });

  provider.on('sync', (isSynced: boolean) => {
    console.log(`[sync] synced: ${isSynced}`);
  });

  return provider;
}
