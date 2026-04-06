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
  // Analysis data
  aesthetic_score: number | null;
  blur_score: number | null;
  scene_type: string | null;
  dominant_colors: string | null;
  perceptual_hash: string | null;
  is_screenshot: boolean | null;
  is_nsfw: boolean | null;
  ocr_text: string | null;
  weather: string | null;
  time_of_day: string | null;
  event_id: string | null;
  analysis_processed: boolean;
  // User data
  rating: number;
  is_favorite: boolean;
  is_trashed: boolean;
  trashed_at: string | null;
  // Processing state
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

export async function getPhotos(folderPath: string, limit?: number, offset?: number, mediaType?: string): Promise<Photo[]> {
  return invoke('get_photos', { folderPath, limit, offset, mediaType });
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

export async function ensureThumbnail(folderPath: string, photoId: string): Promise<boolean> {
  return invoke('ensure_thumbnail', { folderPath, photoId });
}

export interface DuplicateGroup {
  content_hash: string;
  photos: Photo[];
}

export async function findDuplicates(folderPath: string): Promise<DuplicateGroup[]> {
  return invoke('find_duplicates', { folderPath });
}

// Favorites & Rating
export async function toggleFavorite(folderPath: string, photoId: string): Promise<boolean> {
  return invoke('toggle_favorite', { folderPath, photoId });
}

export async function setRating(folderPath: string, photoId: string, rating: number): Promise<void> {
  return invoke('set_rating', { folderPath, photoId, rating });
}

// Trash
export async function trashPhoto(folderPath: string, photoId: string): Promise<void> {
  return invoke('trash_photo', { folderPath, photoId });
}

export async function restorePhoto(folderPath: string, photoId: string): Promise<void> {
  return invoke('restore_photo', { folderPath, photoId });
}

export async function emptyTrash(folderPath: string): Promise<number> {
  return invoke('empty_trash', { folderPath });
}

export async function getTrashed(folderPath: string): Promise<Photo[]> {
  return invoke('get_trashed', { folderPath });
}

// Video external
export async function openVideoExternal(folderPath: string, photoId: string): Promise<void> {
  return invoke('open_video_external', { folderPath, photoId });
}

// Share
export async function sharePhotoOs(folderPath: string, photoId: string): Promise<void> {
  return invoke('share_photo_os', { folderPath, photoId });
}

// Backup
export async function backupDatabase(folderPath: string, destPath: string): Promise<void> {
  return invoke('backup_database', { folderPath, destPath });
}

export async function restoreDatabase(folderPath: string, sourcePath: string): Promise<void> {
  return invoke('restore_database', { folderPath, sourcePath });
}

// Storage stats
export interface StorageStats {
  total_photos: number;
  total_size: number;
  thumbnail_size: number;
  face_crops_size: number;
  db_size: number;
}

export async function getStorageStats(folderPath: string): Promise<StorageStats> {
  return invoke('get_storage_stats', { folderPath });
}

// Memories
export async function getMemories(folderPath: string): Promise<Photo[]> {
  return invoke('get_memories', { folderPath });
}

// Smart Albums
export async function createSmartAlbum(folderPath: string, name: string, rulesJson: string): Promise<any> {
  return invoke('create_smart_album', { folderPath, name, rulesJson });
}

export async function getSmartAlbumPhotos(folderPath: string, albumId: string): Promise<Photo[]> {
  return invoke('get_smart_album_photos', { folderPath, albumId });
}

// Export
export async function exportAlbumZip(folderPath: string, albumId: string, destPath: string): Promise<number> {
  return invoke('export_album_zip', { folderPath, albumId, destPath });
}

// Locked Folders
export async function lockFolder(folderPath: string, password: string): Promise<void> {
  return invoke('lock_folder', { folderPath, password });
}

export async function unlockFolder(folderPath: string, password: string): Promise<void> {
  return invoke('unlock_folder', { folderPath, password });
}

export async function verifyFolderPassword(folderPath: string, password: string): Promise<boolean> {
  return invoke('verify_folder_password', { folderPath, password });
}

export async function openLockedFolder(folderPath: string, password: string): Promise<void> {
  return invoke('open_locked_folder', { folderPath, password });
}
