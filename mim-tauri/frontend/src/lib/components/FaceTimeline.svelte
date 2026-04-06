<script lang="ts">
  import { activeFolder } from '$lib/stores/photos';
  import { selectedPhotoId } from '$lib/stores/ui';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, fly } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';
  import type { Face } from '$lib/api/faces';

  let {
    identityName = '',
    photos = [],
  }: {
    identityName: string;
    photos: { photo: Photo; face: Face }[];
  } = $props();

  // Group photos by year
  let photosByYear = $derived.by(() => {
    const map = new Map<number, { photo: Photo; face: Face }[]>();
    const sorted = [...photos].sort((a, b) => {
      const dateA = a.photo.taken_at || a.photo.file_modified_at || a.photo.created_at;
      const dateB = b.photo.taken_at || b.photo.file_modified_at || b.photo.created_at;
      return (dateA || '').localeCompare(dateB || '');
    });

    for (const item of sorted) {
      const dateStr = item.photo.taken_at || item.photo.file_modified_at || item.photo.created_at;
      const year = dateStr ? new Date(dateStr).getFullYear() : 0;
      const list = map.get(year) || [];
      list.push(item);
      map.set(year, list);
    }
    return Array.from(map.entries()).sort(([a], [b]) => a - b);
  });

  function getThumbnailSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    const prefix = photo.content_hash.slice(0, 2);
    return convertFileSrc(`${$activeFolder.path}/.mim/thumbnails/${prefix}/${photo.content_hash}_256.webp`);
  }

  function getFullSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    return convertFileSrc(`${$activeFolder.path}/${photo.relative_path}`);
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '';
    return new Date(dateStr).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }
</script>

<div class="mt-4" in:fade={{ duration: 200 }}>
  <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
    Timeline for {identityName}
  </div>

  {#if photosByYear.length === 0}
    <div class="text-center py-8" style="color: var(--color-text-muted);">
      <p class="text-sm">No photos to show in timeline</p>
    </div>
  {:else}
    <div class="relative pl-6" style="border-left: 2px solid var(--color-border-glass);">
      {#each photosByYear as [year, yearPhotos], yi}
        <div
          class="mb-6 relative"
          in:fly={{ x: -20, duration: 300, delay: yi * 100 }}
        >
          <!-- Year marker -->
          <div class="absolute -left-[29px] w-4 h-4 rounded-full flex items-center justify-center"
            style="background: var(--color-accent); box-shadow: 0 0 8px var(--color-accent-glow);">
            <div class="w-2 h-2 rounded-full" style="background: var(--color-bg);"></div>
          </div>

          <div class="flex items-center gap-2 mb-3 ml-2">
            <span class="text-sm font-semibold" style="color: var(--color-text-primary);">
              {year === 0 ? 'Unknown' : year}
            </span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full"
              style="background: var(--color-surface); color: var(--color-text-muted);">
              {yearPhotos.length}
            </span>
          </div>

          <!-- Horizontal scroll row of photos -->
          <div class="flex gap-2 overflow-x-auto pb-2 ml-2" style="scrollbar-width: thin;">
            {#each yearPhotos as { photo, face }, pi}
              <button
                class="flex-shrink-0 relative w-24 h-24 rounded-xl overflow-hidden group cursor-pointer transition-all hover:scale-105"
                style="box-shadow: var(--shadow-neu-soft);"
                onclick={() => selectedPhotoId.set(photo.id)}
              >
                <img
                  src={getThumbnailSrc(photo)}
                  alt={photo.filename}
                  class="w-full h-full object-cover"
                  loading="lazy"
                  onerror={(e) => { (e.target as HTMLImageElement).src = getFullSrc(photo); }}
                />
                <div class="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent
                  opacity-0 group-hover:opacity-100 transition-opacity">
                  <div class="absolute bottom-1 left-1 right-1">
                    <p class="text-white text-[8px] truncate">
                      {formatDate(photo.taken_at || photo.file_modified_at)}
                    </p>
                  </div>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
