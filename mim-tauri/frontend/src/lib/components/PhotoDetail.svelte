<script lang="ts">
  import { selectedPhotoId } from '$lib/stores/ui';
  import { photos, activeFolder } from '$lib/stores/photos';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, fly, scale } from 'svelte/transition';
  import { chatAboutPhoto } from '$lib/api/gemma';
  import { getFacesForPhoto, type Face } from '$lib/api/faces';
  import type { Photo } from '$lib/api/photos';

  let chatInput = $state('');
  let chatMessages = $state<{ role: 'user' | 'ai'; text: string }[]>([]);
  let isChatting = $state(false);
  let faces = $state<Face[]>([]);

  // Load faces when photo changes
  $effect(() => {
    const p = $photos.find(p => p.id === $selectedPhotoId);
    if (p && $activeFolder) {
      getFacesForPhoto($activeFolder.path, p.id)
        .then(f => { faces = f; })
        .catch(() => { faces = []; });
    } else {
      faces = [];
    }
    chatMessages = [];
  });

  async function sendChat() {
    if (!chatInput.trim() || !photo || !$activeFolder || isChatting) return;
    const question = chatInput.trim();
    chatInput = '';
    chatMessages = [...chatMessages, { role: 'user', text: question }];
    isChatting = true;
    try {
      const response = await chatAboutPhoto($activeFolder.path, photo.id, question);
      chatMessages = [...chatMessages, { role: 'ai', text: response }];
    } catch (e) {
      chatMessages = [...chatMessages, { role: 'ai', text: `Error: ${e}` }];
    }
    isChatting = false;
  }

  let photo = $derived($photos.find(p => p.id === $selectedPhotoId) ?? null);

  function close() {
    selectedPhotoId.set(null);
  }

  function getFullSrc(p: Photo): string {
    if (!$activeFolder) return '';
    return convertFileSrc(`${$activeFolder.path}/${p.relative_path}`);
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return 'Unknown';
    return new Date(dateStr).toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      const idx = $photos.findIndex(p => p.id === $selectedPhotoId);
      if (idx < 0) return;
      const next = e.key === 'ArrowRight'
        ? Math.min(idx + 1, $photos.length - 1)
        : Math.max(idx - 1, 0);
      selectedPhotoId.set($photos[next].id);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if photo}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-[100]"
    style="background: rgba(0,0,0,0.85); backdrop-filter: blur(30px);"
    transition:fade={{ duration: 250 }}
    role="dialog"
    aria-modal="true"
    onclick={close}
    onkeydown={(e) => e.key === 'Escape' && close()}
    tabindex="-1"
  >
    <!-- Content -->
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 flex"
      onclick={(e) => e.stopPropagation()}
      transition:scale={{ start: 0.92, duration: 350 }}
    >
      <!-- Image area -->
      <div class="flex-1 flex items-center justify-center p-8">
        <img
          src={getFullSrc(photo)}
          alt={photo.filename}
          class="max-w-full max-h-full object-contain rounded-lg"
          style="box-shadow: 0 20px 60px rgba(0,0,0,0.5);"
        />
      </div>

      <!-- Info panel -->
      <div
        class="glass-heavy w-[340px] h-full overflow-y-auto p-6 flex flex-col gap-5"
        style="border-left: 1px solid var(--color-border-glass);"
        in:fly={{ x: 340, duration: 350, delay: 100 }}
      >
        <!-- Close button -->
        <button
          class="neu-button self-end w-8 h-8 rounded-lg flex items-center justify-center text-sm"
          style="background: var(--color-surface); color: var(--color-text-secondary);"
          onclick={close}
        >
          ✕
        </button>

        <!-- Filename -->
        <h3 class="text-base font-semibold break-all" style="color: var(--color-text-primary);">
          {photo.filename}
        </h3>

        <!-- Date -->
        {#if photo.taken_at}
          <div class="glass rounded-xl p-3">
            <div class="text-[10px] uppercase tracking-wider font-semibold mb-1" style="color: var(--color-text-muted);">
              Date Taken
            </div>
            <div class="text-sm" style="color: var(--color-text-primary);">
              {formatDate(photo.taken_at)}
            </div>
          </div>
        {/if}

        <!-- Camera info -->
        {#if photo.camera_make || photo.camera_model}
          <div class="glass rounded-xl p-3">
            <div class="text-[10px] uppercase tracking-wider font-semibold mb-2" style="color: var(--color-text-muted);">
              Camera
            </div>
            <div class="text-sm" style="color: var(--color-text-primary);">
              {[photo.camera_make, photo.camera_model].filter(Boolean).join(' ')}
            </div>
            {#if photo.lens_model}
              <div class="text-xs mt-1" style="color: var(--color-text-secondary);">
                {photo.lens_model}
              </div>
            {/if}
            <div class="flex flex-wrap gap-2 mt-2">
              {#if photo.focal_length}
                <span class="glass text-[10px] px-2 py-1 rounded-full">{photo.focal_length}mm</span>
              {/if}
              {#if photo.aperture}
                <span class="glass text-[10px] px-2 py-1 rounded-full">f/{photo.aperture}</span>
              {/if}
              {#if photo.shutter_speed}
                <span class="glass text-[10px] px-2 py-1 rounded-full">{photo.shutter_speed}</span>
              {/if}
              {#if photo.iso}
                <span class="glass text-[10px] px-2 py-1 rounded-full">ISO {photo.iso}</span>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Location -->
        {#if photo.latitude && photo.longitude}
          <div class="glass rounded-xl p-3">
            <div class="text-[10px] uppercase tracking-wider font-semibold mb-1" style="color: var(--color-text-muted);">
              Location
            </div>
            <div class="text-sm" style="color: var(--color-text-primary);">
              {photo.location_name || `${photo.latitude.toFixed(4)}, ${photo.longitude.toFixed(4)}`}
            </div>
          </div>
        {/if}

        <!-- AI Description -->
        {#if photo.ai_description}
          <div class="glass rounded-xl p-3">
            <div class="text-[10px] uppercase tracking-wider font-semibold mb-1" style="color: var(--color-text-muted);">
              AI Description
            </div>
            <div class="text-sm leading-relaxed" style="color: var(--color-text-primary);">
              {photo.ai_description}
            </div>
          </div>
        {/if}

        <!-- File info -->
        <div class="glass rounded-xl p-3">
          <div class="text-[10px] uppercase tracking-wider font-semibold mb-2" style="color: var(--color-text-muted);">
            File Info
          </div>
          <div class="grid grid-cols-2 gap-y-1.5 text-xs">
            <span style="color: var(--color-text-muted);">Size</span>
            <span style="color: var(--color-text-primary);">{formatFileSize(photo.file_size)}</span>

            {#if photo.width && photo.height}
              <span style="color: var(--color-text-muted);">Dimensions</span>
              <span style="color: var(--color-text-primary);">{photo.width} × {photo.height}</span>
            {/if}

            {#if photo.format}
              <span style="color: var(--color-text-muted);">Format</span>
              <span style="color: var(--color-text-primary);">{photo.format.toUpperCase()}</span>
            {/if}

            <span style="color: var(--color-text-muted);">Type</span>
            <span style="color: var(--color-text-primary);">{photo.media_type}</span>
          </div>
        </div>

        <!-- Faces detected -->
        {#if faces.length > 0}
          <div class="glass rounded-xl p-3">
            <div class="text-[10px] uppercase tracking-wider font-semibold mb-2" style="color: var(--color-text-muted);">
              Faces ({faces.length})
            </div>
            <div class="flex flex-wrap gap-2">
              {#each faces as face}
                <div class="flex items-center gap-1.5 px-2 py-1 rounded-full text-[10px]"
                  style="background: var(--color-accent-soft); color: var(--color-accent);">
                  ◉ {face.identity_id ? 'Identified' : 'Unknown'} ({(face.detection_confidence * 100).toFixed(0)}%)
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Processing status -->
        <div class="flex flex-wrap gap-2">
          <span class="text-[10px] px-2 py-1 rounded-full" style="
            background: {photo.thumbnail_generated ? 'var(--color-accent-soft)' : 'var(--color-surface)'};
            color: {photo.thumbnail_generated ? 'var(--color-accent)' : 'var(--color-text-muted)'};
          ">
            {photo.thumbnail_generated ? '✓' : '○'} Thumbnail
          </span>
          <span class="text-[10px] px-2 py-1 rounded-full" style="
            background: {photo.faces_processed ? 'var(--color-accent-soft)' : 'var(--color-surface)'};
            color: {photo.faces_processed ? 'var(--color-accent)' : 'var(--color-text-muted)'};
          ">
            {photo.faces_processed ? '✓' : '○'} Faces
          </span>
          <span class="text-[10px] px-2 py-1 rounded-full" style="
            background: {photo.ai_processed ? 'var(--color-accent-soft)' : 'var(--color-surface)'};
            color: {photo.ai_processed ? 'var(--color-accent)' : 'var(--color-text-muted)'};
          ">
            {photo.ai_processed ? '✓' : '○'} AI Tags
          </span>
        </div>

        <!-- Chat about photo -->
        <div class="mt-auto pt-4" style="border-top: 1px solid var(--color-border-glass);">
          <div class="text-[10px] uppercase tracking-wider font-semibold mb-2" style="color: var(--color-text-muted);">
            Ask about this photo
          </div>

          {#if chatMessages.length > 0}
            <div class="flex flex-col gap-2 mb-3 max-h-48 overflow-y-auto">
              {#each chatMessages as msg}
                <div class="text-xs p-2 rounded-lg {msg.role === 'user' ? 'self-end' : ''}"
                  style="
                    background: {msg.role === 'user' ? 'var(--color-accent-soft)' : 'var(--color-surface)'};
                    color: {msg.role === 'user' ? 'var(--color-accent)' : 'var(--color-text-primary)'};
                    max-width: 90%;
                  ">
                  {msg.text}
                </div>
              {/each}
              {#if isChatting}
                <div class="text-xs p-2 rounded-lg" style="background: var(--color-surface); color: var(--color-text-muted);">
                  <span class="inline-block animate-spin">◌</span> Thinking...
                </div>
              {/if}
            </div>
          {/if}

          <div class="flex gap-2">
            <input
              type="text"
              bind:value={chatInput}
              placeholder="What's in this photo?"
              class="flex-1 px-3 py-2 rounded-xl text-xs border-none outline-none"
              style="background: var(--color-surface); color: var(--color-text-primary);"
              onkeydown={(e) => e.key === 'Enter' && sendChat()}
              disabled={isChatting}
            />
            <button
              class="px-3 py-2 rounded-xl text-xs font-medium transition-all"
              style="background: var(--color-accent-soft); color: var(--color-accent);"
              onclick={sendChat}
              disabled={isChatting || !chatInput.trim()}
            >
              Ask
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
