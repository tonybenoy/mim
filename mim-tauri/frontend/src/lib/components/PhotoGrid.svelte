<script lang="ts">
  import { photos, activeFolder, photosByDate, isLoading } from '$lib/stores/photos';
  import { viewMode, selectedPhotoId, isScanning, selectedGridIndex, selectionMode, selectedPhotoIds } from '$lib/stores/ui';
  import { getPhotos, toggleFavorite, setRating, trashPhoto } from '$lib/api/photos';
  import { addToAlbum, getAlbums, type Album } from '$lib/api/albums';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { fly, fade, scale } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import PhotoSkeleton from './PhotoSkeleton.svelte';
  import CalendarView from './CalendarView.svelte';
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

  // Batch selection
  let lastSelectedIndex = $state(-1);
  let showAlbumPicker = $state(false);
  let albumList = $state<Album[]>([]);
  let batchRating = $state(0);
  let showBatchRating = $state(false);

  function toggleSelection(photo: Photo, index: number, e?: MouseEvent) {
    const current = new Set($selectedPhotoIds);
    if (e?.shiftKey && lastSelectedIndex >= 0) {
      // Range select
      const start = Math.min(lastSelectedIndex, index);
      const end = Math.max(lastSelectedIndex, index);
      for (let i = start; i <= end; i++) {
        current.add($photos[i].id);
      }
    } else {
      if (current.has(photo.id)) {
        current.delete(photo.id);
      } else {
        current.add(photo.id);
      }
    }
    selectedPhotoIds.set(current);
    lastSelectedIndex = index;
  }

  function selectAll() {
    selectedPhotoIds.set(new Set($photos.map(p => p.id)));
  }

  function clearSelection() {
    selectedPhotoIds.set(new Set());
    lastSelectedIndex = -1;
  }

  function exitSelectionMode() {
    selectionMode.set(false);
    clearSelection();
  }

  async function batchFavorite() {
    if (!$activeFolder) return;
    for (const id of $selectedPhotoIds) {
      const photo = $photos.find(p => p.id === id);
      if (photo && !photo.is_favorite) {
        await toggleFavorite($activeFolder.path, id);
        photos.update(list => list.map(p => p.id === id ? { ...p, is_favorite: true } : p));
      }
    }
    clearSelection();
  }

  async function batchTrash() {
    if (!$activeFolder) return;
    if (!window.confirm(`Move ${$selectedPhotoIds.size} photos to trash?`)) return;
    for (const id of $selectedPhotoIds) {
      await trashPhoto($activeFolder.path, id);
    }
    photos.update(list => list.filter(p => !$selectedPhotoIds.has(p.id)));
    clearSelection();
  }

  async function batchRate(rating: number) {
    if (!$activeFolder) return;
    for (const id of $selectedPhotoIds) {
      await setRating($activeFolder.path, id, rating);
      photos.update(list => list.map(p => p.id === id ? { ...p, rating } : p));
    }
    showBatchRating = false;
    clearSelection();
  }

  async function openAlbumPicker() {
    if (!$activeFolder) return;
    try {
      albumList = await getAlbums($activeFolder.path);
      showAlbumPicker = true;
    } catch (e) {
      console.error('Failed to load albums:', e);
    }
  }

  async function batchAddToAlbum(albumId: string) {
    if (!$activeFolder) return;
    for (const id of $selectedPhotoIds) {
      try {
        await addToAlbum($activeFolder.path, albumId, id);
      } catch {}
    }
    showAlbumPicker = false;
    clearSelection();
  }

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

  async function handleToggleFavorite(e: Event, photo: Photo) {
    e.stopPropagation();
    if (!$activeFolder) return;
    const newVal = await toggleFavorite($activeFolder.path, photo.id);
    photos.update(list => list.map(p => p.id === photo.id ? { ...p, is_favorite: newVal } : p));
  }

  async function handleSetRating(photo: Photo, rating: number) {
    if (!$activeFolder) return;
    await setRating($activeFolder.path, photo.id, rating);
    photos.update(list => list.map(p => p.id === photo.id ? { ...p, rating } : p));
  }

  async function handleTrash(photo: Photo) {
    if (!$activeFolder) return;
    await trashPhoto($activeFolder.path, photo.id);
    photos.update(list => list.filter(p => p.id !== photo.id));
  }

  function handleGridKeydown(e: KeyboardEvent) {
    // Don't handle if detail is open or user is typing
    if ($selectedPhotoId) return;
    if ((e.target as HTMLElement)?.tagName === 'INPUT' || (e.target as HTMLElement)?.tagName === 'TEXTAREA') return;

    const currentPhotos = $photos;
    let idx = $selectedGridIndex;

    if (e.key === 'j' || e.key === 'ArrowDown') {
      e.preventDefault();
      idx = Math.min(idx + 1, currentPhotos.length - 1);
      selectedGridIndex.set(idx);
    } else if (e.key === 'k' || e.key === 'ArrowUp') {
      e.preventDefault();
      idx = Math.max(idx - 1, 0);
      selectedGridIndex.set(idx);
    } else if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      if (idx >= 0 && idx < currentPhotos.length) {
        selectedPhotoId.set(currentPhotos[idx].id);
      }
    } else if (e.key === 'f' && idx >= 0 && idx < currentPhotos.length) {
      e.preventDefault();
      handleToggleFavorite(new Event('click'), currentPhotos[idx]);
    } else if (e.key >= '1' && e.key <= '5' && idx >= 0 && idx < currentPhotos.length) {
      e.preventDefault();
      handleSetRating(currentPhotos[idx], parseInt(e.key));
    } else if (e.key === 'Delete' && idx >= 0 && idx < currentPhotos.length) {
      e.preventDefault();
      handleTrash(currentPhotos[idx]);
    }
  }
</script>

<svelte:window onkeydown={handleGridKeydown} />

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
        {$activeFolder?.label || $activeFolder?.path.split(/[/\\]/).pop() || 'Select a folder'}
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

    <div class="flex items-center gap-2">
      <!-- Selection mode toggle -->
      <button
        class="px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all duration-200"
        style="
          background: {$selectionMode ? 'var(--color-accent-soft)' : 'var(--color-surface)'};
          color: {$selectionMode ? 'var(--color-accent)' : 'var(--color-text-muted)'};
        "
        onclick={() => { if ($selectionMode) { exitSelectionMode(); } else { selectionMode.set(true); } }}
        title="Toggle selection mode"
      >
        &#x2611;
      </button>

      <!-- View switcher -->
      <div class="flex items-center gap-1 p-1 rounded-xl" style="background: var(--color-surface);">
        {#each [['timeline', '\u2261'], ['grid', '\u229E'], ['masonry', '\u229F'], ['calendar', '\u25A6']] as [mode, icon]}
          <button
            class="px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-200
              {$viewMode === mode ? 'neu-raised' : ''}"
            style="
              background: {$viewMode === mode ? 'var(--color-bg)' : 'transparent'};
              color: {$viewMode === mode ? 'var(--color-accent)' : 'var(--color-text-muted)'};
            "
            onclick={() => viewMode.set(mode as any)}
          >
            {icon}
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
          onclick={(e) => { if ($selectionMode) { toggleSelection(photo, i, e); } else { selectPhoto(photo); } }}
          onkeydown={(e) => e.key === 'Enter' && ($selectionMode ? toggleSelection(photo, i) : selectPhoto(photo))}
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
            {#if !$selectionMode}
              <!-- Favorite button on hover -->
              <button
                class="absolute top-2 left-2 w-7 h-7 rounded-full flex items-center justify-center text-sm transition-all duration-200 hover:scale-125"
                style="background: rgba(0,0,0,0.4); color: {photo.is_favorite ? '#f43f5e' : '#fff'};"
                onclick={(e) => handleToggleFavorite(e, photo)}
              >
                {photo.is_favorite ? '\u2665' : '\u2661'}
              </button>
            {/if}
          </div>
          <!-- Selection checkbox -->
          {#if $selectionMode}
            <div class="absolute top-2 left-2 w-6 h-6 rounded-md flex items-center justify-center text-xs transition-all"
              style="background: {$selectedPhotoIds.has(photo.id) ? 'var(--color-accent)' : 'rgba(0,0,0,0.4)'}; color: #fff;">
              {$selectedPhotoIds.has(photo.id) ? '\u2713' : ''}
            </div>
            {#if $selectedPhotoIds.has(photo.id)}
              <div class="absolute inset-0 rounded-2xl pointer-events-none" style="box-shadow: inset 0 0 0 3px var(--color-accent);"></div>
            {/if}
          {:else}
            <!-- Persistent favorite indicator -->
            {#if photo.is_favorite}
              <div class="absolute top-2 left-2 w-5 h-5 rounded-full flex items-center justify-center text-[10px] group-hover:opacity-0 transition-opacity"
                style="background: rgba(244,63,94,0.8); color: #fff;">
                &#x2665;
              </div>
            {/if}
          {/if}
          <!-- Rating stars -->
          {#if photo.rating > 0}
            <div class="absolute top-2 right-2 flex gap-0.5 group-hover:opacity-0 transition-opacity">
              {#each Array(photo.rating) as _}
                <span class="text-[8px] drop-shadow-lg" style="color: #fbbf24;">&#x2605;</span>
              {/each}
            </div>
          {/if}
          {#if photo.media_type === 'video'}
            <div class="absolute bottom-2 right-2 glass w-6 h-6 rounded-full flex items-center justify-center text-[10px]">
              &#x25B6;
            </div>
          {/if}
          <!-- Grid index highlight -->
          {#if !$selectionMode && $selectedGridIndex === i}
            <div class="absolute inset-0 rounded-2xl pointer-events-none" style="box-shadow: inset 0 0 0 2px var(--color-accent);"></div>
          {/if}
        </button>
      {/each}
    </div>

  <!-- Masonry view -->
  {:else if $viewMode === 'masonry'}
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

  <!-- Calendar view -->
  {:else if $viewMode === 'calendar'}
    <CalendarView />
  {/if}

  <!-- Batch action bar -->
  {#if $selectionMode && $selectedPhotoIds.size > 0}
    <div
      class="fixed bottom-[72px] left-1/2 -translate-x-1/2 z-40 glass-heavy rounded-2xl px-4 py-3 flex items-center gap-3"
      style="box-shadow: 0 8px 32px rgba(0,0,0,0.3); min-width: 400px;"
      in:fly={{ y: 20, duration: 250 }}
    >
      <span class="text-xs font-semibold px-2 py-1 rounded-full"
        style="background: var(--color-accent-soft); color: var(--color-accent);">
        {$selectedPhotoIds.size} selected
      </span>

      <div class="flex-1"></div>

      <button
        class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: var(--color-surface); color: var(--color-text-secondary);"
        onclick={selectAll}
      >
        Select All
      </button>

      <button
        class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: var(--color-surface); color: #f43f5e;"
        onclick={batchFavorite}
        title="Favorite all selected"
      >
        &#x2665; Favorite
      </button>

      <div class="relative">
        <button
          class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
          style="background: var(--color-surface); color: #fbbf24;"
          onclick={() => showBatchRating = !showBatchRating}
          title="Rate all selected"
        >
          &#x2605; Rate
        </button>
        {#if showBatchRating}
          <div class="absolute bottom-full mb-2 left-1/2 -translate-x-1/2 glass-heavy rounded-xl p-2 flex gap-1"
            style="box-shadow: 0 4px 16px rgba(0,0,0,0.2);">
            {#each [1, 2, 3, 4, 5] as star}
              <button
                class="text-lg transition-all hover:scale-125"
                style="color: #fbbf24;"
                onclick={() => batchRate(star)}
              >
                &#x2605;
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <button
        class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: var(--color-surface); color: var(--color-accent);"
        onclick={openAlbumPicker}
        title="Add all to album"
      >
        &#x25A3; Album
      </button>

      <button
        class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: #fee2e2; color: #dc2626;"
        onclick={batchTrash}
        title="Trash all selected"
      >
        &#x2715; Trash
      </button>

      <button
        class="text-xs px-2 py-1.5 rounded-lg"
        style="color: var(--color-text-muted);"
        onclick={clearSelection}
      >
        Clear
      </button>
    </div>
  {/if}

  <!-- Album picker modal -->
  {#if showAlbumPicker}
    <div
      class="fixed inset-0 z-[200] flex items-center justify-center"
      style="background: rgba(0,0,0,0.6); backdrop-filter: blur(10px);"
      transition:fade={{ duration: 200 }}
      onclick={() => showAlbumPicker = false}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="glass-heavy w-[340px] max-h-[60vh] rounded-2xl p-5"
        style="box-shadow: 0 20px 60px rgba(0,0,0,0.3);"
        onclick={(e) => e.stopPropagation()}
        transition:fly={{ y: 20, duration: 300 }}
      >
        <h3 class="text-sm font-semibold mb-3" style="color: var(--color-text-primary);">
          Add {$selectedPhotoIds.size} photos to album
        </h3>
        {#if albumList.length === 0}
          <p class="text-xs py-4 text-center" style="color: var(--color-text-muted);">No albums yet. Create one first.</p>
        {:else}
          <div class="space-y-1 max-h-[40vh] overflow-y-auto">
            {#each albumList as album (album.id)}
              <button
                class="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm text-left transition-all hover:scale-[1.02]"
                style="background: var(--color-surface); color: var(--color-text-primary);"
                onclick={() => batchAddToAlbum(album.id)}
              >
                <span style="color: var(--color-accent);">&#x25A3;</span>
                <span class="flex-1 truncate">{album.name}</span>
                <span class="text-[10px]" style="color: var(--color-text-muted);">{album.photo_count}</span>
              </button>
            {/each}
          </div>
        {/if}
        <button
          class="w-full mt-3 py-2 rounded-xl text-xs"
          style="background: var(--color-surface); color: var(--color-text-muted);"
          onclick={() => showAlbumPicker = false}
        >Cancel</button>
      </div>
    </div>
  {/if}
</div>
