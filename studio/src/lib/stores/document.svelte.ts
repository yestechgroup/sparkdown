import type { EntityDto, EntityDetailDto, SidecarStatus, StaleAnchor } from '$lib/tauri/commands';

let entities = $state<EntityDto[]>([]);
let sidecarStatus = $state<SidecarStatus>({ synced: 0, stale: 0, detached: 0, total_triples: 0 });
let staleAnchors = $state<StaleAnchor[]>([]);

// Phase 2 state
export type EditorMode = 'deep-writing' | 'light-writing' | 'review' | 'full-reading';

let selectedEntityId = $state<string | null>(null);
let selectedEntityDetail = $state<EntityDetailDto | null>(null);
let knowledgePanelOpen = $state(false);
let editorMode = $state<EditorMode>('deep-writing');

export function getEntities() { return entities; }
export function getSidecarStatus() { return sidecarStatus; }
export function getStaleAnchors() { return staleAnchors; }

export function setEntities(e: EntityDto[]) { entities = e; }
export function setSidecarStatus(s: SidecarStatus) { sidecarStatus = s; }
export function setStaleAnchors(a: StaleAnchor[]) { staleAnchors = a; }

// Phase 2 getters/setters
export function getSelectedEntityId() { return selectedEntityId; }
export function setSelectedEntityId(id: string | null) { selectedEntityId = id; }

export function getSelectedEntityDetail() { return selectedEntityDetail; }
export function setSelectedEntityDetail(d: EntityDetailDto | null) { selectedEntityDetail = d; }

export function getKnowledgePanelOpen() { return knowledgePanelOpen; }
export function setKnowledgePanelOpen(open: boolean) { knowledgePanelOpen = open; }

export function getEditorMode() { return editorMode; }
export function setEditorMode(mode: EditorMode) { editorMode = mode; }

export function clearDocumentState() {
  entities = [];
  sidecarStatus = { synced: 0, stale: 0, detached: 0, total_triples: 0 };
  staleAnchors = [];
  selectedEntityId = null;
  selectedEntityDetail = null;
  knowledgePanelOpen = false;
  editorMode = 'deep-writing';
}
