<script lang="ts">
  import { photos, activeFolder } from '$lib/stores/photos';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';

  let { photoA = $bindable(), onclose = () => {} }: { photoA: Photo | null; onclose?: () => void } = $props();
  let photoB = $state<Photo | null>(null);
  let picking = $state(true);

  function getFullSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    return convertFileSrc(`${$activeFolder.path}/${photo.relative_path}`);
  }

  function getThumbnailSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    const prefix = photo.content_hash.slice(0, 2);
    return convertFileSrc(`${$activeFolder.path}/.mim/thumbnails/${prefix}/${photo.content_hash}_256.webp`);
  }

  function pickPhoto(photo: Photo) {
    photoB = photo;
    picking = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if photoA}
  <div
    class="fixed inset-0 z-[110]"
    style="background: rgba(0,0,0,0.9); backdrop-filter: blur(30px);"
    transition:fade={{ duration: 250 }}
  >
    <!-- Header -->
    <div class="absolute top-4 left-4 right-4 flex items-center justify-between z-10">
      <h3 class="text-sm font-semibold" style="color: var(--color-text-primary);">
        Compare Photos
      </h3>
      <button
        class="neu-button w-8 h-8 rounded-lg flex items-center justify-center text-sm"
        style="background: var(--color-surface); color: var(--color-text-secondary);"
        onclick={onclose}
      >
        &#10005;
      </button>
    </div>

    {#if picking}
      <!-- Photo picker -->
      <div class="absolute inset-0 pt-14 px-4 pb-4 overflow-y-auto">
        <p class="text-xs mb-3" style="color: var(--color-text-muted);">
          Select a photo to compare with "{photoA.filename}"
        </p>
        <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));">
          {#each $photos.filter(p => p.id !== photoA?.id) as photo (photo.id)}
            <button
              class="relative aspect-square overflow-hidden rounded-xl group cursor-pointer transition-all hover:ring-2"
              style="--tw-ring-color: var(--color-accent);"
              onclick={() => pickPhoto(photo)}
            >
              <img
                src={getThumbnailSrc(photo)}
                alt={photo.filename}
                class="w-full h-full object-cover"
                loading="lazy"
                onerror={(e) => {
                  const img = e.target as HTMLImageElement;
                  img.src = getFullSrc(photo);
                }}
              />
            </button>
          {/each}
        </div>
      </div>
    {:else if photoB}
      <!-- Side by side comparison -->
      <div class="absolute inset-0 pt-14 px-4 pb-4 flex gap-4">
        <div class="flex-1 flex flex-col items-center justify-center gap-2">
          <img
            src={getFullSrc(photoA)}
            alt={photoA.filename}
            class="max-w-full max-h-[calc(100vh-140px)] object-contain rounded-lg"
          />
          <span class="text-xs" style="color: var(--color-text-muted);">{photoA.filename}</span>
        </div>
        <div class="w-px" style="background: var(--color-border-glass);"></div>
        <div class="flex-1 flex flex-col items-center justify-center gap-2">
          <img
            src={getFullSrc(photoB)}
            alt={photoB.filename}
            class="max-w-full max-h-[calc(100vh-140px)] object-contain rounded-lg"
          />
          <span class="text-xs" style="color: var(--color-text-muted);">{photoB.filename}</span>
        </div>
      </div>
      <div class="absolute bottom-4 left-1/2 -translate-x-1/2 flex gap-3">
        <button
          class="neu-button px-4 py-2 rounded-xl text-xs font-medium"
          style="background: var(--color-surface); color: var(--color-text-secondary);"
          onclick={() => { picking = true; photoB = null; }}
        >
          Pick Different
        </button>
      </div>
    {/if}
  </div>
{/if}
