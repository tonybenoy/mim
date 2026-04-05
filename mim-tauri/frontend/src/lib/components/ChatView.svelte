<script lang="ts">
  import { activeFolder, photos } from '$lib/stores/photos';
  import { selectedPhotoId } from '$lib/stores/ui';
  import { chatAboutPhoto } from '$lib/api/gemma';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { fade, fly } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';

  let messages = $state<{ role: 'user' | 'ai'; text: string; photos?: Photo[] }[]>([]);
  let input = $state('');
  let isSending = $state(false);
  let chatContainer: HTMLDivElement;

  async function send() {
    if (!input.trim() || isSending || !$activeFolder) return;
    const question = input.trim();
    input = '';

    messages = [...messages, { role: 'user', text: question }];
    scrollToBottom();
    isSending = true;

    try {
      // First: search the library using the query
      const searchResults: Photo[] = await invoke('search_photos', {
        folderPath: $activeFolder.path,
        query: question,
        limit: 20,
      });

      if (searchResults.length > 0) {
        // Found matching photos — show them and describe
        messages = [...messages, {
          role: 'ai',
          text: `Found ${searchResults.length} photo${searchResults.length > 1 ? 's' : ''} matching "${question}":`,
          photos: searchResults.slice(0, 12),
        }];

        // If a specific photo matches well, ask Gemma about it
        if (searchResults.length <= 3) {
          for (const photo of searchResults.slice(0, 1)) {
            try {
              const detail = await chatAboutPhoto($activeFolder.path, photo.id, question);
              messages = [...messages, { role: 'ai', text: detail }];
            } catch {}
          }
        }
      } else {
        // No search results — try asking Gemma about a random recent photo for context
        // or just respond with a suggestion
        messages = [...messages, {
          role: 'ai',
          text: `No photos found matching "${question}". Try searching by:\n• Filename (e.g., "Screenshot")\n• AI description (run AI tagging first)\n• Tags (e.g., "sunset", "portrait")\n• Location name`,
        }];
      }
    } catch (e) {
      messages = [...messages, { role: 'ai', text: `Error: ${e}` }];
    }

    isSending = false;
    scrollToBottom();
  }

  function scrollToBottom() {
    setTimeout(() => {
      if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
    }, 50);
  }

  function getThumbnailSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    const prefix = photo.content_hash.slice(0, 2);
    return convertFileSrc(`${$activeFolder.path}/.mim/thumbnails/${prefix}/${photo.content_hash}_256.webp`);
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

  function handleImageError(e: Event, photo: Photo) {
    const img = e.target as HTMLImageElement;
    if (!img.src.includes('_64.webp')) {
      img.src = getMicroSrc(photo);
    } else {
      img.src = getFullSrc(photo);
    }
  }
</script>

<div class="flex-1 flex flex-col" style="padding-bottom: 64px;">
  <!-- Header -->
  <div class="p-4" style="border-bottom: 1px solid var(--color-border-glass);">
    <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">Chat with your photos</h2>
    <p class="text-xs" style="color: var(--color-text-muted);">
      Ask anything — "sunset photos", "screenshots from December", "photos with text"
    </p>
  </div>

  <!-- Messages -->
  <div class="flex-1 overflow-y-auto p-4 space-y-4" bind:this={chatContainer}>
    {#if messages.length === 0}
      <div class="flex flex-col items-center justify-center h-full gap-4 animate-fade-in">
        <div class="w-20 h-20 rounded-2xl flex items-center justify-center text-3xl neu-raised"
          style="background: var(--color-surface);">
          ✦
        </div>
        <p class="text-base font-medium" style="color: var(--color-text-secondary);">
          Search your photo library naturally
        </p>
        <div class="flex flex-wrap gap-2 justify-center max-w-md">
          {#each ['sunset photos', 'screenshots with code', 'photos from December', 'people outdoors'] as suggestion}
            <button
              class="px-3 py-1.5 rounded-full text-xs transition-all hover:scale-105"
              style="background: var(--color-surface); color: var(--color-text-secondary); box-shadow: var(--shadow-neu-soft);"
              onclick={() => { input = suggestion; send(); }}
            >
              {suggestion}
            </button>
          {/each}
        </div>
      </div>
    {:else}
      {#each messages as msg, i}
        <div class="flex {msg.role === 'user' ? 'justify-end' : 'justify-start'}" in:fly={{ y: 10, duration: 200 }}>
          <div class="max-w-[85%]">
            <div class="p-3 rounded-2xl text-sm leading-relaxed whitespace-pre-line"
              style="
                background: {msg.role === 'user' ? 'var(--color-accent-soft)' : 'var(--color-surface)'};
                color: {msg.role === 'user' ? 'var(--color-accent)' : 'var(--color-text-primary)'};
                {msg.role === 'user' ? 'border-bottom-right-radius: 4px;' : 'border-bottom-left-radius: 4px;'}
              ">
              {msg.text}
            </div>

            {#if msg.photos && msg.photos.length > 0}
              <div class="grid gap-1.5 mt-2"
                style="grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));">
                {#each msg.photos as photo (photo.id)}
                  <button
                    class="relative aspect-square overflow-hidden rounded-xl group cursor-pointer hover:scale-105 transition-transform"
                    style="box-shadow: var(--shadow-neu-soft);"
                    onclick={() => selectedPhotoId.set(photo.id)}
                  >
                    <img
                      src={getThumbnailSrc(photo)}
                      alt={photo.filename}
                      class="w-full h-full object-cover"
                      loading="lazy"
                      onerror={(e) => handleImageError(e, photo)}
                    />
                    <div class="absolute inset-0 bg-black/30 opacity-0 group-hover:opacity-100 transition-opacity rounded-xl flex items-end p-1.5">
                      <p class="text-white text-[9px] truncate w-full">{photo.filename}</p>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/each}
      {#if isSending}
        <div class="flex justify-start" in:fly={{ y: 10, duration: 200 }}>
          <div class="p-3 rounded-2xl text-sm" style="background: var(--color-surface); color: var(--color-text-muted);">
            <span class="inline-block animate-spin">◌</span> Searching your library...
          </div>
        </div>
      {/if}
    {/if}
  </div>

  <!-- Input bar -->
  <div class="p-3 flex gap-2 items-end" style="border-top: 1px solid var(--color-border-glass);">
    <input
      type="text"
      bind:value={input}
      placeholder="Ask about your photos..."
      class="flex-1 px-4 py-2.5 rounded-xl text-sm border-none outline-none"
      style="background: var(--color-surface); color: var(--color-text-primary);"
      onkeydown={(e) => e.key === 'Enter' && send()}
      disabled={isSending}
    />
    <button
      class="px-4 py-2.5 rounded-xl text-sm font-medium shrink-0 transition-all"
      style="background: var(--color-accent); color: white; opacity: {input.trim() && !isSending ? '1' : '0.5'};"
      onclick={send}
      disabled={isSending || !input.trim()}
    >
      Send
    </button>
  </div>
</div>
