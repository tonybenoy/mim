<script lang="ts">
  import { getIdentities, getFacesForPhoto, renameIdentity, mergeIdentities, getIdentitiesWithAvatars, type FaceIdentity, type Face, type IdentityWithAvatar } from '$lib/api/faces';
  import { activeFolder, photos } from '$lib/stores/photos';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, fly, scale } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';

  let identitiesWithAvatars = $state<IdentityWithAvatar[]>([]);
  let identities = $derived(identitiesWithAvatars.map(i => i.identity));
  let selectedIdentity = $state<FaceIdentity | null>(null);
  let identityPhotos = $state<{ photo: Photo; face: Face }[]>([]);
  let isLoading = $state(true);
  let editingId = $state<string | null>(null);
  let editName = $state('');
  let showMerge = $state(false);

  $effect(() => {
    loadIdentities();
  });

  async function loadIdentities() {
    isLoading = true;
    try {
      if ($activeFolder) {
        identitiesWithAvatars = await getIdentitiesWithAvatars($activeFolder.path);
      }
    } catch (e) {
      console.error('Failed to load identities:', e);
    }
    isLoading = false;
  }

  function getAvatarSrc(identity: FaceIdentity): string | null {
    const data = identitiesWithAvatars.find(i => i.identity.id === identity.id);
    if (!data?.face_crop_path) return null;
    return convertFileSrc(data.face_crop_path);
  }

  async function selectIdentity(identity: FaceIdentity) {
    selectedIdentity = identity;
    if (!$activeFolder) return;

    // Find photos containing faces of this identity
    identityPhotos = [];
    for (const photo of $photos) {
      try {
        const faces = await getFacesForPhoto($activeFolder.path, photo.id);
        for (const face of faces) {
          if (face.identity_id === identity.id) {
            identityPhotos = [...identityPhotos, { photo, face }];
            break;
          }
        }
      } catch {}
    }
  }

  function getFaceCropSrc(photo: Photo, face: Face): string {
    if (!$activeFolder) return '';
    return convertFileSrc(`${$activeFolder.path}/${photo.relative_path}`);
  }

  function getThumbnailSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    const prefix = photo.content_hash.slice(0, 2);
    return convertFileSrc(`${$activeFolder.path}/.mim/thumbnails/${prefix}/${photo.content_hash}_256.webp`);
  }

  function startRename(identity: FaceIdentity) {
    editingId = identity.id;
    editName = identity.name;
  }

  async function saveRename() {
    if (!editingId || !editName.trim()) return;
    try {
      await renameIdentity(editingId, editName.trim());
      identities = identities.map(i =>
        i.id === editingId ? { ...i, name: editName.trim() } : i
      );
      if (selectedIdentity?.id === editingId) {
        selectedIdentity = { ...selectedIdentity, name: editName.trim() };
      }
    } catch (e) {
      console.error('Failed to rename:', e);
    }
    editingId = null;
  }

  function cancelRename() {
    editingId = null;
  }

  async function handleMerge(targetIdentity: FaceIdentity) {
    if (!selectedIdentity || !$activeFolder) return;
    if (targetIdentity.id === selectedIdentity.id) return;

    const sourceName = selectedIdentity.name;
    const confirmed = window.confirm(
      `Merge "${sourceName}" into "${targetIdentity.name}"? This will combine all their photos.`
    );
    if (!confirmed) return;

    try {
      await mergeIdentities(targetIdentity.id, selectedIdentity.id, $activeFolder.path);
      selectedIdentity = null;
      identityPhotos = [];
      showMerge = false;
      await loadIdentities();
    } catch (e) {
      console.error('Merge failed:', e);
    }
  }
</script>

<div class="flex-1 overflow-y-auto p-4" style="padding-bottom: 80px;">
  {#if selectedIdentity}
    <!-- Identity detail view -->
    <div in:fade={{ duration: 200 }}>
      <button
        class="flex items-center gap-2 mb-4 px-3 py-2 rounded-xl text-sm transition-all hover:opacity-80"
        style="color: var(--color-accent);"
        onclick={() => { selectedIdentity = null; identityPhotos = []; }}
      >
        ← Back to People
      </button>

      <div class="flex items-center gap-4 mb-6">
        {#if getAvatarSrc(selectedIdentity)}
          <div class="w-16 h-16 rounded-full overflow-hidden neu-raised ring-2 ring-white/20">
            <img
              src={getAvatarSrc(selectedIdentity)}
              alt={selectedIdentity.name}
              class="w-full h-full object-cover"
            />
          </div>
        {:else}
          <div class="w-16 h-16 rounded-full flex items-center justify-center text-2xl neu-raised"
            style="background: var(--color-accent-soft); color: var(--color-accent);">
            {selectedIdentity.name.charAt(0).toUpperCase()}
          </div>
        {/if}
        <div>
          {#if editingId === selectedIdentity.id}
            <div class="flex items-center gap-2">
              <input
                type="text"
                bind:value={editName}
                class="px-2 py-1 rounded-lg text-lg font-semibold border-none outline-none"
                style="background: var(--color-surface); color: var(--color-text-primary);"
                onkeydown={(e) => e.key === 'Enter' && saveRename()}
              />
              <button class="text-xs px-2 py-1 rounded-lg" style="color: var(--color-accent);" onclick={saveRename}>Save</button>
              <button class="text-xs px-2 py-1 rounded-lg" style="color: var(--color-text-muted);" onclick={cancelRename}>Cancel</button>
            </div>
          {:else}
            <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">
              {selectedIdentity.name}
            </h2>
          {/if}
          <p class="text-sm" style="color: var(--color-text-muted);">
            {selectedIdentity.photo_count} photos
          </p>
        </div>
        {#if editingId !== selectedIdentity.id}
          <div class="flex gap-2">
            <button
              class="text-xs px-3 py-1.5 rounded-lg"
              style="background: var(--color-surface); color: var(--color-text-secondary);"
              onclick={() => startRename(selectedIdentity!)}
            >
              Rename
            </button>
            <button
              class="text-xs px-3 py-1.5 rounded-lg"
              style="background: {showMerge ? 'var(--color-accent-soft)' : 'var(--color-surface)'}; color: {showMerge ? 'var(--color-accent)' : 'var(--color-text-secondary)'};"
              onclick={() => showMerge = !showMerge}
            >
              {showMerge ? 'Cancel Merge' : 'Merge Into...'}
            </button>
          </div>
        {/if}
      </div>

      {#if showMerge}
        <div class="mb-4 p-3 rounded-xl" style="background: var(--color-surface);" in:fly={{ y: -10, duration: 200 }}>
          <p class="text-xs font-medium mb-2" style="color: var(--color-text-muted);">
            Select a person to merge "{selectedIdentity.name}" into:
          </p>
          <div class="flex flex-wrap gap-2">
            {#each identities.filter(i => i.id !== selectedIdentity?.id) as target (target.id)}
              <button
                class="flex items-center gap-2 px-3 py-2 rounded-xl text-sm transition-all hover:scale-105"
                style="background: var(--color-bg); box-shadow: var(--shadow-neu-soft);"
                onclick={() => handleMerge(target)}
              >
                <span class="w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-semibold"
                  style="background: var(--color-accent-soft); color: var(--color-accent);">
                  {target.name.charAt(0).toUpperCase()}
                </span>
                <span style="color: var(--color-text-primary);">{target.name}</span>
                <span class="text-[10px]" style="color: var(--color-text-muted);">{target.photo_count}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      {#if identityPhotos.length === 0}
        <div class="text-center py-12" style="color: var(--color-text-muted);">
          <span class="inline-block animate-spin text-lg">◌</span>
          <p class="mt-2 text-sm">Loading photos...</p>
        </div>
      {:else}
        <div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
          {#each identityPhotos as { photo, face }, i (photo.id)}
            <div
              class="relative aspect-square overflow-hidden rounded-xl group"
              style="box-shadow: var(--shadow-neu-soft);"
              in:scale={{ start: 0.9, duration: 300, delay: Math.min(i * 30, 500) }}
            >
              <img
                src={getThumbnailSrc(photo)}
                alt={photo.filename}
                class="w-full h-full object-cover"
                loading="lazy"
                onerror={(e) => { (e.target as HTMLImageElement).src = getFaceCropSrc(photo, face); }}
              />
              <!-- Face bounding box overlay -->
              <div
                class="absolute border-2 rounded-sm pointer-events-none opacity-0 group-hover:opacity-100 transition-opacity"
                style="
                  border-color: var(--color-accent);
                  left: {(face.bbox_x / (photo.width || 1)) * 100}%;
                  top: {(face.bbox_y / (photo.height || 1)) * 100}%;
                  width: {(face.bbox_width / (photo.width || 1)) * 100}%;
                  height: {(face.bbox_height / (photo.height || 1)) * 100}%;
                "
              ></div>
              <div class="absolute bottom-0 left-0 right-0 p-2 bg-gradient-to-t from-black/50 to-transparent
                opacity-0 group-hover:opacity-100 transition-opacity">
                <p class="text-white text-[10px] truncate">{photo.filename}</p>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

  {:else}
    <!-- People grid -->
    <div class="flex items-center justify-between mb-4" in:fade={{ duration: 200 }}>
      <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">
        People
      </h2>
      {#if identities.length > 0}
        <span class="text-xs px-2 py-0.5 rounded-full"
          style="background: var(--color-accent-soft); color: var(--color-accent);">
          {identities.length} people
        </span>
      {/if}
    </div>

    {#if isLoading}
      <div class="text-center py-20" style="color: var(--color-text-muted);">
        <span class="inline-block animate-spin text-2xl">◌</span>
        <p class="mt-3 text-sm">Loading people...</p>
      </div>
    {:else if identities.length === 0}
      <div class="flex flex-col items-center justify-center h-[60vh] gap-4 animate-fade-in">
        <div class="w-20 h-20 rounded-2xl flex items-center justify-center text-3xl neu-raised"
          style="background: var(--color-surface);">
          ◉
        </div>
        <p class="text-base font-medium" style="color: var(--color-text-secondary);">
          No people found yet
        </p>
        <p class="text-sm" style="color: var(--color-text-muted);">
          Run face detection on a folder first
        </p>
      </div>
    {:else}
      <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));">
        {#each identities as identity, i (identity.id)}
          <button
            class="flex flex-col items-center gap-3 p-4 rounded-2xl transition-all duration-200 cursor-pointer hover:scale-105"
            style="background: var(--color-surface); box-shadow: var(--shadow-neu-raised);"
            in:fly={{ y: 20, duration: 300, delay: i * 50 }}
            onclick={() => selectIdentity(identity)}
          >
            {#if getAvatarSrc(identity)}
              <div class="w-20 h-20 rounded-full overflow-hidden ring-2 ring-white/20" style="box-shadow: 0 4px 16px rgba(0,0,0,0.2);">
                <img
                  src={getAvatarSrc(identity)}
                  alt={identity.name}
                  class="w-full h-full object-cover"
                />
              </div>
            {:else}
              <div class="w-20 h-20 rounded-full flex items-center justify-center text-2xl font-semibold"
                style="background: var(--color-accent-soft); color: var(--color-accent);">
                {identity.name.charAt(0).toUpperCase()}
              </div>
            {/if}
            <div class="text-center">
              <p class="text-sm font-medium truncate w-full" style="color: var(--color-text-primary);">
                {identity.name}
              </p>
              <p class="text-[11px]" style="color: var(--color-text-muted);">
                {identity.photo_count} photos
              </p>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>
