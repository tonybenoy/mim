<script lang="ts">
  import { createAlbum, getAlbums, getAlbumPhotos, deleteAlbum, renameAlbum, type Album } from '$lib/api/albums';
  import { activeFolder, photos } from '$lib/stores/photos';
  import { selectedPhotoId } from '$lib/stores/ui';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, fly, scale } from 'svelte/transition';
  import { createSmartAlbum, getSmartAlbumPhotos, exportAlbumZip, type Photo } from '$lib/api/photos';

  let albums = $state<Album[]>([]);
  let selectedAlbum = $state<Album | null>(null);
  let albumPhotoIds = $state<string[]>([]);
  let smartAlbumPhotos = $state<Photo[]>([]);
  let isLoading = $state(true);
  let showCreate = $state(false);
  let newName = $state('');
  let editingId = $state<string | null>(null);
  let editName = $state('');

  // Smart album creation
  let createMode = $state<'choice' | 'manual' | 'smart'>('choice');
  let smartRules = $state<{ field: string; operator: string; value: string }[]>([
    { field: 'tags', operator: 'contains', value: '' }
  ]);

  // Export state
  let isExporting = $state(false);
  let exportResult = $state<string | null>(null);

  const ruleFields = [
    { value: 'tags', label: 'Tags' },
    { value: 'rating', label: 'Rating' },
    { value: 'is_favorite', label: 'Favorite' },
    { value: 'date_range', label: 'Date Range' },
    { value: 'scene_type', label: 'Scene Type' },
    { value: 'location', label: 'Location' },
  ];

  const operatorsByField: Record<string, { value: string; label: string }[]> = {
    tags: [
      { value: 'contains', label: 'Contains' },
      { value: 'equals', label: 'Equals' },
      { value: 'not_contains', label: 'Does Not Contain' },
    ],
    rating: [
      { value: 'equals', label: 'Equals' },
      { value: 'greater_than', label: 'Greater Than' },
      { value: 'less_than', label: 'Less Than' },
    ],
    is_favorite: [
      { value: 'equals', label: 'Is' },
    ],
    date_range: [
      { value: 'after', label: 'After' },
      { value: 'before', label: 'Before' },
      { value: 'between', label: 'Between' },
    ],
    scene_type: [
      { value: 'equals', label: 'Equals' },
      { value: 'contains', label: 'Contains' },
    ],
    location: [
      { value: 'contains', label: 'Contains' },
      { value: 'equals', label: 'Equals' },
    ],
  };

  function addRule() {
    smartRules = [...smartRules, { field: 'tags', operator: 'contains', value: '' }];
  }

  function removeRule(index: number) {
    smartRules = smartRules.filter((_, i) => i !== index);
  }

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
      createMode = 'choice';
    } catch (e) {
      console.error('Failed to create album:', e);
    }
  }

  async function handleCreateSmart() {
    if (!newName.trim() || !$activeFolder) return;
    const validRules = smartRules.filter(r => r.value.trim() !== '');
    if (validRules.length === 0) return;
    try {
      const rulesJson = JSON.stringify(validRules);
      const album = await createSmartAlbum($activeFolder.path, newName.trim(), rulesJson);
      albums = [album, ...albums];
      newName = '';
      smartRules = [{ field: 'tags', operator: 'contains', value: '' }];
      showCreate = false;
      createMode = 'choice';
    } catch (e) {
      console.error('Failed to create smart album:', e);
    }
  }

  async function selectAlbum(album: Album) {
    selectedAlbum = album;
    smartAlbumPhotos = [];
    if (!$activeFolder) return;
    try {
      if (album.album_type === 'smart') {
        smartAlbumPhotos = await getSmartAlbumPhotos($activeFolder.path, album.id);
        albumPhotoIds = smartAlbumPhotos.map(p => p.id);
      } else {
        albumPhotoIds = await getAlbumPhotos($activeFolder.path, album.id);
      }
    } catch (e) {
      console.error('Failed to load album photos:', e);
    }
  }

  async function handleExport() {
    if (!selectedAlbum || !$activeFolder) return;
    const dest = window.prompt('Export destination path (folder will be created):');
    if (!dest) return;
    isExporting = true;
    exportResult = null;
    try {
      const count = await exportAlbumZip($activeFolder.path, selectedAlbum.id, dest);
      exportResult = `Exported ${count} photos to ${dest}`;
    } catch (e) {
      exportResult = `Export failed: ${e}`;
    }
    isExporting = false;
    setTimeout(() => { exportResult = null; }, 5000);
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
    if (selectedAlbum?.album_type === 'smart' && smartAlbumPhotos.length > 0) {
      return smartAlbumPhotos;
    }
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
            style="background: var(--color-accent-soft); color: var(--color-accent);"
            onclick={handleExport}
            disabled={isExporting}
          >
            {isExporting ? 'Exporting...' : 'Export'}
          </button>
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

      <div class="flex items-center gap-2 mb-4">
        <p class="text-sm" style="color: var(--color-text-muted);">{albumPhotoIds.length} photos</p>
        {#if selectedAlbum.album_type === 'smart'}
          <span class="text-[10px] px-2 py-0.5 rounded-full" style="background: var(--color-accent-soft); color: var(--color-accent);">Smart</span>
        {/if}
      </div>

      {#if exportResult}
        <div class="mb-3 px-3 py-2 rounded-xl text-xs"
          style="background: {exportResult.startsWith('Export failed') ? '#fee2e2' : 'var(--color-accent-soft)'}; color: {exportResult.startsWith('Export failed') ? '#dc2626' : 'var(--color-accent)'};"
          in:fly={{ y: -10, duration: 200 }}
        >
          {exportResult}
        </div>
      {/if}

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
      <div class="mb-4 p-4 rounded-xl" style="background: var(--color-surface);" in:fly={{ y: -10, duration: 200 }}>
        {#if createMode === 'choice'}
          <p class="text-xs font-medium mb-3" style="color: var(--color-text-muted);">Choose album type:</p>
          <div class="flex gap-2">
            <button
              class="flex-1 py-3 rounded-xl text-sm font-medium transition-all hover:scale-[1.02]"
              style="background: var(--color-bg); color: var(--color-text-primary); box-shadow: var(--shadow-neu-soft);"
              onclick={() => createMode = 'manual'}
            >
              <div class="text-lg mb-1">&#x25A3;</div>
              Manual Album
            </button>
            <button
              class="flex-1 py-3 rounded-xl text-sm font-medium transition-all hover:scale-[1.02]"
              style="background: var(--color-bg); color: var(--color-text-primary); box-shadow: var(--shadow-neu-soft);"
              onclick={() => createMode = 'smart'}
            >
              <div class="text-lg mb-1">&#x2726;</div>
              Smart Album
            </button>
          </div>
          <button
            class="w-full mt-2 text-xs py-1"
            style="color: var(--color-text-muted);"
            onclick={() => { showCreate = false; createMode = 'choice'; }}
          >Cancel</button>

        {:else if createMode === 'manual'}
          <div class="flex gap-2">
            <input
              bind:value={newName}
              placeholder="Album name"
              class="flex-1 px-3 py-2 rounded-xl text-sm border-none outline-none"
              style="background: var(--color-bg); color: var(--color-text-primary);"
              onkeydown={(e) => e.key === 'Enter' && handleCreate()}
            />
            <button class="px-4 py-2 rounded-xl text-sm font-medium"
              style="background: var(--color-accent-soft); color: var(--color-accent);"
              onclick={handleCreate}>Create</button>
            <button class="px-3 py-2 rounded-xl text-sm"
              style="color: var(--color-text-muted);"
              onclick={() => { showCreate = false; createMode = 'choice'; }}>Cancel</button>
          </div>

        {:else if createMode === 'smart'}
          <div class="space-y-3">
            <input
              bind:value={newName}
              placeholder="Smart album name"
              class="w-full px-3 py-2 rounded-xl text-sm border-none outline-none"
              style="background: var(--color-bg); color: var(--color-text-primary);"
            />

            <div class="text-[10px] uppercase tracking-wider font-semibold" style="color: var(--color-text-muted);">
              Rules
            </div>

            {#each smartRules as rule, i}
              <div class="flex gap-2 items-center" in:fly={{ y: -10, duration: 200 }}>
                <select
                  bind:value={rule.field}
                  class="px-2 py-1.5 rounded-lg text-xs border-none outline-none"
                  style="background: var(--color-bg); color: var(--color-text-primary);"
                  onchange={() => { rule.operator = (operatorsByField[rule.field]?.[0]?.value || 'contains'); }}
                >
                  {#each ruleFields as f}
                    <option value={f.value}>{f.label}</option>
                  {/each}
                </select>

                <select
                  bind:value={rule.operator}
                  class="px-2 py-1.5 rounded-lg text-xs border-none outline-none"
                  style="background: var(--color-bg); color: var(--color-text-primary);"
                >
                  {#each (operatorsByField[rule.field] || []) as op}
                    <option value={op.value}>{op.label}</option>
                  {/each}
                </select>

                {#if rule.field === 'is_favorite'}
                  <select
                    bind:value={rule.value}
                    class="flex-1 px-2 py-1.5 rounded-lg text-xs border-none outline-none"
                    style="background: var(--color-bg); color: var(--color-text-primary);"
                  >
                    <option value="true">Yes</option>
                    <option value="false">No</option>
                  </select>
                {:else if rule.field === 'rating'}
                  <select
                    bind:value={rule.value}
                    class="flex-1 px-2 py-1.5 rounded-lg text-xs border-none outline-none"
                    style="background: var(--color-bg); color: var(--color-text-primary);"
                  >
                    {#each [1, 2, 3, 4, 5] as r}
                      <option value={String(r)}>{'&#9733;'.repeat(r)}</option>
                    {/each}
                  </select>
                {:else}
                  <input
                    bind:value={rule.value}
                    placeholder="Value"
                    class="flex-1 px-2 py-1.5 rounded-lg text-xs border-none outline-none"
                    style="background: var(--color-bg); color: var(--color-text-primary);"
                  />
                {/if}

                {#if smartRules.length > 1}
                  <button
                    class="text-xs px-2 py-1 rounded-lg"
                    style="color: #dc2626;"
                    onclick={() => removeRule(i)}
                  >&#x2715;</button>
                {/if}
              </div>
            {/each}

            <button
              class="text-xs px-3 py-1.5 rounded-lg"
              style="background: var(--color-bg); color: var(--color-text-secondary);"
              onclick={addRule}
            >
              + Add Rule
            </button>

            <div class="flex gap-2 pt-2" style="border-top: 1px solid var(--color-border-glass);">
              <button class="flex-1 px-4 py-2 rounded-xl text-sm font-medium"
                style="background: var(--color-accent-soft); color: var(--color-accent);"
                onclick={handleCreateSmart}>Create Smart Album</button>
              <button class="px-3 py-2 rounded-xl text-sm"
                style="color: var(--color-text-muted);"
                onclick={() => { showCreate = false; createMode = 'choice'; smartRules = [{ field: 'tags', operator: 'contains', value: '' }]; }}>Cancel</button>
            </div>
          </div>
        {/if}
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
            <div class="aspect-video w-full flex items-center justify-center text-4xl relative"
              style="background: var(--color-accent-soft); color: var(--color-accent);">
              {album.album_type === 'smart' ? '&#x2726;' : '&#x25A3;'}
              {#if album.album_type === 'smart'}
                <span class="absolute top-2 right-2 text-[9px] px-1.5 py-0.5 rounded-full"
                  style="background: var(--color-accent); color: #fff;">Smart</span>
              {/if}
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
