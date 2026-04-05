import { writable, derived } from 'svelte/store';
import type { Photo, FolderSource } from '$lib/api/photos';

export const folders = writable<FolderSource[]>([]);
export const activeFolder = writable<FolderSource | null>(null);
export const photos = writable<Photo[]>([]);
export const photoCount = writable(0);
export const isLoading = writable(false);

export const photosByDate = derived(photos, ($photos) => {
  const groups = new Map<string, Photo[]>();
  for (const photo of $photos) {
    const dateStr = photo.taken_at || photo.file_modified_at || photo.created_at;
    const date = dateStr
      ? new Date(dateStr).toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' })
      : 'Unknown Date';
    const group = groups.get(date) || [];
    group.push(photo);
    groups.set(date, group);
  }
  return Array.from(groups.entries()).map(([date, photos]) => ({ date, photos }));
});
