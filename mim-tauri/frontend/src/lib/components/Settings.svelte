<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';
  import { getStorageStats, type StorageStats } from '$lib/api/photos';
  import { setupSync, getSyncStatus, stopSync, addSyncFolder, type SyncStatus } from '$lib/api/sync';
  import { activeFolder } from '$lib/stores/photos';
  import { open } from '@tauri-apps/plugin-dialog';

  let { show = $bindable(false) } = $props();

  let gemmaModel = $state('gemma-4-E4B');
  let scrfdModel = $state('scrfd-10g');
  let useGpu = $state(false);
  let thumbnailSize = $state(256);

  // Storage stats
  let storageStats = $state<StorageStats | null>(null);
  let storageLoading = $state(false);

  // Sync state
  let syncStatus = $state<SyncStatus | null>(null);
  let syncLoading = $state(false);
  let syncStep = $state(0); // 0=not started, 1=setting up, 2=ready, 3=add folder
  let syncFolderPath = $state('');
  let syncFolderLabel = $state('');

  $effect(() => {
    if (show && $activeFolder) {
      loadStorageStats();
      loadSyncStatus();
    }
  });

  async function loadStorageStats() {
    if (!$activeFolder) return;
    storageLoading = true;
    try {
      storageStats = await getStorageStats($activeFolder.path);
    } catch (e) {
      console.error('Failed to load storage stats:', e);
    }
    storageLoading = false;
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  async function loadSyncStatus() {
    try {
      syncStatus = await getSyncStatus();
      if (syncStatus.device_id) {
        syncStep = 2;
      }
    } catch {
      syncStatus = null;
    }
  }

  async function handleSetupSync() {
    syncLoading = true;
    syncStep = 1;
    try {
      syncStatus = await setupSync();
      syncStep = 2;
    } catch (e) {
      console.error('Failed to setup sync:', e);
      syncStep = 0;
    }
    syncLoading = false;
  }

  async function handleStopSync() {
    try {
      await stopSync();
      syncStatus = { ...syncStatus!, running: false };
    } catch (e) {
      console.error('Failed to stop sync:', e);
    }
  }

  async function handleStartSync() {
    try {
      syncStatus = await setupSync();
    } catch (e) {
      console.error('Failed to start sync:', e);
    }
  }

  async function handleAddSyncFolder() {
    let selected: string | null = null;
    try {
      const result = await open({ directory: true, multiple: false });
      if (result && typeof result === 'string') selected = result;
    } catch {}
    if (!selected) return;
    const label = window.prompt('Label for this sync folder:', selected.split('/').pop() || 'Phone') || 'Phone';
    try {
      await addSyncFolder(selected, label);
      await loadSyncStatus();
    } catch (e) {
      console.error('Failed to add sync folder:', e);
    }
  }

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

      <!-- Storage Dashboard -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          Storage
        </div>
        {#if storageLoading}
          <div class="flex items-center justify-center py-4">
            <span class="inline-block animate-spin text-sm" style="color: var(--color-text-muted);">&#x25CC;</span>
          </div>
        {:else if storageStats}
          <div class="rounded-xl p-4 space-y-3" style="background: var(--color-surface);">
            <div class="grid grid-cols-2 gap-3">
              <div class="rounded-lg p-3" style="background: var(--color-bg);">
                <div class="text-[10px] uppercase tracking-wider" style="color: var(--color-text-muted);">Total Photos</div>
                <div class="text-lg font-semibold" style="color: var(--color-text-primary);">{storageStats.total_photos.toLocaleString()}</div>
              </div>
              <div class="rounded-lg p-3" style="background: var(--color-bg);">
                <div class="text-[10px] uppercase tracking-wider" style="color: var(--color-text-muted);">Total Size</div>
                <div class="text-lg font-semibold" style="color: var(--color-text-primary);">{formatSize(storageStats.total_size)}</div>
              </div>
            </div>
            <div class="space-y-2">
              <div class="flex items-center justify-between text-xs">
                <span style="color: var(--color-text-secondary);">Thumbnail Cache</span>
                <span class="font-medium" style="color: var(--color-text-primary);">{formatSize(storageStats.thumbnail_size)}</span>
              </div>
              <div class="flex items-center justify-between text-xs">
                <span style="color: var(--color-text-secondary);">Face Crops</span>
                <span class="font-medium" style="color: var(--color-text-primary);">{formatSize(storageStats.face_crops_size)}</span>
              </div>
              <div class="flex items-center justify-between text-xs">
                <span style="color: var(--color-text-secondary);">Database</span>
                <span class="font-medium" style="color: var(--color-text-primary);">{formatSize(storageStats.db_size)}</span>
              </div>
            </div>
            <!-- Visual bar -->
            <div class="w-full h-2 rounded-full overflow-hidden flex" style="background: var(--color-bg);">
              {#if storageStats.total_size > 0}
                {@const total = storageStats.total_size + storageStats.thumbnail_size + storageStats.face_crops_size + storageStats.db_size}
                <div class="h-full" style="width: {(storageStats.total_size / total) * 100}%; background: var(--color-accent);"></div>
                <div class="h-full" style="width: {(storageStats.thumbnail_size / total) * 100}%; background: #fbbf24;"></div>
                <div class="h-full" style="width: {(storageStats.face_crops_size / total) * 100}%; background: #34d399;"></div>
                <div class="h-full" style="width: {(storageStats.db_size / total) * 100}%; background: #f472b6;"></div>
              {/if}
            </div>
            <div class="flex flex-wrap gap-3 text-[9px]">
              <div class="flex items-center gap-1"><div class="w-2 h-2 rounded-full" style="background: var(--color-accent);"></div><span style="color: var(--color-text-muted);">Photos</span></div>
              <div class="flex items-center gap-1"><div class="w-2 h-2 rounded-full" style="background: #fbbf24;"></div><span style="color: var(--color-text-muted);">Thumbnails</span></div>
              <div class="flex items-center gap-1"><div class="w-2 h-2 rounded-full" style="background: #34d399;"></div><span style="color: var(--color-text-muted);">Faces</span></div>
              <div class="flex items-center gap-1"><div class="w-2 h-2 rounded-full" style="background: #f472b6;"></div><span style="color: var(--color-text-muted);">Database</span></div>
            </div>
          </div>
        {:else}
          <div class="text-xs p-3 rounded-xl" style="background: var(--color-surface); color: var(--color-text-muted);">
            Select a folder to see storage stats
          </div>
        {/if}
      </div>

      <!-- Phone Sync -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          Phone Sync
        </div>
        <div class="rounded-xl p-4 space-y-3" style="background: var(--color-surface);">
          {#if syncStep === 0}
            <!-- Step 1: Enable -->
            <p class="text-xs" style="color: var(--color-text-secondary);">
              Sync photos from your phone using Syncthing. Runs locally, no cloud needed.
            </p>
            <button
              class="w-full py-2.5 rounded-xl text-sm font-medium transition-all hover:scale-[1.02]"
              style="background: var(--color-accent-soft); color: var(--color-accent);"
              onclick={handleSetupSync}
              disabled={syncLoading}
            >
              {syncLoading ? 'Setting up...' : 'Enable Phone Sync'}
            </button>
          {:else if syncStep === 1}
            <!-- Setting up -->
            <div class="flex items-center gap-2 py-4 justify-center">
              <span class="inline-block animate-spin" style="color: var(--color-accent);">&#x25CC;</span>
              <span class="text-sm" style="color: var(--color-text-secondary);">Setting up Syncthing...</span>
            </div>
          {:else if syncStep >= 2}
            <!-- Step 2: Show device ID -->
            {#if syncStatus?.device_id}
              <div>
                <div class="text-[10px] uppercase tracking-wider mb-1" style="color: var(--color-text-muted);">
                  Your Device ID
                </div>
                <div class="text-[10px] font-mono p-2 rounded-lg break-all select-all"
                  style="background: var(--color-bg); color: var(--color-text-primary);">
                  {syncStatus.device_id}
                </div>
                <p class="text-[10px] mt-1" style="color: var(--color-text-muted);">
                  Install Syncthing on your phone, then add this device ID to sync photos.
                </p>
              </div>
            {/if}

            <!-- Sync status -->
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full" style="background: {syncStatus?.running ? '#34d399' : '#94a3b8'};"></div>
                <span class="text-xs" style="color: var(--color-text-secondary);">
                  {syncStatus?.running ? 'Running' : 'Stopped'}
                </span>
              </div>
              <button
                class="text-xs px-3 py-1.5 rounded-lg"
                style="background: var(--color-bg); color: var(--color-text-secondary);"
                onclick={syncStatus?.running ? handleStopSync : handleStartSync}
              >
                {syncStatus?.running ? 'Stop' : 'Start'}
              </button>
            </div>

            <!-- Synced folders -->
            {#if syncStatus?.synced_folders && syncStatus.synced_folders.length > 0}
              <div>
                <div class="text-[10px] uppercase tracking-wider mb-1" style="color: var(--color-text-muted);">
                  Synced Folders
                </div>
                {#each syncStatus.synced_folders as folder}
                  <div class="flex items-center gap-2 text-xs py-1" style="color: var(--color-text-secondary);">
                    <span>&#x25C7;</span> {folder}
                  </div>
                {/each}
              </div>
            {/if}

            <!-- Step 3: Add folder to sync -->
            <button
              class="w-full py-2 rounded-xl text-xs font-medium transition-all hover:scale-[1.02]"
              style="background: var(--color-bg); color: var(--color-text-secondary);"
              onclick={handleAddSyncFolder}
            >
              + Add Folder to Sync
            </button>
          {/if}
        </div>
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
