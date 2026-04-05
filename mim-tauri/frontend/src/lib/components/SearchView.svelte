<script lang="ts">
  import { searchQuery, selectedPhotoId } from '$lib/stores/ui';
  import { activeFolder } from '$lib/stores/photos';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { fade, fly, scale } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import type { Photo } from '$lib/api/photos';

  let results = $state<Photo[]>([]);
  let isSearching = $state(false);
  let hasSearched = $state(false);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const q = $searchQuery;
    const folder = $activeFolder;
    if (debounceTimer) clearTimeout(debounceTimer);

    if (!q || q.length < 2 || !folder) {
      if (!q) { results = []; hasSearched = false; }
      return;
    }

    debounceTimer = setTimeout(async () => {
      isSearching = true;
      try {
        results = await invoke('search_photos', {
          folderPath: folder.path,
          query: q,
          limit: 200,
        });
        hasSearched = true;
      } catch (e) {
        console.error('Search failed:', e);
      }
      isSearching = false;
    }, 300);
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

  function selectPhoto(photo: Photo) {
    selectedPhotoId.set(photo.id);
  }
</script>

<div class="flex-1 overflow-y-auto p-4" style="padding-bottom: 80px;">
  <div class="flex items-center justify-between mb-4" in:fade={{ duration: 200 }}>
    <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">
      Search
    </h2>
    {#if results.length > 0}
      <span class="text-xs px-2 py-0.5 rounded-full"
        style="background: var(--color-accent-soft); color: var(--color-accent);">
        {results.length} results
      </span>
    {/if}
  </div>

  {#if isSearching}
    <div class="text-center py-12" style="color: var(--color-text-muted);">
      <span class="inline-block animate-spin text-xl">◌</span>
      <p class="mt-2 text-sm">Searching...</p>
    </div>
  {:else if !hasSearched}
    <div class="flex flex-col items-center justify-center h-[60vh] gap-4 animate-fade-in">
      <div class="w-20 h-20 rounded-2xl flex items-center justify-center text-3xl neu-raised"
        style="background: var(--color-surface);">
        ⌕
      </div>
      <p class="text-base font-medium" style="color: var(--color-text-secondary);">
        Search your photos
      </p>
      <p class="text-sm text-center max-w-xs" style="color: var(--color-text-muted);">
        Search by filename, AI description, tags, or location. Type in the search bar above.
      </p>
    </div>
  {:else if results.length === 0}
    <div class="flex flex-col items-center justify-center h-[60vh] gap-4 animate-fade-in">
      <div class="w-16 h-16 rounded-2xl flex items-center justify-center text-2xl neu-raised"
        style="background: var(--color-surface);">
        ∅
      </div>
      <p class="text-sm" style="color: var(--color-text-muted);">
        No results for "{$searchQuery}"
      </p>
    </div>
  {:else}
    <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
      {#each results as photo, i (photo.id)}
        <button
          class="relative aspect-square overflow-hidden rounded-xl transition-all duration-200 group cursor-pointer"
          style="box-shadow: var(--shadow-neu-soft);"
          animate:flip={{ duration: 300 }}
          in:scale={{ start: 0.9, duration: 300, delay: Math.min(i * 20, 500) }}
          onclick={() => selectPhoto(photo)}
        >
          <img
            src={getThumbnailSrc(photo)}
            alt={photo.filename}
            class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
            loading="lazy"
            onerror={(e) => { (e.target as HTMLImageElement).src = getFullSrc(photo); }}
          />
          <div class="absolute inset-0 bg-gradient-to-t from-black/50 via-transparent to-transparent
            opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-xl">
            <div class="absolute bottom-2 left-2 right-2">
              <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
              {#if photo.ai_description}
                <p class="text-white/70 text-[9px] truncate">{photo.ai_description}</p>
              {/if}
            </div>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>
