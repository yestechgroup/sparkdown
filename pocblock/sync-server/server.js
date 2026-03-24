import http from 'node:http';
import { WebSocketServer } from 'ws';
import * as Y from 'yjs';
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

  // Send initial sync step 1 (our state vector)
  {
    const encoder = encoding.createEncoder();
    encoding.writeVarUint(encoder, MSG_SYNC);
    encoding.writeVarUint(encoder, 0); // SyncStep1
    encoding.writeVarUint8Array(encoder, Y.encodeStateVector(room.doc));
    ws.send(encoding.toUint8Array(encoder));
  }

  // Send current awareness state (only if there are states)
  {
    const clients = Array.from(room.awareness.getStates().keys());
    if (clients.length > 0) {
      const awarenessStates = awarenessProtocol.encodeAwarenessUpdate(
        room.awareness,
        clients
      );
      const encoder = encoding.createEncoder();
      encoding.writeVarUint(encoder, MSG_AWARENESS);
      encoding.writeVarUint8Array(encoder, awarenessStates);
      ws.send(encoding.toUint8Array(encoder));
    }
  }

  ws.on('message', (data) => {
    try {
      const message = new Uint8Array(data);
      const hexPreview = Array.from(message.slice(0, 20)).map(b => b.toString(16).padStart(2, '0')).join(' ');
      console.log(`Recv [${message.length} bytes]: ${hexPreview}${message.length > 20 ? '...' : ''}`);
      const decoder = decoding.createDecoder(message);
      const msgType = decoding.readVarUint(decoder);

      switch (msgType) {
        case MSG_SYNC: {
          const syncType = decoding.readVarUint(decoder);

          if (syncType === 0) {
            // SyncStep1: client sends state vector, we reply with SyncStep2 (missing updates)
            const svBytes = decoding.readVarUint8Array(decoder);
            let sv;
            if (svBytes.length === 0) {
              // Empty state vector = client has nothing, send full state
              sv = new Uint8Array([0]); // encoded empty state vector (0 entries)
            } else {
              sv = svBytes;
            }
            const update = Y.encodeStateAsUpdate(room.doc, Y.decodeStateVector(sv));
            const encoder = encoding.createEncoder();
            encoding.writeVarUint(encoder, MSG_SYNC);
            encoding.writeVarUint(encoder, 1); // SyncStep2
            encoding.writeVarUint8Array(encoder, update);
            ws.send(encoding.toUint8Array(encoder));

          } else if (syncType === 1) {
            // SyncStep2: client sends missing updates, apply them
            const update = decoding.readVarUint8Array(decoder);
            Y.applyUpdate(room.doc, update);

          } else if (syncType === 2) {
            // Update: incremental update, apply and broadcast
            const update = decoding.readVarUint8Array(decoder);
            Y.applyUpdate(room.doc, update);
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
