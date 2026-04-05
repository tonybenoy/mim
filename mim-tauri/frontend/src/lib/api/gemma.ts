import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface TaggingProgress {
  total: number;
  processed: number;
  tagged: number;
}

export async function tagPhotos(folderPath: string): Promise<TaggingProgress> {
  return invoke('tag_photos', { folderPath });
}

export async function chatAboutPhoto(
  folderPath: string,
  photoId: string,
  question: string
): Promise<string> {
  return invoke('chat_about_photo', { folderPath, photoId, question });
}

export function onGemmaStatus(callback: (status: string) => void): Promise<UnlistenFn> {
  return listen<string>('gemma-status', (event) => {
    callback(event.payload);
  });
}

export function onTaggingProgress(callback: (progress: TaggingProgress) => void): Promise<UnlistenFn> {
  return listen<TaggingProgress>('gemma-tagging-progress', (event) => {
    callback(event.payload);
  });
}
