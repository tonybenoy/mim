<script lang="ts">
  import { photos, activeFolder, photosByDate, isLoading } from '$lib/stores/photos';
  import { viewMode, selectedPhotoId, isScanning, selectedGridIndex, selectionMode, selectedPhotoIds } from '$lib/stores/ui';
  import { getPhotos, getPhotoCount, toggleFavorite, setRating, trashPhoto, restorePhoto, ensureThumbnail } from '$lib/api/photos';
  import { addToAlbum, getAlbums, type Album } from '$lib/api/albums';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { fly, fade, scale } from 'svelte/transition';
  import { pushAction } from '$lib/undo';
  import { tStore } from '$lib/i18n';
  import PhotoSkeleton from './PhotoSkeleton.svelte';
  import CalendarView from './CalendarView.svelte';
  import type { Photo } from '$lib/api/photos';

  // ── Layout state ──────────────────────────────────────
  let gridSize = $state(180);
  let loadedHashes = $state(new Set<string>());

  // ── Virtual scroll state ──────────────────────────────
  let scrollContainer: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let containerWidth = $state(800);
  let containerHeight = $state(600);
  const GAP = 6; // gap-1.5 = 6px
  const BUFFER_ROWS = 3;

  let cols = $derived(Math.max(1, Math.floor((containerWidth + GAP) / (gridSize + GAP))));
  let rowHeight = $derived(gridSize + GAP);
  let totalRows = $derived(Math.ceil($photos.length / cols));
  let totalHeight = $derived(totalRows * rowHeight);
  let startRow = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - BUFFER_ROWS));
  let endRow = $derived(Math.min(totalRows, Math.ceil((scrollTop + containerHeight) / rowHeight) + BUFFER_ROWS));
  let visiblePhotos = $derived($photos.slice(startRow * cols, endRow * cols));
  let offsetTop = $derived(startRow * rowHeight);

  // ── Folder version guard (race condition prevention) ──
  let folderPathForPhotos = $state('');

  // ── Pagination state ──────────────────────────────────
  const PAGE_SIZE = 200;
  let totalCount = $state(0);
  let loadingMore = $state(false);

  // ── Thumbnail self-healing ────────────────────────────
  // Non-reactive to avoid circular dependency in the $effect that reads AND writes it
  const _pendingThumbnails = new Set<string>();
  const MAX_CONCURRENT_THUMBNAILS = 6;

  function normalizePath(p: string): string {
    return p.replace(/\\/g, '/');
  }

  // ── Folder change: load first page ────────────────────
  // Use a non-reactive variable for version tracking to avoid $effect re-triggers
  let _folderVersion = 0;

  $effect(() => {
    const folder = $activeFolder;
    if (folder) {
      // Increment version — stale async callbacks compare against this
      _folderVersion++;
      const thisVersion = _folderVersion;

      // Wipe synchronously — no images in DOM during transition
      folderPathForPhotos = '';
      photos.set([]);
      loadedHashes = new Set();
      _pendingThumbnails.clear();
      totalCount = 0;
      scrollTop = 0;
      isLoading.set(true);

      // Load first page + count in parallel
      Promise.all([
        getPhotos(folder.path, PAGE_SIZE, 0, 'photo'),
        getPhotoCount(folder.path),
      ]).then(([p, count]) => {
        if (_folderVersion !== thisVersion) return; // folder changed, discard
        folderPathForPhotos = folder.path;
        photos.set(p);
        totalCount = count;
        isLoading.set(false);
      }).catch(() => {
        if (_folderVersion === thisVersion) isLoading.set(false);
      });
    }
  });

  // ── Pagination: load more when scrolling near bottom ──
  function maybeLoadMore() {
    if (loadingMore || !folderPathForPhotos) return;
    const loaded = $photos.length;
    if (loaded >= totalCount) return; // all loaded
    // Trigger when within 2 screens of the end
    const threshold = (endRow + BUFFER_ROWS * 2) * cols;
    if (threshold >= loaded) {
      loadingMore = true;
      const thisVersion = _folderVersion;
      getPhotos(folderPathForPhotos, PAGE_SIZE, loaded, 'photo').then(more => {
        if (_folderVersion !== thisVersion) return;
        if (more.length > 0) {
          photos.update(existing => [...existing, ...more]);
        } else {
          totalCount = loaded; // no more to load
        }
        loadingMore = false;
      }).catch(() => { loadingMore = false; });
    }
  }

  // ── Image source: thumbnail or placeholder ────────────
  function getImageSrc(photo: Photo): string {
    if (!folderPathForPhotos) return '';
    if (photo.thumbnail_generated) {
      const prefix = photo.content_hash.slice(0, 2);
      return convertFileSrc(normalizePath(
        `${folderPathForPhotos}/.mim/thumbnails/${prefix}/${photo.content_hash}_256.webp`
      ));
    }
    // No thumbnail — return empty, self-healing will kick in
    return '';
  }

  function getFullSrc(photo: Photo): string {
    if (!folderPathForPhotos) return '';
    return convertFileSrc(normalizePath(`${folderPathForPhotos}/${photo.relative_path}`));
  }

  // ── Self-healing: request thumbnails for visible photos ──
  // Called on scroll and after photos load to generate missing thumbnails
  function requestMissingThumbnails() {
    if (!folderPathForPhotos) return;
    const folderPath = folderPathForPhotos;
    for (const photo of visiblePhotos) {
      if (photo.thumbnail_generated) continue;
      if (photo.media_type === 'video') continue; // skip videos
      if (_pendingThumbnails.has(photo.id)) continue;
      if (_pendingThumbnails.size >= MAX_CONCURRENT_THUMBNAILS) break;

      _pendingThumbnails.add(photo.id);
      ensureThumbnail(folderPath, photo.id)
        .catch(() => {})
        .finally(() => {
          _pendingThumbnails.delete(photo.id);
          // After one finishes, try to start more
          requestMissingThumbnails();
        });
    }
  }

  // Trigger thumbnail generation when photos change or scroll happens
  $effect(() => {
    // Read reactive deps to subscribe: photos, visible range
    const _ = [visiblePhotos.length, folderPathForPhotos];
    requestMissingThumbnails();
  });

  // ── Listen for thumbnail-ready events ─────────────────
  $effect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ photo_id: string }>('thumbnail-ready', (event) => {
      const { photo_id } = event.payload;
      photos.update(list =>
        list.map(p => p.id === photo_id ? { ...p, thumbnail_generated: true } : p)
      );
    }).then(fn => { unlisten = fn; });
    return () => { unlisten?.(); };
  });

  // ── Error handler: never cascade, trigger self-heal ───
  function handleImageError(e: Event, photo: Photo) {
    if (!folderPathForPhotos) return;
    const img = e.target as HTMLImageElement;
    if (img.dataset.failed) return; // already handled
    img.dataset.failed = '1';
    // Mark as not generated so self-heal picks it up
    if (photo.thumbnail_generated) {
      photos.update(list =>
        list.map(p => p.id === photo.id ? { ...p, thumbnail_generated: false } : p)
      );
    }
  }

  // ── ResizeObserver for container dimensions ────────────
  $effect(() => {
    if (!scrollContainer) return;
    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        containerWidth = entry.contentRect.width;
        containerHeight = entry.contentRect.height;
      }
    });
    observer.observe(scrollContainer);
    return () => observer.disconnect();
  });

  function handleScroll(e: Event) {
    const el = e.target as HTMLElement;
    scrollTop = el.scrollTop;

    // Pagination: load more when near bottom
    maybeLoadMore();

    // Generate missing thumbnails for newly visible photos
    requestMissingThumbnails();

    // Scroll date indicator
    const headers = el.querySelectorAll('[data-date]');
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

  // ── Photo interaction ─────────────────────────────────
  function selectPhoto(photo: Photo) {
    selectedPhotoId.set(photo.id);
  }

  function onImageLoad(hash: string) {
    loadedHashes = new Set([...loadedHashes, hash]);
  }

  // ── Batch selection ───────────────────────────────────
  let lastSelectedIndex = $state(-1);
  let showAlbumPicker = $state(false);
  let albumList = $state<Album[]>([]);
  let batchRating = $state(0);
  let showBatchRating = $state(false);

  function toggleSelection(photo: Photo, index: number, e?: MouseEvent) {
    const current = new Set($selectedPhotoIds);
    if (e?.shiftKey && lastSelectedIndex >= 0) {
      const start = Math.min(lastSelectedIndex, index);
      const end = Math.max(lastSelectedIndex, index);
      for (let i = start; i <= end; i++) {
        current.add($photos[i].id);
      }
    } else {
      if (current.has(photo.id)) current.delete(photo.id);
      else current.add(photo.id);
    }
    selectedPhotoIds.set(current);
    lastSelectedIndex = index;
  }

  function selectAll() { selectedPhotoIds.set(new Set($photos.map(p => p.id))); }
  function clearSelection() { selectedPhotoIds.set(new Set()); lastSelectedIndex = -1; }
  function exitSelectionMode() { selectionMode.set(false); clearSelection(); }

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
      try { await addToAlbum($activeFolder.path, albumId, id); } catch {}
    }
    showAlbumPicker = false;
    clearSelection();
  }

  // ── Scroll date indicator ─────────────────────────────
  let scrollDate = $state('');
  let showScrollDate = $state(false);
  let scrollTimeout: ReturnType<typeof setTimeout> | null = null;

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return 'Unknown Date';
    return new Date(dateStr).toLocaleDateString('en-US', {
      weekday: 'long', year: 'numeric', month: 'long', day: 'numeric',
    });
  }

  // ── Undo-able actions ─────────────────────────────────
  async function handleToggleFavorite(e: Event, photo: Photo) {
    e.stopPropagation();
    if (!$activeFolder) return;
    const folder = $activeFolder.path;
    const oldVal = photo.is_favorite;
    const newVal = await toggleFavorite(folder, photo.id);
    photos.update(list => list.map(p => p.id === photo.id ? { ...p, is_favorite: newVal } : p));
    const photoId = photo.id;
    pushAction({
      type: 'toggle_favorite',
      description: $tStore('undo.toggle_favorite'),
      forward: async () => {
        await toggleFavorite(folder, photoId);
        photos.update(list => list.map(p => p.id === photoId ? { ...p, is_favorite: newVal } : p));
      },
      backward: async () => {
        await toggleFavorite(folder, photoId);
        photos.update(list => list.map(p => p.id === photoId ? { ...p, is_favorite: oldVal } : p));
      },
    });
  }

  async function handleSetRating(photo: Photo, rating: number) {
    if (!$activeFolder) return;
    const folder = $activeFolder.path;
    const oldRating = photo.rating;
    await setRating(folder, photo.id, rating);
    photos.update(list => list.map(p => p.id === photo.id ? { ...p, rating } : p));
    const photoId = photo.id;
    pushAction({
      type: 'set_rating',
      description: $tStore('undo.set_rating'),
      forward: async () => {
        await setRating(folder, photoId, rating);
        photos.update(list => list.map(p => p.id === photoId ? { ...p, rating } : p));
      },
      backward: async () => {
        await setRating(folder, photoId, oldRating);
        photos.update(list => list.map(p => p.id === photoId ? { ...p, rating: oldRating } : p));
      },
    });
  }

  async function handleTrash(photo: Photo) {
    if (!$activeFolder) return;
    const folder = $activeFolder.path;
    const photoId = photo.id;
    const photoCopy = { ...photo };
    await trashPhoto(folder, photoId);
    photos.update(list => list.filter(p => p.id !== photoId));
    pushAction({
      type: 'trash_photo',
      description: $tStore('undo.trash_photo'),
      forward: async () => {
        await trashPhoto(folder, photoId);
        photos.update(list => list.filter(p => p.id !== photoId));
      },
      backward: async () => {
        await restorePhoto(folder, photoId);
        photos.update(list => [...list, { ...photoCopy, is_trashed: false }]);
      },
    });
  }

  function handleGridKeydown(e: KeyboardEvent) {
    if ($selectedPhotoId) return;
    if ((e.target as HTMLElement)?.tagName === 'INPUT' || (e.target as HTMLElement)?.tagName === 'TEXTAREA') return;
    const currentPhotos = $photos;
    let idx = $selectedGridIndex;
    if (e.key === 'j' || e.key === 'ArrowDown') { e.preventDefault(); idx = Math.min(idx + 1, currentPhotos.length - 1); selectedGridIndex.set(idx); }
    else if (e.key === 'k' || e.key === 'ArrowUp') { e.preventDefault(); idx = Math.max(idx - 1, 0); selectedGridIndex.set(idx); }
    else if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); if (idx >= 0 && idx < currentPhotos.length) selectedPhotoId.set(currentPhotos[idx].id); }
    else if (e.key === 'f' && idx >= 0 && idx < currentPhotos.length) { e.preventDefault(); handleToggleFavorite(new Event('click'), currentPhotos[idx]); }
    else if (e.key >= '1' && e.key <= '5' && idx >= 0 && idx < currentPhotos.length) { e.preventDefault(); handleSetRating(currentPhotos[idx], parseInt(e.key)); }
    else if (e.key === 'Delete' && idx >= 0 && idx < currentPhotos.length) { e.preventDefault(); handleTrash(currentPhotos[idx]); }
  }
</script>

<svelte:window onkeydown={handleGridKeydown} />

<div
  class="flex-1 overflow-y-auto p-4 relative"
  style="padding-bottom: 80px;"
  onscroll={handleScroll}
  bind:this={scrollContainer}
>
  <!-- Floating scroll date indicator -->
  {#if showScrollDate && scrollDate}
    <div class="fixed z-50 pointer-events-none" style="top: 70px; right: 24px;"
      in:fade={{ duration: 150 }} out:fade={{ duration: 300 }}>
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
        {$activeFolder?.label || $activeFolder?.path.split(/[/\\]/).pop() || $tStore('grid.select_folder')}
      </h2>
      {#if totalCount > 0}
        <span class="text-xs px-2 py-0.5 rounded-full"
          style="background: var(--color-accent-soft); color: var(--color-accent);">
          {totalCount} {$tStore('grid.photos')}
        </span>
      {/if}
      {#if $isScanning}
        <div class="flex items-center gap-1.5 text-xs animate-pulse-glow px-2 py-0.5 rounded-full"
          style="color: var(--color-accent);">
          <span class="w-1.5 h-1.5 rounded-full" style="background: var(--color-accent);"></span>
          {$tStore('grid.scanning')}
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <button
        class="px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all duration-200"
        style="background: {$selectionMode ? 'var(--color-accent-soft)' : 'var(--color-surface)'}; color: {$selectionMode ? 'var(--color-accent)' : 'var(--color-text-muted)'};"
        onclick={() => { if ($selectionMode) { exitSelectionMode(); } else { selectionMode.set(true); } }}
        title="Toggle selection mode"
      >
        {'\u2611'}
      </button>

      <div class="flex items-center gap-1 p-1 rounded-xl" style="background: var(--color-surface);">
        {#each [['timeline', '\u2261'], ['grid', '\u229E'], ['masonry', '\u229F'], ['calendar', '\u25A6']] as [mode, icon]}
          <button
            class="px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 {$viewMode === mode ? 'neu-raised' : ''}"
            style="background: {$viewMode === mode ? 'var(--color-bg)' : 'transparent'}; color: {$viewMode === mode ? 'var(--color-accent)' : 'var(--color-text-muted)'};"
            onclick={() => viewMode.set(mode as any)}
          >
            {icon} {mode.charAt(0).toUpperCase() + mode.slice(1)}
          </button>
        {/each}
        <input type="range" min="100" max="300" bind:value={gridSize} class="w-16 ml-2 accent-[var(--color-accent)]" />
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
        {'\u25C8'}
      </div>
      <p class="text-base font-medium" style="color: var(--color-text-secondary);">
        {$activeFolder ? $tStore('grid.no_photos') : $tStore('grid.add_folder_hint')}
      </p>
      <p class="text-sm" style="color: var(--color-text-muted);">
        {$activeFolder ? $tStore('grid.no_photos_hint') : $tStore('grid.add_folder_sidebar')}
      </p>
    </div>

  <!-- Timeline view -->
  {:else if $viewMode === 'timeline'}
    {#each $photosByDate as group, gi}
      <div class="mb-6 animate-fade-in" style="animation-delay: {gi * 50}ms;">
        <div class="glass inline-block px-3 py-1.5 rounded-full mb-3 text-xs font-medium"
          style="color: var(--color-text-secondary);" data-date={group.date}>
          {group.date}
          <span style="color: var(--color-text-muted);">{'\u00B7'} {group.photos.length}</span>
        </div>
        <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax({gridSize}px, 1fr));">
          {#each group.photos as photo (photo.id)}
            <button
              class="relative aspect-square overflow-hidden rounded-xl transition-all duration-200 group cursor-pointer"
              style="box-shadow: var(--shadow-neu-soft);"
              onclick={() => selectPhoto(photo)}
            >
              {#if !loadedHashes.has(photo.content_hash) && getImageSrc(photo)}
                <div class="absolute inset-0 animate-shimmer rounded-xl"></div>
              {/if}
              {#if getImageSrc(photo)}
                <img
                  src={getImageSrc(photo)}
                  alt={photo.filename}
                  class="w-full h-full object-cover"
                  loading="lazy"
                  onload={() => onImageLoad(photo.content_hash)}
                  onerror={(e) => handleImageError(e, photo)}
                />
              {:else}
                <!-- Placeholder while thumbnail generates -->
                <div class="absolute inset-0 flex items-center justify-center rounded-xl"
                  style="background: var(--color-surface);">
                  <div class="w-6 h-6 rounded-full border-2 border-t-transparent animate-spin"
                    style="border-color: var(--color-accent); border-top-color: transparent;"></div>
                </div>
              {/if}
              <div class="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent
                opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-xl">
                <div class="absolute bottom-2 left-2 right-2">
                  <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
                </div>
              </div>
              {#if photo.media_type === 'video'}
                <div class="absolute top-2 right-2 glass w-6 h-6 rounded-full flex items-center justify-center text-[10px]">
                  {'\u25B6'}
                </div>
              {/if}
            </button>
          {/each}
        </div>
      </div>
    {/each}

  <!-- Grid view (VIRTUALIZED) -->
  {:else if $viewMode === 'grid'}
    <div style="height: {totalHeight}px; position: relative;">
      <div
        class="grid gap-1.5"
        style="grid-template-columns: repeat(auto-fill, minmax({gridSize}px, 1fr)); position: absolute; top: {offsetTop}px; left: 0; right: 0;"
      >
        {#each visiblePhotos as photo, vi (photo.id)}
          {@const globalIndex = startRow * cols + vi}
          <button
            class="photo-card relative aspect-square overflow-hidden rounded-2xl group cursor-pointer"
            style="box-shadow: var(--shadow-neu-soft);"
            onclick={(e) => { if ($selectionMode) { toggleSelection(photo, globalIndex, e); } else { selectPhoto(photo); } }}
          >
            {#if !loadedHashes.has(photo.content_hash) && getImageSrc(photo)}
              <div class="absolute inset-0 animate-shimmer rounded-xl"></div>
            {/if}
            {#if getImageSrc(photo)}
              <img
                src={getImageSrc(photo)}
                alt={photo.filename}
                class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-110"
                loading="lazy"
                onload={() => onImageLoad(photo.content_hash)}
                onerror={(e) => handleImageError(e, photo)}
              />
            {:else}
              <!-- Placeholder while thumbnail generates -->
              <div class="absolute inset-0 flex items-center justify-center rounded-2xl"
                style="background: var(--color-surface);">
                <div class="w-6 h-6 rounded-full border-2 border-t-transparent animate-spin"
                  style="border-color: var(--color-accent); border-top-color: transparent;"></div>
              </div>
            {/if}
            <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-black/10 to-transparent
              opacity-0 group-hover:opacity-100 transition-all duration-300 rounded-2xl">
              <div class="absolute bottom-2 left-2 right-2">
                <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
              </div>
              {#if !$selectionMode}
                <button
                  class="absolute top-2 left-2 w-7 h-7 rounded-full flex items-center justify-center text-sm transition-all duration-200 hover:scale-125"
                  style="background: rgba(0,0,0,0.4); color: {photo.is_favorite ? '#f43f5e' : '#fff'};"
                  onclick={(e) => handleToggleFavorite(e, photo)}
                >
                  {photo.is_favorite ? '\u2665' : '\u2661'}
                </button>
              {/if}
            </div>
            {#if $selectionMode}
              <div class="absolute top-2 left-2 w-6 h-6 rounded-md flex items-center justify-center text-xs transition-all"
                style="background: {$selectedPhotoIds.has(photo.id) ? 'var(--color-accent)' : 'rgba(0,0,0,0.4)'}; color: #fff;">
                {$selectedPhotoIds.has(photo.id) ? '\u2713' : ''}
              </div>
              {#if $selectedPhotoIds.has(photo.id)}
                <div class="absolute inset-0 rounded-2xl pointer-events-none" style="box-shadow: inset 0 0 0 3px var(--color-accent);"></div>
              {/if}
            {:else}
              {#if photo.is_favorite}
                <div class="absolute top-2 left-2 w-5 h-5 rounded-full flex items-center justify-center text-[10px] group-hover:opacity-0 transition-opacity"
                  style="background: rgba(244,63,94,0.8); color: #fff;">
                  {'\u2665'}
                </div>
              {/if}
            {/if}
            {#if photo.rating > 0}
              <div class="absolute top-2 right-2 flex gap-0.5 group-hover:opacity-0 transition-opacity">
                {#each Array(photo.rating) as _}
                  <span class="text-[8px] drop-shadow-lg" style="color: #fbbf24;">{'\u2605'}</span>
                {/each}
              </div>
            {/if}
            {#if photo.media_type === 'video'}
              <div class="absolute bottom-2 right-2 glass w-6 h-6 rounded-full flex items-center justify-center text-[10px]">
                {'\u25B6'}
              </div>
            {/if}
            {#if !$selectionMode && $selectedGridIndex === globalIndex}
              <div class="absolute inset-0 rounded-2xl pointer-events-none" style="box-shadow: inset 0 0 0 2px var(--color-accent);"></div>
            {/if}
          </button>
        {/each}
      </div>
    </div>
    {#if loadingMore}
      <div class="flex justify-center py-4">
        <div class="w-6 h-6 rounded-full border-2 border-t-transparent animate-spin"
          style="border-color: var(--color-accent); border-top-color: transparent;"></div>
      </div>
    {/if}

  <!-- Masonry view (capped) -->
  {:else if $viewMode === 'masonry'}
    <div class="columns-[{gridSize}px] gap-1.5">
      {#each $photos.slice(0, 200) as photo (photo.id)}
        <button
          class="relative overflow-hidden rounded-xl mb-1.5 w-full transition-all duration-200 group cursor-pointer break-inside-avoid"
          style="box-shadow: var(--shadow-neu-soft);"
          onclick={() => selectPhoto(photo)}
        >
          {#if getImageSrc(photo)}
            <img
              src={getImageSrc(photo)}
              alt={photo.filename}
              class="w-full h-auto transition-transform duration-300 group-hover:scale-105"
              loading="lazy"
              onload={() => onImageLoad(photo.content_hash)}
              onerror={(e) => handleImageError(e, photo)}
            />
          {:else}
            <div class="w-full aspect-square flex items-center justify-center rounded-xl"
              style="background: var(--color-surface);">
              <div class="w-6 h-6 rounded-full border-2 border-t-transparent animate-spin"
                style="border-color: var(--color-accent); border-top-color: transparent;"></div>
            </div>
          {/if}
          <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-black/10 to-transparent
            opacity-0 group-hover:opacity-100 transition-all duration-300 rounded-2xl">
            <div class="absolute bottom-2 left-2 right-2">
              <p class="text-white text-[10px] truncate drop-shadow-lg">{photo.filename}</p>
            </div>
          </div>
        </button>
      {/each}
    </div>
    {#if $photos.length > 200}
      <p class="text-center text-xs py-4" style="color: var(--color-text-muted);">
        Showing 200 of {$photos.length} photos. Switch to Grid view for full virtual scrolling.
      </p>
    {/if}

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
        {$selectedPhotoIds.size} {$tStore('grid.selected')}
      </span>
      <div class="flex-1"></div>
      <button class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: var(--color-surface); color: var(--color-text-secondary);" onclick={selectAll}>
        {$tStore('grid.select_all')}
      </button>
      <button class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: var(--color-surface); color: #f43f5e;" onclick={batchFavorite} title="Favorite all selected">
        {'\u2665'} {$tStore('action.favorite')}
      </button>
      <div class="relative">
        <button class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
          style="background: var(--color-surface); color: #fbbf24;"
          onclick={() => showBatchRating = !showBatchRating} title="Rate all selected">
          {'\u2605'} {$tStore('action.rate')}
        </button>
        {#if showBatchRating}
          <div class="absolute bottom-full mb-2 left-1/2 -translate-x-1/2 glass-heavy rounded-xl p-2 flex gap-1"
            style="box-shadow: 0 4px 16px rgba(0,0,0,0.2);">
            {#each [1, 2, 3, 4, 5] as star}
              <button class="text-lg transition-all hover:scale-125" style="color: #fbbf24;" onclick={() => batchRate(star)}>
                {'\u2605'}
              </button>
            {/each}
          </div>
        {/if}
      </div>
      <button class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: var(--color-surface); color: var(--color-accent);" onclick={openAlbumPicker} title="Add all to album">
        {'\u25A3'} {$tStore('action.album')}
      </button>
      <button class="text-xs px-3 py-1.5 rounded-lg transition-all hover:scale-105"
        style="background: #fee2e2; color: #dc2626;" onclick={batchTrash} title="Trash all selected">
        {'\u2715'} {$tStore('action.trash')}
      </button>
      <button class="text-xs px-2 py-1.5 rounded-lg" style="color: var(--color-text-muted);" onclick={clearSelection}>
        {$tStore('action.clear')}
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
      role="dialog" aria-modal="true" tabindex="-1"
    >
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="glass-heavy w-[340px] max-h-[60vh] rounded-2xl p-5"
        style="box-shadow: 0 20px 60px rgba(0,0,0,0.3);"
        onclick={(e) => e.stopPropagation()}
        transition:fly={{ y: 20, duration: 300 }}>
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
                onclick={() => batchAddToAlbum(album.id)}>
                <span style="color: var(--color-accent);">{'\u25A3'}</span>
                <span class="flex-1 truncate">{album.name}</span>
                <span class="text-[10px]" style="color: var(--color-text-muted);">{album.photo_count}</span>
              </button>
            {/each}
          </div>
        {/if}
        <button class="w-full mt-3 py-2 rounded-xl text-xs"
          style="background: var(--color-surface); color: var(--color-text-muted);"
          onclick={() => showAlbumPicker = false}>Cancel</button>
      </div>
    </div>
  {/if}
</div>
