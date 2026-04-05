<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';

  let { show = $bindable(false) } = $props();

  let gemmaModel = $state('gemma-4-E4B');
  let scrfdModel = $state('scrfd-10g');
  let useGpu = $state(false);
  let thumbnailSize = $state(256);

  const gemmaOptions = [
    { value: 'gemma-4-E4B', label: 'Gemma 4 E4B (4.98 GB) — Best vision + OCR', desc: 'Recommended for image understanding' },
    { value: 'gemma-4-E2B', label: 'Gemma 4 E2B (1.5 GB) — Lightweight', desc: 'Faster, less accurate' },
    { value: 'gemma-3-4b', label: 'Gemma 3 4B (2.49 GB) — Legacy', desc: 'Older model, stable' },
  ];

  const scrfdOptions = [
    { value: 'scrfd-10g', label: 'SCRFD-10G (17 MB) — Most accurate' },
    { value: 'scrfd-2.5g', label: 'SCRFD-2.5G (3 MB) — Balanced' },
    { value: 'scrfd-500m', label: 'SCRFD-500M (1 MB) — Fastest' },
  ];

  function close() {
    show = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  async function handleExit() {
    if (window.confirm('Exit Mim?')) {
      try { await invoke('exit_app'); } catch {}
      window.close();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if show}
  <div
    class="fixed inset-0 z-[200] flex items-center justify-center"
    style="background: rgba(0,0,0,0.6); backdrop-filter: blur(10px);"
    transition:fade={{ duration: 200 }}
    onclick={close}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="glass-heavy w-[500px] max-h-[80vh] overflow-y-auto rounded-2xl p-6"
      style="box-shadow: 0 20px 60px rgba(0,0,0,0.3);"
      onclick={(e) => e.stopPropagation()}
      transition:fly={{ y: 20, duration: 300 }}
    >
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">Settings</h2>
        <button
          class="w-8 h-8 rounded-lg flex items-center justify-center"
          style="background: var(--color-surface); color: var(--color-text-secondary);"
          onclick={close}
        >✕</button>
      </div>

      <!-- AI Model Selection -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          Vision Model (Gemma)
        </div>
        <div class="space-y-2">
          {#each gemmaOptions as opt}
            <label
              class="flex items-start gap-3 p-3 rounded-xl cursor-pointer transition-all"
              style="background: {gemmaModel === opt.value ? 'var(--color-accent-soft)' : 'var(--color-surface)'};"
            >
              <input type="radio" bind:group={gemmaModel} value={opt.value} class="mt-1 accent-[var(--color-accent)]" />
              <div>
                <div class="text-sm font-medium" style="color: var(--color-text-primary);">{opt.label}</div>
                <div class="text-xs" style="color: var(--color-text-muted);">{opt.desc}</div>
              </div>
            </label>
          {/each}
        </div>
      </div>

      <!-- Face Detection Model -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          Face Detection Model
        </div>
        <div class="space-y-2">
          {#each scrfdOptions as opt}
            <label
              class="flex items-center gap-3 p-3 rounded-xl cursor-pointer transition-all"
              style="background: {scrfdModel === opt.value ? 'var(--color-accent-soft)' : 'var(--color-surface)'};"
            >
              <input type="radio" bind:group={scrfdModel} value={opt.value} class="accent-[var(--color-accent)]" />
              <span class="text-sm" style="color: var(--color-text-primary);">{opt.label}</span>
            </label>
          {/each}
        </div>
      </div>

      <!-- GPU -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          Performance
        </div>
        <label class="flex items-center justify-between p-3 rounded-xl cursor-pointer"
          style="background: var(--color-surface);">
          <span class="text-sm" style="color: var(--color-text-primary);">Use GPU (CUDA) when available</span>
          <input type="checkbox" bind:checked={useGpu} class="accent-[var(--color-accent)]" />
        </label>
      </div>

      <!-- About -->
      <div class="mb-6 p-4 rounded-xl" style="background: var(--color-surface);">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-2" style="color: var(--color-text-muted);">
          About
        </div>
        <div class="text-sm font-semibold" style="color: var(--color-text-primary);">Mim</div>
        <div class="text-xs" style="color: var(--color-text-muted);">
          Photo Library Manager v0.1.0
        </div>
        <div class="text-xs mt-2" style="color: var(--color-text-muted);">
          Named after Mimir, the Norse god of wisdom and memory.
          Face detection, AI tagging, and photo chat — all running locally.
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-2">
        <button
          class="flex-1 px-4 py-2.5 rounded-xl text-sm font-medium"
          style="background: var(--color-accent-soft); color: var(--color-accent);"
          onclick={close}
        >
          Done
        </button>
        <button
          class="px-4 py-2.5 rounded-xl text-sm"
          style="background: #fee2e2; color: #dc2626;"
          onclick={handleExit}
        >
          Exit Mim
        </button>
      </div>
    </div>
  </div>
{/if}
