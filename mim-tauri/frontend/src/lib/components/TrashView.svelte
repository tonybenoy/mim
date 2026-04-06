<script lang="ts">
  import { getTrashed, restorePhoto, emptyTrash, type Photo } from '$lib/api/photos';
  import { activeFolder } from '$lib/stores/photos';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, fly, scale } from 'svelte/transition';

  let trashedPhotos = $state<Photo[]>([]);
  let isLoading = $state(true);
  let isEmptying = $state(false);
  let showConfirm = $state(false);

  $effect(() => {
    if ($activeFolder) {
      loadTrashed();
    }
  });

  async function loadTrashed() {
    if (!$activeFolder) return;
    isLoading = true;
    try {
      trashedPhotos = await getTrashed($activeFolder.path);
    } catch (e) {
      console.error('Failed to load trashed photos:', e);
    }
    isLoading = false;
  }

  async function handleRestore(photo: Photo) {
    if (!$activeFolder) return;
    try {
      await restorePhoto($activeFolder.path, photo.id);
      trashedPhotos = trashedPhotos.filter(p => p.id !== photo.id);
    } catch (e) {
      console.error('Failed to restore photo:', e);
    }
  }

  async function handleEmptyTrash() {
    if (!$activeFolder) return;
    isEmptying = true;
    try {
      const deleted = await emptyTrash($activeFolder.path);
      trashedPhotos = [];
      showConfirm = false;
    } catch (e) {
      console.error('Failed to empty trash:', e);
    }
    isEmptying = false;
  }

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
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  }
</script>

<div class="flex-1 overflow-y-auto p-4" style="padding-bottom: 80px;">
  <div class="flex items-center justify-between mb-4" in:fade={{ duration: 200 }}>
    <div class="flex items-center gap-3">
      <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">Trash</h2>
      {#if trashedPhotos.length > 0}
        <span class="text-xs px-2 py-0.5 rounded-full"
          style="background: #fee2e2; color: #dc2626;">
          {trashedPhotos.length} {trashedPhotos.length === 1 ? 'photo' : 'photos'}
        </span>
      {/if}
    </div>
    {#if trashedPhotos.length > 0}
      <button
        class="text-xs px-3 py-1.5 rounded-lg font-medium transition-all hover:scale-105"
        style="background: #fee2e2; color: #dc2626;"
        onclick={() => showConfirm = true}
      >
        Empty Trash
      </button>
    {/if}
  </div>

  <!-- Empty trash confirmation modal -->
  {#if showConfirm}
    <div
      class="fixed inset-0 z-[200] flex items-center justify-center"
      style="background: rgba(0,0,0,0.6); backdrop-filter: blur(10px);"
      transition:fade={{ duration: 200 }}
      onclick={() => showConfirm = false}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="glass-heavy w-[380px] rounded-2xl p-6"
        style="box-shadow: 0 20px 60px rgba(0,0,0,0.3);"
        onclick={(e) => e.stopPropagation()}
        transition:fly={{ y: 20, duration: 300 }}
      >
        <h3 class="text-base font-semibold mb-2" style="color: var(--color-text-primary);">
          Empty Trash?
        </h3>
        <p class="text-sm mb-4" style="color: var(--color-text-muted);">
          This will permanently delete {trashedPhotos.length} {trashedPhotos.length === 1 ? 'photo' : 'photos'}.
          This action cannot be undone.
        </p>
        <div class="flex gap-2 justify-end">
          <button
            class="px-4 py-2 rounded-xl text-sm"
            style="background: var(--color-surface); color: var(--color-text-secondary);"
            onclick={() => showConfirm = false}
          >
            Cancel
          </button>
          <button
            class="px-4 py-2 rounded-xl text-sm font-medium"
            style="background: #dc2626; color: #fff;"
            onclick={handleEmptyTrash}
            disabled={isEmptying}
          >
            {isEmptying ? 'Deleting...' : 'Delete All'}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if isLoading}
    <div class="text-center py-20" style="color: var(--color-text-muted);">
      <span class="inline-block animate-spin text-xl">&#x25CC;</span>
    </div>
  {:else if trashedPhotos.length === 0}
    <div class="flex flex-col items-center justify-center h-[60vh] gap-4 animate-fade-in">
      <div class="w-20 h-20 rounded-2xl flex items-center justify-center text-3xl neu-raised"
        style="background: var(--color-surface);">
        &#x2298;
      </div>
      <p class="text-base font-medium" style="color: var(--color-text-secondary);">Trash is empty</p>
      <p class="text-sm" style="color: var(--color-text-muted);">Deleted photos will appear here</p>
    </div>
  {:else}
    <div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
      {#each trashedPhotos as photo, i (photo.id)}
        <div
          class="relative aspect-square overflow-hidden rounded-xl group"
          style="box-shadow: var(--shadow-neu-soft); opacity: 0.85;"
          in:scale={{ start: 0.9, duration: 300, delay: Math.min(i * 20, 500) }}
        >
          <img
            src={getThumbnailSrc(photo)}
            alt={photo.filename}
            class="w-full h-full object-cover"
            loading="lazy"
            onerror={(e) => { (e.target as HTMLImageElement).src = getFullSrc(photo); }}
          />
          <!-- Hover overlay -->
          <div class="absolute inset-0 bg-gradient-to-t from-black/60 via-black/20 to-transparent
            opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-xl flex flex-col justify-end p-3">
            <p class="text-white text-[10px] truncate mb-1">{photo.filename}</p>
            {#if photo.trashed_at}
              <p class="text-white/60 text-[9px] mb-2">Trashed {formatDate(photo.trashed_at)}</p>
            {/if}
            <button
              class="w-full py-1.5 rounded-lg text-[11px] font-medium transition-all hover:scale-105"
              style="background: rgba(255,255,255,0.2); color: #fff; backdrop-filter: blur(4px);"
              onclick={() => handleRestore(photo)}
            >
              Restore
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
