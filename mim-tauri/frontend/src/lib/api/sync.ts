import { invoke } from '@tauri-apps/api/core';

export interface SyncStatus {
  running: boolean;
  device_id: string | null;
  synced_folders: string[];
}

export async function setupSync(): Promise<SyncStatus> {
  return invoke('setup_sync');
}

export async function getSyncStatus(): Promise<SyncStatus> {
  return invoke('get_sync_status');
}

export async function stopSync(): Promise<void> {
  return invoke('stop_sync');
}

export async function addSyncFolder(folderPath: string, label: string): Promise<void> {
  return invoke('add_sync_folder', { folderPath, label });
}
