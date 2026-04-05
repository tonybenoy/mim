import { invoke } from '@tauri-apps/api/core';

export interface AnalysisProgress {
  total: number;
  processed: number;
}

export interface PhotoEvent {
  id: string;
  name: string;
  start_time: string | null;
  end_time: string | null;
  location_name: string | null;
  photo_count: number;
}

export async function analyzeFolder(folderPath: string): Promise<AnalysisProgress> {
  return invoke('analyze_folder', { folderPath });
}

export async function findSimilarPhotos(folderPath: string, photoId: string): Promise<any[]> {
  return invoke('find_similar_photos', { folderPath, photoId });
}

export async function getEvents(folderPath: string): Promise<PhotoEvent[]> {
  return invoke('get_events', { folderPath });
}

export async function getPhotoColors(folderPath: string, photoId: string): Promise<string[]> {
  return invoke('get_photo_colors', { folderPath, photoId });
}
