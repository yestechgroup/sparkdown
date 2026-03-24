use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, Transact, Update};

const ROOM_NAME: &str = "sparkdown-poc";

// --- lib0 varUint encoding helpers (compatible with y-protocols wire format) ---

fn write_var_uint(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        if value < 0x80 {
            buf.push(value as u8);
            break;
        }
        buf.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
}

fn write_var_uint8_array(buf: &mut Vec<u8>, data: &[u8]) {
    write_var_uint(buf, data.len() as u64);
    buf.extend_from_slice(data);
}

fn read_var_uint(data: &[u8], pos: &mut usize) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() {
            return result;
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if byte < 0x80 {
            break;
        }
        shift += 7;
    }
    result
}

fn read_var_uint8_array<'a>(data: &'a [u8], pos: &mut usize) -> &'a [u8] {
    let len = read_var_uint(data, pos) as usize;
    let end = (*pos + len).min(data.len());
    let slice = &data[*pos..end];
    *pos = end;
    slice
}

/// Connect a Yrs Doc to the y-websocket server as a Yjs peer.
///
/// Implements the Yjs sync protocol manually:
/// 1. Send SyncStep1 (our state vector)
/// 2. Receive SyncStep2 (missing updates from server)
/// 3. Exchange incremental updates bidirectionally
pub async fn connect_and_sync(ws_url: &str, doc: Arc<Mutex<Doc>>) -> Result<()> {
    let url = format!("{}/{}", ws_url, ROOM_NAME);

    tracing::info!("Connecting to y-websocket at {url}");

    let (ws_stream, _response) = tokio_tungstenite::connect_async(&url).await?;
    tracing::info!("Connected to y-websocket");

    let (mut sink, mut stream) = ws_stream.split();

    // Step 1: Send our state vector (SyncStep1)
    // y-protocols wire format: [varUint(0=sync), varUint(0=step1), varUint8Array(stateVector)]
    let sv_msg = {
        let doc_guard = doc.lock().await;
        let txn = doc_guard.transact();
        let sv = txn.state_vector().encode_v1();
        drop(txn);
        drop(doc_guard);

        let mut msg = Vec::new();
        write_var_uint(&mut msg, 0); // messageYjsSync
        write_var_uint(&mut msg, 0); // syncStep1
        write_var_uint8_array(&mut msg, &sv);
        msg
    };

    sink.send(tokio_tungstenite::tungstenite::Message::Binary(
        sv_msg.into(),
    ))
    .await?;
    tracing::debug!("Sent SyncStep1");

    // Set up update observer to forward local changes to the websocket
    let sink = Arc::new(Mutex::new(sink));
    let sink_clone = sink.clone();

    {
        let doc_guard = doc.lock().await;
        let _sub = doc_guard.observe_update_v1(move |_txn, event| {
            let update = event.update.clone();
            let sink = sink_clone.clone();
            tokio::spawn(async move {
                let mut msg = Vec::new();
                write_var_uint(&mut msg, 0); // messageYjsSync
                write_var_uint(&mut msg, 2); // syncUpdate
                write_var_uint8_array(&mut msg, &update);

                let mut sink_guard = sink.lock().await;
                if let Err(e) = sink_guard
                    .send(tokio_tungstenite::tungstenite::Message::Binary(msg.into()))
                    .await
                {
                    tracing::error!("Failed to send update: {e}");
                }
            });
        })?;
        // Keep subscription alive (PoC approach)
        std::mem::forget(_sub);
    }

    // Process incoming messages
    while let Some(msg) = stream.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Binary(data)) => {
                if data.is_empty() {
                    continue;
                }

                let mut pos = 0;
                let msg_type = read_var_uint(&data, &mut pos);

                match msg_type {
                    0 => {
                        // Sync message
                        let sync_type = read_var_uint(&data, &mut pos);
                        let payload = read_var_uint8_array(&data, &mut pos);

                        match sync_type {
                            0 => {
                                // SyncStep1 from server — send our diff as SyncStep2
                                tracing::debug!("Received SyncStep1");
                                if let Ok(sv) = yrs::StateVector::decode_v1(payload) {
                                    let resp = {
                                        let doc_guard = doc.lock().await;
                                        let txn = doc_guard.transact();
                                        let update = txn.encode_diff_v1(&sv);
                                        drop(txn);
                                        drop(doc_guard);

                                        let mut resp = Vec::new();
                                        write_var_uint(&mut resp, 0); // messageYjsSync
                                        write_var_uint(&mut resp, 1); // syncStep2
                                        write_var_uint8_array(&mut resp, &update);
                                        resp
                                    };

                                    let mut sink_guard = sink.lock().await;
                                    if let Err(e) = sink_guard
                                        .send(
                                            tokio_tungstenite::tungstenite::Message::Binary(
                                                resp.into(),
                                            ),
                                        )
                                        .await
                                    {
                                        tracing::error!("Failed to send SyncStep2: {e}");
                                    }
                                }
                            }
                            1 => {
                                // SyncStep2 from server — apply the update
                                tracing::debug!(
                                    "Received SyncStep2 ({} bytes)",
                                    payload.len()
                                );
                                let payload = payload.to_vec();
                                let doc_guard = doc.lock().await;
                                if let Ok(update) = Update::decode_v1(&payload) {
                                    let mut txn = doc_guard.transact_mut();
                                    txn.apply_update(update)?;
                                    tracing::debug!("Applied SyncStep2");
                                }
                                drop(doc_guard);
                            }
                            2 => {
                                // Incremental update
                                tracing::debug!(
                                    "Received update ({} bytes)",
                                    payload.len()
                                );
                                let payload = payload.to_vec();
                                let doc_guard = doc.lock().await;
                                if let Ok(update) = Update::decode_v1(&payload) {
                                    let mut txn = doc_guard.transact_mut();
                                    txn.apply_update(update)?;
                                }
                                drop(doc_guard);
                            }
                            _ => {
                                tracing::trace!("Unknown sync type: {sync_type}");
                            }
                        }
                    }
                    1 => {
                        // Awareness message — ignore for PoC
                        tracing::trace!("Received awareness message");
                    }
                    _ => {
                        tracing::trace!("Unknown message type: {msg_type}");
                    }
                }
            }
            Ok(_) => {} // Ignore non-binary messages
            Err(e) => {
                tracing::error!("WebSocket error: {e}");
                return Err(e.into());
            }
        }
    }

    Ok(())
}
