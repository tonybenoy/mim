import { writable } from 'svelte/store';

export type ViewMode = 'timeline' | 'grid' | 'masonry';
export type AppSection = 'library' | 'people' | 'albums' | 'search' | 'chat';

export const currentSection = writable<AppSection>('library');
export const viewMode = writable<ViewMode>('grid');
export const sidebarOpen = writable(true);
export const selectedPhotoId = writable<string | null>(null);
export const searchQuery = writable('');
export const isScanning = writable(false);

// ML processing state
export type MlStatus = 'idle' | 'downloading' | 'detecting' | 'clustering' | 'done' | 'error';
export const mlStatus = writable<MlStatus>('idle');
export const mlStatusText = writable('');
export const mlProgress = writable({ total: 0, processed: 0, faces_found: 0 });
