use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, Transact, Update};

const ROOM_NAME: &str = "sparkdown-poc";

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
    // Yjs wire format: [messageType=0 (sync), syncType=0 (step1), ...stateVector]
    let sv_msg = {
        let doc_guard = doc.lock().await;
        let txn = doc_guard.transact();
        let sv = txn.state_vector().encode_v1();
        drop(txn);
        drop(doc_guard);

        let mut msg = Vec::with_capacity(2 + sv.len());
        msg.push(0u8); // messageYjsSyncStep1
        msg.push(0u8); // syncStep1
        msg.extend_from_slice(&sv);
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
                let mut msg = Vec::with_capacity(2 + update.len());
                msg.push(0u8); // messageYjsSync
                msg.push(2u8); // syncUpdate
                msg.extend_from_slice(&update);

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
                if data.len() < 2 {
                    continue;
                }

                let msg_type = data[0];
                let sync_type = data[1];
                let payload = &data[2..];

                match (msg_type, sync_type) {
                    (0, 0) => {
                        // SyncStep1 from server — send our diff as SyncStep2
                        tracing::debug!("Received SyncStep1");
                        if let Ok(sv) = yrs::StateVector::decode_v1(payload) {
                            let resp = {
                                let doc_guard = doc.lock().await;
                                let txn = doc_guard.transact();
                                let update = txn.encode_diff_v1(&sv);
                                drop(txn);
                                drop(doc_guard);

                                let mut resp = Vec::with_capacity(2 + update.len());
                                resp.push(0u8); // messageYjsSync
                                resp.push(1u8); // syncStep2
                                resp.extend_from_slice(&update);
                                resp
                            };

                            let mut sink_guard = sink.lock().await;
                            if let Err(e) = sink_guard
                                .send(
                                    tokio_tungstenite::tungstenite::Message::Binary(resp.into()),
                                )
                                .await
                            {
                                tracing::error!("Failed to send SyncStep2: {e}");
                            }
                        }
                    }
                    (0, 1) => {
                        // SyncStep2 from server — apply the update
                        tracing::debug!("Received SyncStep2 ({} bytes)", payload.len());
                        let payload = payload.to_vec();
                        let doc_guard = doc.lock().await;
                        if let Ok(update) = Update::decode_v1(&payload) {
                            let mut txn = doc_guard.transact_mut();
                            txn.apply_update(update)?;
                            tracing::debug!("Applied SyncStep2");
                        }
                        drop(doc_guard);
                    }
                    (0, 2) => {
                        // Incremental update
                        tracing::debug!("Received update ({} bytes)", payload.len());
                        let payload = payload.to_vec();
                        let doc_guard = doc.lock().await;
                        if let Ok(update) = Update::decode_v1(&payload) {
                            let mut txn = doc_guard.transact_mut();
                            txn.apply_update(update)?;
                        }
                        drop(doc_guard);
                    }
                    (1, _) => {
                        // Awareness message — ignore for PoC
                        tracing::trace!("Received awareness message");
                    }
                    _ => {
                        tracing::trace!(
                            "Unknown message type: msg={msg_type}, sync={sync_type}"
                        );
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
