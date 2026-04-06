<script lang="ts">
  import { photos, activeFolder } from '$lib/stores/photos';
  import { selectedPhotoId } from '$lib/stores/ui';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, scale } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';

  // Group photos by location_name, or by rounded lat/lon
  let locationGroups = $derived(() => {
    const groups = new Map<string, Photo[]>();
    for (const photo of $photos) {
      let key = 'No Location';
      if (photo.location_name) {
        key = photo.location_name;
      } else if (photo.latitude != null && photo.longitude != null) {
        key = `${photo.latitude.toFixed(1)}, ${photo.longitude.toFixed(1)}`;
      }
      const group = groups.get(key) || [];
      group.push(photo);
      groups.set(key, group);
    }
    return Array.from(groups.entries())
      .sort((a, b) => b[1].length - a[1].length)
      .map(([location, photos]) => ({ location, photos }));
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
    Places
  </h2>

  {#if locationGroups().length === 0}
    <div class="flex flex-col items-center justify-center h-[40vh] gap-4" in:fade={{ duration: 200 }}>
      <div class="w-16 h-16 rounded-2xl flex items-center justify-center text-2xl neu-raised"
        style="background: var(--color-surface);">
        &#9673;
      </div>
      <p class="text-sm" style="color: var(--color-text-secondary);">
        No location data found
      </p>
    </div>
  {:else}
    {#each locationGroups() as group, gi}
      <div class="mb-6 animate-fade-in" style="animation-delay: {gi * 50}ms;" in:fade={{ duration: 200, delay: gi * 40 }}>
        <div class="flex items-center gap-2 mb-3">
          <div class="glass inline-block px-3 py-1.5 rounded-full text-xs font-medium"
            style="color: var(--color-text-secondary);">
            {group.location}
            <span style="color: var(--color-text-muted);">&middot; {group.photos.length}</span>
          </div>
        </div>
        <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));">
          {#each group.photos.slice(0, 12) as photo, pi (photo.id)}
            <button
              class="photo-card relative aspect-square overflow-hidden rounded-xl group cursor-pointer"
              style="box-shadow: var(--shadow-neu-soft);"
              in:scale={{ start: 0.9, duration: 300, delay: pi * 20 }}
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
            </button>
          {/each}
          {#if group.photos.length > 12}
            <div class="aspect-square flex items-center justify-center rounded-xl"
              style="background: var(--color-surface); color: var(--color-text-muted);">
              <span class="text-xs">+{group.photos.length - 12} more</span>
            </div>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>
