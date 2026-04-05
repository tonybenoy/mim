<script lang="ts">
  import { photos, activeFolder, photosByDate, isLoading } from '$lib/stores/photos';
  import { viewMode, selectedPhotoId, isScanning } from '$lib/stores/ui';
  import { getPhotos } from '$lib/api/photos';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { fly, fade, scale } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import PhotoSkeleton from './PhotoSkeleton.svelte';
  import type { Photo } from '$lib/api/photos';

  let gridSize = $state(180);
  let loadedHashes = $state(new Set<string>());

  // Load photos when active folder changes
  $effect(() => {
    const folder = $activeFolder;
    if (folder) {
      isLoading.set(true);
      getPhotos(folder.path, 500, 0)
        .then(p => {
          photos.set(p);
          isLoading.set(false);
        })
        .catch(() => isLoading.set(false));
    }
  });

  function getThumbnailSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    const prefix = photo.content_hash.slice(0, 2);
    const base = `${$activeFolder.path}/.mim/thumbnails/${prefix}/${photo.content_hash}`;
    // Try 256 first, the onerror handler will cascade to smaller/full
    return convertFileSrc(`${base}_256.webp`);
  }

  function getMicroSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    const prefix = photo.content_hash.slice(0, 2);
    return convertFileSrc(`${$activeFolder.path}/.mim/thumbnails/${prefix}/${photo.content_hash}_64.webp`);
  }

  function getFullSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    return convertFileSrc(`${$activeFolder.path}/${photo.relative_path}`);
  }

  // Cascade: 256 thumbnail → 64 thumbnail → full image
  function handleImageError(e: Event, photo: Photo) {
    const img = e.target as HTMLImageElement;
    const microSrc = getMicroSrc(photo);
    const fullSrc = getFullSrc(photo);
    if (!img.src.includes('_64.webp') && microSrc) {
      img.src = microSrc;
    } else {
      img.src = fullSrc;
    }
  }

  function selectPhoto(photo: Photo) {
    selectedPhotoId.set(photo.id);
  }

  // Scroll date indicator
  let scrollDate = $state('');
  let showScrollDate = $state(false);
  let scrollTimeout: ReturnType<typeof setTimeout> | null = null;

  function handleScroll(e: Event) {
    const container = e.target as HTMLElement;
    // Find the first visible date header
    const headers = container.querySelectorAll('[data-date]');
    for (const header of headers) {
      const rect = header.getBoundingClientRect();
      if (rect.top >= 56 && rect.top < 300) {
        scrollDate = (header as HTMLElement).dataset.date || '';
        break;
      }
    }
    showScrollDate = true;
    if (scrollTimeout) clearTimeout(scrollTimeout);
    scrollTimeout = setTimeout(() => { showScrollDate = false; }, 1200);
  }

  function onImageLoad(hash: string) {
    loadedHashes = new Set([...loadedHashes, hash]);
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return 'Unknown Date';
    return new Date(dateStr).toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  }
</script>

<div class="flex-1 overflow-y-auto p-4 relative" style="padding-bottom: 80px;" onscroll={handleScroll}>
  <!-- Floating scroll date indicator -->
  {#if showScrollDate && scrollDate}
    <div
      class="fixed z-50 pointer-events-none"
      style="top: 70px; right: 24px;"
      in:fade={{ duration: 150 }}
      out:fade={{ duration: 300 }}
    >
      <div class="glass px-4 py-2 rounded-full text-sm font-medium shadow-lg"
        style="color: var(--color-text-primary); backdrop-filter: blur(20px);">
        {scrollDate}
      </div>
    </div>
  {/if}
  <!-- Toolbar -->
  <div class="flex items-center justify-between mb-4" in:fade={{ duration: 200 }}>
    <div class="flex items-center gap-2">
      <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">
        {$activeFolder?.label || $activeFolder?.path.split('/').pop() || 'Select a folder'}
      </h2>
      {#if $photos.length > 0}
        <span class="text-xs px-2 py-0.5 rounded-full"
          style="background: var(--color-accent-soft); color: var(--color-accent);">
          {$photos.length} photos
        </span>
      {/if}
      {#if $isScanning}
        <div class="flex items-center gap-1.5 text-xs animate-pulse-glow px-2 py-0.5 rounded-full"
          style="color: var(--color-accent);">
          <span class="w-1.5 h-1.5 rounded-full" style="background: var(--color-accent);"></span>
          Scanning...
        </div>
      {/if}
    </div>

    <!-- View switcher -->
    <div class="flex items-center gap-1 p-1 rounded-xl" style="background: var(--color-surface);">
      {#each ['timeline', 'grid', 'masonry'] as mode}
        <button
          class="px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-200
            {$viewMode === mode ? 'neu-raised' : ''}"
          style="
            background: {$viewMode === mode ? 'var(--color-bg)' : 'transparent'};
            color: {$viewMode === mode ? 'var(--color-accent)' : 'var(--color-text-muted)'};
          "
          onclick={() => viewMode.set(mode as any)}
        >
          {mode === 'timeline' ? '≡' : mode === 'grid' ? '⊞' : '⊟'}
          {mode.charAt(0).toUpperCase() + mode.slice(1)}
        </button>
      {/each}

      <!-- Size slider -->
      <input
        type="range"
        min="100"
        max="300"
        bind:value={gridSize}
        class="w-16 ml-2 accent-[var(--color-accent)]"
      />
    </div>
  </div>

  <!-- Loading state -->
  {#if $isLoading}
    <div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax({gridSize}px, 1fr));">
      {#each Array(20) as _, i}
        <PhotoSkeleton delay={i * 30} />
      {/each}
    </div>

  <!-- Empty state -->
  {:else if $photos.length === 0}
    <div class="flex flex-col items-center justify-center h-[60vh] gap-4 animate-fade-in">
      <div class="w-20 h-20 rounded-2xl flex items-center justify-center text-3xl neu-raised"
        style="background: var(--color-surface);">
        ◈
      </div>
      <p class="text-base font-medium" style="color: var(--color-text-secondary);">
        {$activeFolder ? 'No photos found' : 'Add a folder to get started'}
      </p>
      <p class="text-sm" style="color: var(--color-text-muted);">
        {$activeFolder ? 'Try adding a folder with images' : 'Click "Add Folder" in the sidebar'}
      </p>
    </div>

  <!-- Timeline view -->
  {:else if $viewMode === 'timeline'}
    {#each $photosByDate as group, gi}
      <div class="mb-6 animate-fade-in" style="animation-delay: {gi * 50}ms;">
        <div class="glass inline-block px-3 py-1.5 rounded-full mb-3 text-xs font-medium"
          style="color: var(--color-text-secondary);"
          data-date={group.date}>
          {group.date}
          <span style="color: var(--color-text-muted);">· {group.photos.length}</span>
        </div>
        <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax({gridSize}px, 1fr));">
          {#each group.photos as photo, pi (photo.id)}
            <button
              class="relative aspect-square overflow-hidden rounded-xl transition-all duration-200 group cursor-pointer"
              style="box-shadow: var(--shadow-neu-soft);"
              animate:flip={{ duration: 300 }}
              in:scale={{ start: 0.9, duration: 300, delay: pi * 30 }}
              onclick={() => selectPhoto(photo)}
              onkeydown={(e) => e.key === 'Enter' && selectPhoto(photo)}
            >
              {#if !loadedHashes.has(photo.content_hash)}
                <div class="absolute inset-0 animate-shimmer rounded-xl"></div>
              {/if}
              <img
                src={getThumbnailSrc(photo)}
                alt={photo.filename}
                class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-110"
                loading="lazy"
                onload={() => onImageLoad(photo.content_hash)}
                onerror={(e) => handleImageError(e, photo)}
              />
              <!-- Hover overlay -->
              <div class="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent
                opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-xl">
                <div class="absolute bottom-2 left-2 right-2">
                  <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
                </div>
              </div>
              {#if photo.media_type === 'video'}
                <div class="absolute top-2 right-2 glass w-6 h-6 rounded-full flex items-center justify-center text-[10px]">
                  ▶
                </div>
              {/if}
            </button>
          {/each}
        </div>
      </div>
    {/each}

  <!-- Grid view -->
  {:else if $viewMode === 'grid'}
    <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax({gridSize}px, 1fr));">
      {#each $photos as photo, i (photo.id)}
        <button
          class="photo-card relative aspect-square overflow-hidden rounded-2xl group cursor-pointer"
          style="box-shadow: var(--shadow-neu-soft);"
          animate:flip={{ duration: 300 }}
          in:scale={{ start: 0.9, duration: 300, delay: Math.min(i * 20, 500) }}
          onclick={() => selectPhoto(photo)}
          onkeydown={(e) => e.key === 'Enter' && selectPhoto(photo)}
        >
          {#if !loadedHashes.has(photo.content_hash)}
            <div class="absolute inset-0 animate-shimmer rounded-xl"></div>
          {/if}
          <img
            src={getThumbnailSrc(photo)}
            alt={photo.filename}
            class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-110"
            loading="lazy"
            onload={() => onImageLoad(photo.content_hash)}
            onerror={(e) => {
              const img = e.target as HTMLImageElement;
              img.src = getFullSrc(photo);
            }}
          />
          <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-black/10 to-transparent
            opacity-0 group-hover:opacity-100 transition-all duration-300 rounded-2xl">
            <div class="absolute bottom-2 left-2 right-2">
              <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
            </div>
          </div>
          {#if photo.media_type === 'video'}
            <div class="absolute top-2 right-2 glass w-6 h-6 rounded-full flex items-center justify-center text-[10px]">
              ▶
            </div>
          {/if}
        </button>
      {/each}
    </div>

  <!-- Masonry view -->
  {:else}
    <div class="columns-[{gridSize}px] gap-1.5">
      {#each $photos as photo, i (photo.id)}
        <button
          class="relative overflow-hidden rounded-xl mb-1.5 w-full transition-all duration-200 group cursor-pointer break-inside-avoid"
          style="box-shadow: var(--shadow-neu-soft);"
          in:fly={{ y: 20, duration: 300, delay: Math.min(i * 20, 500) }}
          onclick={() => selectPhoto(photo)}
          onkeydown={(e) => e.key === 'Enter' && selectPhoto(photo)}
        >
          <img
            src={getThumbnailSrc(photo)}
            alt={photo.filename}
            class="w-full h-auto transition-transform duration-300 group-hover:scale-105"
            loading="lazy"
            onload={() => onImageLoad(photo.content_hash)}
            onerror={(e) => {
              const img = e.target as HTMLImageElement;
              img.src = getFullSrc(photo);
            }}
          />
          <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-black/10 to-transparent
            opacity-0 group-hover:opacity-100 transition-all duration-300 rounded-2xl">
            <div class="absolute bottom-2 left-2 right-2">
              <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
            </div>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>
