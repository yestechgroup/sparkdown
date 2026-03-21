import type { FileEntry } from '$lib/tauri/commands';

let workspacePath = $state<string | null>(null);
let fileList = $state<FileEntry[]>([]);
let activeDocId = $state<string | null>(null);

export function getWorkspacePath() { return workspacePath; }
export function getFileList() { return fileList; }
export function getActiveDocId() { return activeDocId; }

export function setWorkspacePath(path: string | null) { workspacePath = path; }
export function setFileList(files: FileEntry[]) { fileList = files; }
export function setActiveDocId(docId: string | null) { activeDocId = docId; }
