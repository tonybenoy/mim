<script lang="ts">
  import { createAlbum, getAlbums, getAlbumPhotos, deleteAlbum, renameAlbum, type Album } from '$lib/api/albums';
  import { activeFolder, photos } from '$lib/stores/photos';
  import { selectedPhotoId } from '$lib/stores/ui';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, fly, scale } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';

  let albums = $state<Album[]>([]);
  let selectedAlbum = $state<Album | null>(null);
  let albumPhotoIds = $state<string[]>([]);
  let isLoading = $state(true);
  let showCreate = $state(false);
  let newName = $state('');
  let editingId = $state<string | null>(null);
  let editName = $state('');

  $effect(() => {
    if ($activeFolder) {
      loadAlbums();
    }
  });

  async function loadAlbums() {
    if (!$activeFolder) return;
    isLoading = true;
    try {
      albums = await getAlbums($activeFolder.path);
    } catch (e) {
      console.error('Failed to load albums:', e);
    }
    isLoading = false;
  }

  async function handleCreate() {
    if (!newName.trim() || !$activeFolder) return;
    try {
      const album = await createAlbum($activeFolder.path, newName.trim());
      albums = [album, ...albums];
      newName = '';
      showCreate = false;
    } catch (e) {
      console.error('Failed to create album:', e);
    }
  }

  async function selectAlbum(album: Album) {
    selectedAlbum = album;
    if (!$activeFolder) return;
    try {
      albumPhotoIds = await getAlbumPhotos($activeFolder.path, album.id);
    } catch (e) {
      console.error('Failed to load album photos:', e);
    }
  }

  async function handleDelete(album: Album) {
    if (!$activeFolder) return;
    if (!window.confirm(`Delete album "${album.name}"?`)) return;
    try {
      await deleteAlbum($activeFolder.path, album.id);
      albums = albums.filter(a => a.id !== album.id);
      if (selectedAlbum?.id === album.id) {
        selectedAlbum = null;
        albumPhotoIds = [];
      }
    } catch (e) {
      console.error('Failed to delete album:', e);
    }
  }

  async function handleRename() {
    if (!editingId || !editName.trim() || !$activeFolder) return;
    try {
      await renameAlbum($activeFolder.path, editingId, editName.trim());
      albums = albums.map(a => a.id === editingId ? { ...a, name: editName.trim() } : a);
      if (selectedAlbum?.id === editingId) selectedAlbum = { ...selectedAlbum, name: editName.trim() };
    } catch (e) {
      console.error('Failed to rename:', e);
    }
    editingId = null;
  }

  function getAlbumPhotosData(): Photo[] {
    return albumPhotoIds
      .map(id => $photos.find(p => p.id === id))
      .filter((p): p is Photo => !!p);
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
</script>

<div class="flex-1 overflow-y-auto p-4" style="padding-bottom: 80px;">
  {#if selectedAlbum}
    <!-- Album detail -->
    <div in:fade={{ duration: 200 }}>
      <div class="flex items-center justify-between mb-4">
        <button
          class="flex items-center gap-2 px-3 py-2 rounded-xl text-sm"
          style="color: var(--color-accent);"
          onclick={() => { selectedAlbum = null; albumPhotoIds = []; }}
        >
          ← Back
        </button>
        <div class="flex gap-2">
          <button
            class="text-xs px-3 py-1.5 rounded-lg"
            style="background: var(--color-surface); color: var(--color-text-secondary);"
            onclick={() => { editingId = selectedAlbum!.id; editName = selectedAlbum!.name; }}
          >
            Rename
          </button>
          <button
            class="text-xs px-3 py-1.5 rounded-lg"
            style="background: #fee2e2; color: #dc2626;"
            onclick={() => handleDelete(selectedAlbum!)}
          >
            Delete
          </button>
        </div>
      </div>

      {#if editingId === selectedAlbum.id}
        <div class="flex gap-2 mb-4">
          <input bind:value={editName} class="flex-1 px-3 py-2 rounded-xl text-sm border-none outline-none"
            style="background: var(--color-surface); color: var(--color-text-primary);"
            onkeydown={(e) => e.key === 'Enter' && handleRename()} />
          <button class="text-xs px-3 py-1.5 rounded-lg" style="color: var(--color-accent);" onclick={handleRename}>Save</button>
          <button class="text-xs px-3 py-1.5 rounded-lg" style="color: var(--color-text-muted);" onclick={() => editingId = null}>Cancel</button>
        </div>
      {:else}
        <h2 class="text-lg font-semibold mb-1" style="color: var(--color-text-primary);">
          {selectedAlbum.name}
        </h2>
      {/if}

      <p class="text-sm mb-4" style="color: var(--color-text-muted);">{albumPhotoIds.length} photos</p>

      {#if albumPhotoIds.length === 0}
        <div class="text-center py-12" style="color: var(--color-text-muted);">
          <p class="text-sm">No photos in this album yet.</p>
          <p class="text-xs mt-1">Add photos from the photo detail view.</p>
        </div>
      {:else}
        <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
          {#each getAlbumPhotosData() as photo, i (photo.id)}
            <button
              class="relative aspect-square overflow-hidden rounded-xl group cursor-pointer"
              style="box-shadow: var(--shadow-neu-soft);"
              in:scale={{ start: 0.9, duration: 300, delay: Math.min(i * 20, 500) }}
              onclick={() => selectedPhotoId.set(photo.id)}
            >
              <img src={getThumbnailSrc(photo)} alt={photo.filename}
                class="w-full h-full object-cover" loading="lazy"
                onerror={(e) => { (e.target as HTMLImageElement).src = getFullSrc(photo); }} />
              <div class="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent
                opacity-0 group-hover:opacity-100 transition-opacity rounded-xl">
                <div class="absolute bottom-2 left-2 right-2">
                  <p class="text-white text-[10px] truncate">{photo.filename}</p>
                </div>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

  {:else}
    <!-- Albums grid -->
    <div class="flex items-center justify-between mb-4" in:fade={{ duration: 200 }}>
      <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">Albums</h2>
      <button
        class="text-xs px-3 py-1.5 rounded-lg"
        style="background: var(--color-accent-soft); color: var(--color-accent);"
        onclick={() => showCreate = !showCreate}
      >
        + New Album
      </button>
    </div>

    {#if showCreate}
      <div class="flex gap-2 mb-4" in:fly={{ y: -10, duration: 200 }}>
        <input
          bind:value={newName}
          placeholder="Album name"
          class="flex-1 px-3 py-2 rounded-xl text-sm border-none outline-none"
          style="background: var(--color-surface); color: var(--color-text-primary);"
          onkeydown={(e) => e.key === 'Enter' && handleCreate()}
        />
        <button class="px-4 py-2 rounded-xl text-sm font-medium"
          style="background: var(--color-accent-soft); color: var(--color-accent);"
          onclick={handleCreate}>Create</button>
      </div>
    {/if}

    {#if isLoading}
      <div class="text-center py-20" style="color: var(--color-text-muted);">
        <span class="inline-block animate-spin text-xl">◌</span>
      </div>
    {:else if albums.length === 0}
      <div class="flex flex-col items-center justify-center h-[60vh] gap-4 animate-fade-in">
        <div class="w-20 h-20 rounded-2xl flex items-center justify-center text-3xl neu-raised"
          style="background: var(--color-surface);">
          ▣
        </div>
        <p class="text-base font-medium" style="color: var(--color-text-secondary);">No albums yet</p>
        <p class="text-sm" style="color: var(--color-text-muted);">Create one to organize your photos</p>
      </div>
    {:else}
      <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));">
        {#each albums as album, i (album.id)}
          <button
            class="flex flex-col rounded-2xl overflow-hidden transition-all duration-200 cursor-pointer hover:scale-[1.02]"
            style="background: var(--color-surface); box-shadow: var(--shadow-neu-raised);"
            in:fly={{ y: 20, duration: 300, delay: i * 50 }}
            onclick={() => selectAlbum(album)}
          >
            <div class="aspect-video w-full flex items-center justify-center text-4xl"
              style="background: var(--color-accent-soft); color: var(--color-accent);">
              ▣
            </div>
            <div class="p-3">
              <p class="text-sm font-medium truncate" style="color: var(--color-text-primary);">{album.name}</p>
              <p class="text-[11px]" style="color: var(--color-text-muted);">{album.photo_count} photos</p>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>
