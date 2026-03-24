import http from 'node:http';
import { WebSocketServer } from 'ws';
import * as Y from 'yjs';
import * as syncProtocol from 'y-protocols/sync';
import * as awarenessProtocol from 'y-protocols/awareness';
import * as encoding from 'lib0/encoding';
import * as decoding from 'lib0/decoding';

const PORT = parseInt(process.env.PORT || '4444');
const HOST = process.env.HOST || 'localhost';
const CALLBACK_URL = process.env.CALLBACK_URL || 'http://localhost:3001/on-doc-update';
const CALLBACK_DEBOUNCE = parseInt(process.env.CALLBACK_DEBOUNCE_WAIT || '2000');

const MSG_SYNC = 0;
const MSG_AWARENESS = 1;

/** @type {Map<string, { doc: Y.Doc, awareness: awarenessProtocol.Awareness, conns: Set<import('ws').WebSocket>, debounceTimer: any }>} */
const rooms = new Map();

function getRoom(roomName) {
  if (rooms.has(roomName)) return rooms.get(roomName);

  const doc = new Y.Doc();
  const awareness = new awarenessProtocol.Awareness(doc);

  const room = { doc, awareness, conns: new Set(), debounceTimer: null };

  // When the doc updates, schedule a callback
  doc.on('update', () => {
    if (!CALLBACK_URL) return;
    clearTimeout(room.debounceTimer);
    room.debounceTimer = setTimeout(() => {
      fetch(CALLBACK_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ room: roomName }),
      }).catch(err => console.error(`Callback failed: ${err.message}`));
    }, CALLBACK_DEBOUNCE);
  });

  rooms.set(roomName, room);
  console.log(`Created room: ${roomName}`);
  return room;
}

function handleConnection(ws, roomName) {
  const room = getRoom(roomName);
  room.conns.add(ws);
  console.log(`Client joined "${roomName}" (${room.conns.size} total)`);

  // Send initial sync step 1
  {
    const encoder = encoding.createEncoder();
    encoding.writeVarUint(encoder, MSG_SYNC);
    syncProtocol.writeSyncStep1(encoder, room.doc);
    ws.send(encoding.toUint8Array(encoder));
  }

  // Send current awareness state
  {
    const awarenessStates = awarenessProtocol.encodeAwarenessUpdate(
      room.awareness,
      Array.from(room.awareness.getStates().keys())
    );
    const encoder = encoding.createEncoder();
    encoding.writeVarUint(encoder, MSG_AWARENESS);
    encoding.writeVarUint8Array(encoder, awarenessStates);
    ws.send(encoding.toUint8Array(encoder));
  }

  ws.on('message', (data) => {
    try {
      const message = new Uint8Array(data);
      const decoder = decoding.createDecoder(message);
      const msgType = decoding.readVarUint(decoder);

      switch (msgType) {
        case MSG_SYNC: {
          const encoder = encoding.createEncoder();
          encoding.writeVarUint(encoder, MSG_SYNC);
          syncProtocol.readSyncMessage(decoder, encoder, room.doc, ws);
          const reply = encoding.toUint8Array(encoder);
          // If there's a reply (SyncStep2 or update), send it back
          if (encoding.length(encoder) > 1) {
            ws.send(reply);
          }
          // Broadcast the raw message to other clients
          for (const conn of room.conns) {
            if (conn !== ws && conn.readyState === 1) {
              conn.send(message);
            }
          }
          break;
        }
        case MSG_AWARENESS: {
          const update = decoding.readVarUint8Array(decoder);
          awarenessProtocol.applyAwarenessUpdate(room.awareness, update, ws);
          // Broadcast awareness to all other clients
          for (const conn of room.conns) {
            if (conn !== ws && conn.readyState === 1) {
              const encoder = encoding.createEncoder();
              encoding.writeVarUint(encoder, MSG_AWARENESS);
              encoding.writeVarUint8Array(encoder, update);
              conn.send(encoding.toUint8Array(encoder));
            }
          }
          break;
        }
      }
    } catch (err) {
      console.error('Message handling error:', err.message);
    }
  });

  ws.on('close', () => {
    room.conns.delete(ws);
    console.log(`Client left "${roomName}" (${room.conns.size} remaining)`);
    if (room.conns.size === 0) {
      // Keep room alive for reconnects
    }
  });
}

// HTTP server for health checks
const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/plain' });
  res.end('ok');
});

const wss = new WebSocketServer({ server });

wss.on('connection', (ws, req) => {
  // Room name from URL path: /sparkdown-poc -> "sparkdown-poc"
  const roomName = (req.url || '/').slice(1) || 'default';
  handleConnection(ws, roomName);
});

server.listen(PORT, HOST, () => {
  console.log(`Sync server running at ws://${HOST}:${PORT}`);
  if (CALLBACK_URL) {
    console.log(`Callback → ${CALLBACK_URL} (debounce: ${CALLBACK_DEBOUNCE}ms)`);
  }
});
