import { invoke } from '@tauri-apps/api/core';

export interface Photo {
  id: string;
  relative_path: string;
  filename: string;
  file_size: number;
  content_hash: string;
  width: number | null;
  height: number | null;
  format: string | null;
  media_type: 'photo' | 'video';
  taken_at: string | null;
  camera_make: string | null;
  camera_model: string | null;
  lens_model: string | null;
  focal_length: number | null;
  aperture: number | null;
  shutter_speed: string | null;
  iso: number | null;
  latitude: number | null;
  longitude: number | null;
  altitude: number | null;
  location_name: string | null;
  ai_description: string | null;
  thumbnail_generated: boolean;
  faces_processed: boolean;
  ai_processed: boolean;
  file_modified_at: string;
  created_at: string;
  updated_at: string;
}

export interface FolderSource {
  id: string;
  path: string;
  label: string | null;
  is_available: boolean;
  last_scanned_at: string | null;
  created_at: string;
}

export interface ScanProgress {
  total_found: number;
  new_photos: number;
  skipped: number;
  errors: number;
}

export async function addFolder(path: string, label?: string): Promise<FolderSource> {
  return invoke('add_folder', { path, label });
}

export async function getFolders(): Promise<FolderSource[]> {
  return invoke('get_folders');
}

export async function removeFolder(id: string): Promise<void> {
  return invoke('remove_folder', { id });
}

export async function scanFolder(path: string): Promise<ScanProgress> {
  return invoke('scan_folder', { path });
}

export async function getPhotos(folderPath: string, limit?: number, offset?: number): Promise<Photo[]> {
  return invoke('get_photos', { folderPath, limit, offset });
}

export async function getPhotoDetail(folderPath: string, photoId: string): Promise<Photo | null> {
  return invoke('get_photo_detail', { folderPath, photoId });
}

export async function getPhotoCount(folderPath: string): Promise<number> {
  return invoke('get_photo_count', { folderPath });
}

export async function getThumbnailUrl(folderPath: string, contentHash: string, size?: string): Promise<string> {
  return invoke('get_thumbnail_url', { folderPath, contentHash, size });
}
