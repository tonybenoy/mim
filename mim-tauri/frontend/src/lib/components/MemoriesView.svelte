<script lang="ts">
  import { activeFolder } from '$lib/stores/photos';
  import { selectedPhotoId } from '$lib/stores/ui';
  import { getMemories } from '$lib/api/photos';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, scale } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';

  let memories = $state<Photo[]>([]);
  let loading = $state(true);

  // Group by year
  let memoriesByYear = $derived(() => {
    const groups = new Map<string, Photo[]>();
    for (const photo of memories) {
      const year = photo.taken_at ? new Date(photo.taken_at).getFullYear().toString() : 'Unknown';
      const group = groups.get(year) || [];
      group.push(photo);
      groups.set(year, group);
    }
    return Array.from(groups.entries())
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([year, photos]) => ({
        year,
        yearsAgo: new Date().getFullYear() - parseInt(year),
        photos,
      }));
  });

  $effect(() => {
    const folder = $activeFolder;
    if (folder) {
      loading = true;
      getMemories(folder.path)
        .then(p => { memories = p; loading = false; })
        .catch(() => { memories = []; loading = false; });
    }
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
</script>

<div class="flex-1 overflow-y-auto p-4" style="padding-bottom: 80px;">
  <h2 class="text-lg font-semibold mb-4" style="color: var(--color-text-primary);">
    On This Day
  </h2>

  {#if loading}
    <div class="flex items-center justify-center h-[40vh]">
      <div class="text-sm animate-pulse-glow" style="color: var(--color-text-muted);">
        Loading memories...
      </div>
    </div>
  {:else if memories.length === 0}
    <div class="flex flex-col items-center justify-center h-[40vh] gap-4" in:fade={{ duration: 200 }}>
      <div class="w-16 h-16 rounded-2xl flex items-center justify-center text-2xl neu-raised"
        style="background: var(--color-surface);">
        ❋
      </div>
      <p class="text-sm" style="color: var(--color-text-secondary);">
        No memories for today
      </p>
      <p class="text-xs" style="color: var(--color-text-muted);">
        Photos taken on this date in previous years will appear here
      </p>
    </div>
  {:else}
    {#each memoriesByYear() as group, gi}
      <div class="mb-8 animate-fade-in" style="animation-delay: {gi * 100}ms;" in:fade={{ duration: 300, delay: gi * 80 }}>
        <div class="flex items-center gap-3 mb-3">
          <div class="glass inline-block px-3 py-1.5 rounded-full text-xs font-medium"
            style="color: var(--color-accent);">
            {group.yearsAgo} {group.yearsAgo === 1 ? 'year' : 'years'} ago
          </div>
          <span class="text-xs" style="color: var(--color-text-muted);">{group.year}</span>
          <span class="text-xs" style="color: var(--color-text-muted);">{group.photos.length} photos</span>
        </div>
        <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
          {#each group.photos as photo, pi (photo.id)}
            <button
              class="photo-card relative aspect-square overflow-hidden rounded-2xl group cursor-pointer"
              style="box-shadow: var(--shadow-neu-soft);"
              in:scale={{ start: 0.9, duration: 300, delay: pi * 30 }}
              onclick={() => selectedPhotoId.set(photo.id)}
            >
              <img
                src={getThumbnailSrc(photo)}
                alt={photo.filename}
                class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-110"
                loading="lazy"
                onerror={(e) => {
                  const img = e.target as HTMLImageElement;
                  img.src = getFullSrc(photo);
                }}
              />
              <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent
                opacity-0 group-hover:opacity-100 transition-all duration-300 rounded-2xl">
                <div class="absolute bottom-2 left-2 right-2">
                  <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
                </div>
              </div>
            </button>
          {/each}
        </div>
      </div>
    {/each}
  {/if}
</div>
