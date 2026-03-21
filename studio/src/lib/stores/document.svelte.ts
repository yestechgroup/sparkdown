import type { EntityDto, SidecarStatus, StaleAnchor } from '$lib/tauri/commands';

let entities = $state<EntityDto[]>([]);
let sidecarStatus = $state<SidecarStatus>({ synced: 0, stale: 0, detached: 0, total_triples: 0 });
let staleAnchors = $state<StaleAnchor[]>([]);

export function getEntities() { return entities; }
export function getSidecarStatus() { return sidecarStatus; }
export function getStaleAnchors() { return staleAnchors; }

export function setEntities(e: EntityDto[]) { entities = e; }
export function setSidecarStatus(s: SidecarStatus) { sidecarStatus = s; }
export function setStaleAnchors(a: StaleAnchor[]) { staleAnchors = a; }

export function clearDocumentState() {
  entities = [];
  sidecarStatus = { synced: 0, stale: 0, detached: 0, total_triples: 0 };
  staleAnchors = [];
}
