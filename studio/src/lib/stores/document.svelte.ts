import type { EntityDto, EntityDetailDto, DocumentOverviewDto, SidecarStatus, StaleAnchor } from '$lib/tauri/commands';

let entities = $state<EntityDto[]>([]);
let sidecarStatus = $state<SidecarStatus>({ synced: 0, stale: 0, detached: 0, total_triples: 0 });
let staleAnchors = $state<StaleAnchor[]>([]);
let selectedEntityId = $state<string | null>(null);
let entityDetail = $state<EntityDetailDto | null>(null);
let documentOverview = $state<DocumentOverviewDto | null>(null);
let dismissedStaleIds = $state<Set<string>>(new Set());

export function getEntities() { return entities; }
export function getSidecarStatus() { return sidecarStatus; }
export function getStaleAnchors() { return staleAnchors; }
export function getSelectedEntityId() { return selectedEntityId; }
export function getEntityDetailState() { return entityDetail; }
export function getDocumentOverviewState() { return documentOverview; }
export function getDismissedStaleIds() { return dismissedStaleIds; }

export function setEntities(e: EntityDto[]) { entities = e; }
export function setSidecarStatus(s: SidecarStatus) { sidecarStatus = s; }
export function setStaleAnchors(a: StaleAnchor[]) { staleAnchors = a; }
export function setSelectedEntityId(id: string | null) { selectedEntityId = id; }
export function setEntityDetail(detail: EntityDetailDto | null) { entityDetail = detail; }
export function setDocumentOverview(overview: DocumentOverviewDto | null) { documentOverview = overview; }
export function dismissStaleAnchor(id: string) { dismissedStaleIds = new Set([...dismissedStaleIds, id]); }

export function getVisibleStaleAnchors() {
  return staleAnchors.filter(a => !dismissedStaleIds.has(a.entity_id));
}

export function clearDocumentState() {
  entities = [];
  sidecarStatus = { synced: 0, stale: 0, detached: 0, total_triples: 0 };
  staleAnchors = [];
  selectedEntityId = null;
  entityDetail = null;
  documentOverview = null;
  dismissedStaleIds = new Set();
}
