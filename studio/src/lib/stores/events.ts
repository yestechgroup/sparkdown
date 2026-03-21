import { listen } from '@tauri-apps/api/event';
import { setEntities, setSidecarStatus, setStaleAnchors } from './document.svelte';
import type { EntityDto, SidecarStatus, StaleAnchor } from '$lib/tauri/commands';

interface DocumentOpenedPayload {
  doc_id: string;
  entities: EntityDto[];
  sidecar_status: SidecarStatus;
}

interface EntitiesUpdatedPayload {
  doc_id: string;
  entities: EntityDto[];
}

interface SidecarStatusPayload {
  doc_id: string;
  status: SidecarStatus;
}

interface StaleAnchorsPayload {
  doc_id: string;
  anchors: StaleAnchor[];
}

interface ErrorPayload {
  doc_id: string;
  message: string;
}

let unlisteners: (() => void)[] = [];

export async function setupEventListeners() {
  unlisteners.push(
    await listen<DocumentOpenedPayload>('document-opened', (event) => {
      setEntities(event.payload.entities);
      setSidecarStatus(event.payload.sidecar_status);
    }),

    await listen<EntitiesUpdatedPayload>('entities-updated', (event) => {
      setEntities(event.payload.entities);
    }),

    await listen<SidecarStatusPayload>('sidecar-status', (event) => {
      setSidecarStatus(event.payload.status);
    }),

    await listen<StaleAnchorsPayload>('stale-anchors', (event) => {
      setStaleAnchors(event.payload.anchors);
    }),

    await listen<ErrorPayload>('parse-error', (event) => {
      console.warn('[Sparkdown] Parse error:', event.payload.message);
    }),

    await listen<ErrorPayload>('sidecar-error', (event) => {
      console.warn('[Sparkdown] Sidecar error:', event.payload.message);
    }),
  );
}

export function teardownEventListeners() {
  unlisteners.forEach((fn) => fn());
  unlisteners = [];
}
