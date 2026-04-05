import { invoke } from '@tauri-apps/api/core';

export interface Album {
  id: string;
  name: string;
  cover_photo_id: string | null;
  album_type: 'manual' | 'smart' | 'favorites';
  rules: unknown | null;
  photo_count: number;
  created_at: string;
  updated_at: string;
}

export async function createAlbum(folderPath: string, name: string): Promise<Album> {
  return invoke('create_album', { folderPath, name });
}

export async function getAlbums(folderPath: string): Promise<Album[]> {
  return invoke('get_albums', { folderPath });
}

export async function addToAlbum(folderPath: string, albumId: string, photoId: string): Promise<void> {
  return invoke('add_to_album', { folderPath, albumId, photoId });
}

export async function removeFromAlbum(folderPath: string, albumId: string, photoId: string): Promise<void> {
  return invoke('remove_from_album', { folderPath, albumId, photoId });
}

export async function getAlbumPhotos(folderPath: string, albumId: string): Promise<string[]> {
  return invoke('get_album_photos', { folderPath, albumId });
}

export async function deleteAlbum(folderPath: string, albumId: string): Promise<void> {
  return invoke('delete_album', { folderPath, albumId });
}

export async function renameAlbum(folderPath: string, albumId: string, name: string): Promise<void> {
  return invoke('rename_album', { folderPath, albumId, name });
}
