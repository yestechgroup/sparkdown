import { invoke } from '@tauri-apps/api/core';

export interface EntityDto {
  id: string;
  label: string;
  type_iris: string[];
  type_prefix: string;
  span_start: number;
  span_end: number;
  status: 'synced' | 'stale' | 'detached';
  top_relations: Relation[];
}

export interface Relation {
  predicate_label: string;
  target_label: string;
  target_id: string;
}

export interface SidecarStatus {
  synced: number;
  stale: number;
  detached: number;
  total_triples: number;
}

export interface StaleAnchor {
  entity_id: string;
  old_snippet: string;
  new_text: string;
  span_start: number;
  span_end: number;
}

export interface FileEntry {
  name: string;
  path: string;
  has_sidecar: boolean;
}

export interface WorkspaceInfo {
  path: string;
  files: FileEntry[];
}

// Phase 2 types

export interface EntityDetailDto {
  id: string;
  label: string;
  type_iris: string[];
  type_prefix: string;
  span_start: number;
  span_end: number;
  status: 'synced' | 'stale' | 'detached';
  properties: PropertyDto[];
  incoming_relations: IncomingRelation[];
}

export interface PropertyDto {
  predicate_label: string;
  predicate_iri: string;
  value: string;
}

export interface IncomingRelation {
  subject_id: string;
  subject_label: string;
  predicate_label: string;
}

export async function openWorkspace(): Promise<WorkspaceInfo> {
  return invoke('open_workspace');
}

export async function listWorkspaceFiles(path: string): Promise<FileEntry[]> {
  return invoke('list_workspace_files', { path });
}

export async function openDocument(path: string): Promise<string> {
  return invoke('open_document', { path });
}

export async function closeDocument(docId: string): Promise<void> {
  return invoke('close_document', { docId });
}

export async function updateSource(docId: string, newSource: string): Promise<void> {
  return invoke('update_source', { docId, newSource });
}

export async function getEntitiesAt(docId: string, start: number, end: number): Promise<EntityDto[]> {
  return invoke('get_entities_at', { docId, start, end });
}

export async function exportDocument(docId: string, format: 'html_rdfa' | 'json_ld' | 'turtle'): Promise<string> {
  return invoke('export_document', { docId, format });
}

export async function saveDocument(docId: string): Promise<void> {
  return invoke('save_document', { docId });
}

// Phase 2 commands

export async function createEntity(
  docId: string,
  spanStart: number,
  spanEnd: number,
  typeIri: string,
): Promise<EntityDto> {
  return invoke('create_entity', { docId, spanStart, spanEnd, typeIri });
}

export async function updateStaleAnchor(docId: string, entityId: string): Promise<void> {
  return invoke('update_stale_anchor', { docId, entityId });
}

export async function dismissSuggestion(docId: string, entityId: string): Promise<void> {
  return invoke('dismiss_suggestion', { docId, entityId });
}

export async function getAllEntities(docId: string): Promise<EntityDto[]> {
  return invoke('get_all_entities', { docId });
}

export async function getEntityDetails(docId: string, entityId: string): Promise<EntityDetailDto> {
  return invoke('get_entity_details', { docId, entityId });
}

export async function addTriple(
  docId: string,
  subjectId: string,
  predicateIri: string,
  objectValue: string,
  objectIsEntity: boolean,
): Promise<void> {
  return invoke('add_triple', { docId, subjectId, predicateIri, objectValue, objectIsEntity });
}

export async function checkFileModified(docId: string): Promise<boolean> {
  return invoke('check_file_modified', { docId });
}
