<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';
  import { getStorageStats, type StorageStats } from '$lib/api/photos';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import { locale, localeNames, tStore, type LocaleCode } from '$lib/i18n';

  interface ModelInfo {
    name: string;
    filename: string;
    size: number;
    exists: boolean;
    purpose: string;
    is_custom: boolean;
  }

  let modelsList = $state<ModelInfo[]>([]);
  let modelsDir = $state('');
  let modelsLoading = $state(false);
  import { setupSync, getSyncStatus, stopSync, addSyncFolder, type SyncStatus } from '$lib/api/sync';
  import { activeFolder } from '$lib/stores/photos';
  import { open } from '@tauri-apps/plugin-dialog';

  let { show = $bindable(false) } = $props();

  let gemmaModel = $state('gemma-3-4b');
  let scrfdModel = $state('scrfd-10g');
  let gpuInfo = $state<{ cuda_available: boolean; label: string } | null>(null);
  let currentLocale = $state<LocaleCode>('en');
  let importLoading = $state(false);
  let importMessage = $state('');

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
    if (show) {
      loadModels();
      loadSettings();
      loadGpuInfo();
      if ($activeFolder) {
        loadStorageStats();
      }
      loadSyncStatus();
    }
  });

  async function loadGpuInfo() {
    try {
      gpuInfo = await invoke('get_gpu_info');
    } catch (e) {
      console.error('Failed to load GPU info:', e);
    }
  }

  async function loadSettings() {
    try {
      const saved = await invoke('get_setting', { key: 'gemma_model' }) as string | null;
      if (saved) gemmaModel = saved;
      const savedScrfd = await invoke('get_setting', { key: 'scrfd_model' }) as string | null;
      if (savedScrfd) scrfdModel = savedScrfd;
      const savedLocale = await invoke('get_setting', { key: 'locale' }) as string | null;
      if (savedLocale && savedLocale in localeNames) {
        currentLocale = savedLocale as LocaleCode;
        locale.set(currentLocale);
      }
    } catch {}
  }

  async function saveSetting(key: string, value: string) {
    try {
      await invoke('set_setting', { key, value });
    } catch (e) {
      console.error('Failed to save setting:', e);
    }
  }

  // Auto-save when settings change
  $effect(() => { saveSetting('gemma_model', gemmaModel); });
  $effect(() => { saveSetting('scrfd_model', scrfdModel); });
  $effect(() => { locale.set(currentLocale); saveSetting('locale', currentLocale); });

  async function loadModels() {
    modelsLoading = true;
    try {
      modelsList = await invoke('list_models');
      modelsDir = await invoke('get_models_dir');
    } catch (e) {
      console.error('Failed to load models:', e);
    }
    modelsLoading = false;
  }

  async function handleDeleteModel(filename: string, name: string) {
    if (!window.confirm(`Delete model "${name}"?\n\nYou can re-download it later, but it may take time.`)) return;
    try {
      await invoke('delete_model', { filename });
      await loadModels();
    } catch (e) {
      console.error('Failed to delete model:', e);
    }
  }

  async function handleImportModel() {
    let selected: string | null = null;
    try {
      const result = await openDialog({
        multiple: false,
        filters: [{ name: 'Models', extensions: ['onnx', 'gguf'] }],
      });
      if (result && typeof result === 'string') selected = result;
    } catch {}
    if (!selected) selected = window.prompt('Path to .onnx or .gguf model file:');
    if (!selected) return;
    try {
      await invoke('import_custom_model', { sourcePath: selected });
      await loadModels();
    } catch (e) {
      console.error('Failed to import model:', e);
    }
  }

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
    { value: 'gemma-3-4b', label: 'Gemma 3 4B (2.49 GB) — Recommended', desc: 'Best balance of quality and speed, full vision support' },
    { value: 'gemma-3-1b', label: 'Gemma 3 1B (0.9 GB) — Lightweight', desc: 'Faster, uses less RAM, lower quality' },
  ];

  const scrfdOptions = [
    { value: 'scrfd-10g', label: 'SCRFD-10G (17 MB) — Most accurate' },
    { value: 'scrfd-2.5g', label: 'SCRFD-2.5G (3 MB) — Balanced' },
    { value: 'scrfd-500m', label: 'SCRFD-500M (1 MB) — Fastest' },
  ];

  async function handleImportApple() {
    let selected: string | null = null;
    try {
      const result = await open({ directory: true, multiple: false });
      if (result && typeof result === 'string') selected = result;
    } catch {}
    if (!selected) return;

    try {
      const source = await invoke('add_folder', { path: selected });
      await invoke('scan_folder', { path: selected });
      importMessage = 'Folder added and scanned successfully!';
      setTimeout(() => { importMessage = ''; }, 5000);
    } catch (e) {
      importMessage = `Error: ${e}`;
      setTimeout(() => { importMessage = ''; }, 5000);
    }
  }

  async function handleImportGoogleTakeout() {
    // Select source folder
    let sourcePath: string | null = null;
    try {
      const result = await open({ directory: true, multiple: false, title: $tStore('import.select_takeout') });
      if (result && typeof result === 'string') sourcePath = result;
    } catch {}
    if (!sourcePath) return;

    // Select destination folder
    let destPath: string | null = null;
    try {
      const result = await open({ directory: true, multiple: false, title: $tStore('import.select_dest') });
      if (result && typeof result === 'string') destPath = result;
    } catch {}
    if (!destPath) return;

    importLoading = true;
    importMessage = $tStore('import.importing');
    try {
      const result: any = await invoke('import_google_takeout', {
        sourcePath,
        destFolderPath: destPath,
      });
      importMessage = `${$tStore('import.done')}: ${result.imported} imported, ${result.with_metadata} with metadata`;
      // Also add the dest folder to the library
      try {
        await invoke('add_folder', { path: destPath });
        await invoke('scan_folder', { path: destPath });
      } catch {}
      setTimeout(() => { importMessage = ''; }, 10000);
    } catch (e) {
      importMessage = `Error: ${e}`;
      setTimeout(() => { importMessage = ''; }, 5000);
    }
    importLoading = false;
  }

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
        <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">{$tStore('settings.title')}</h2>
        <button
          class="w-8 h-8 rounded-lg flex items-center justify-center"
          style="background: var(--color-surface); color: var(--color-text-secondary);"
          onclick={close}
        >✕</button>
      </div>

      <!-- Language -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          {$tStore('settings.language')}
        </div>
        <select
          class="w-full px-3 py-2.5 rounded-xl text-sm border-none outline-none cursor-pointer"
          style="background: var(--color-surface); color: var(--color-text-primary);"
          bind:value={currentLocale}
        >
          {#each Object.entries(localeNames) as [code, name]}
            <option value={code}>{name}</option>
          {/each}
        </select>
      </div>

      <!-- Import -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          {$tStore('settings.import')}
        </div>
        <div class="space-y-2">
          <button
            class="w-full py-2.5 rounded-xl text-xs font-medium transition-all hover:scale-[1.02]"
            style="background: var(--color-surface); color: var(--color-text-secondary);"
            onclick={handleImportApple}
          >
            {$tStore('settings.import_apple')}
          </button>
          <button
            class="w-full py-2.5 rounded-xl text-xs font-medium transition-all hover:scale-[1.02]"
            style="background: var(--color-accent-soft); color: var(--color-accent);"
            onclick={handleImportGoogleTakeout}
            disabled={importLoading}
          >
            {importLoading ? $tStore('import.importing') : $tStore('settings.import_google')}
          </button>
          <p class="text-[10px] px-1" style="color: var(--color-text-muted);">
            {$tStore('settings.import_google_hint')}
          </p>
          {#if importMessage}
            <div class="text-xs px-3 py-2 rounded-xl" style="background: {importMessage.startsWith('Error') ? '#fee2e2' : 'var(--color-accent-soft)'}; color: {importMessage.startsWith('Error') ? '#dc2626' : 'var(--color-accent)'};">
              {importMessage}
            </div>
          {/if}
        </div>
      </div>

      <!-- AI Model Selection -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          {$tStore('settings.vision_model')}
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
          {$tStore('settings.face_model')}
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

      <!-- GPU Info -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          {$tStore('settings.performance')}
        </div>
        <div class="flex items-center gap-2 p-3 rounded-xl"
          style="background: var(--color-surface);">
          <div class="w-2 h-2 rounded-full shrink-0" style="background: {gpuInfo?.cuda_available ? '#34d399' : '#94a3b8'};"></div>
          <span class="text-sm" style="color: var(--color-text-primary);">
            {gpuInfo?.label ?? 'Checking GPU...'}
          </span>
        </div>
      </div>

      <!-- Storage Dashboard -->
      <div class="mb-6">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-3" style="color: var(--color-text-muted);">
          {$tStore('settings.storage')}
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
          {$tStore('settings.phone_sync')}
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

      <!-- Model Management (Advanced) -->
      <div class="mb-6">
        <div class="flex items-center justify-between mb-3">
          <div class="text-[11px] uppercase tracking-wider font-semibold" style="color: var(--color-text-muted);">
            {$tStore('settings.models_advanced')}
          </div>
          <button
            class="text-[10px] px-2 py-1 rounded-lg transition-all hover:scale-105"
            style="background: var(--color-accent-soft); color: var(--color-accent);"
            onclick={handleImportModel}
          >
            + Import Custom
          </button>
        </div>

        {#if modelsLoading}
          <div class="flex items-center justify-center py-4">
            <span class="inline-block animate-spin text-sm" style="color: var(--color-text-muted);">&#x25CC;</span>
          </div>
        {:else}
          <div class="space-y-2">
            {#each modelsList as model}
              <div class="flex items-center justify-between p-3 rounded-xl transition-all" style="background: var(--color-surface);">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <div class="text-xs font-medium truncate" style="color: var(--color-text-primary);">{model.name}</div>
                    {#if model.is_custom}
                      <span class="text-[9px] px-1.5 py-0.5 rounded-full shrink-0" style="background: var(--color-accent-soft); color: var(--color-accent);">Custom</span>
                    {/if}
                  </div>
                  <div class="text-[10px] truncate" style="color: var(--color-text-muted);">
                    {model.purpose} — {model.exists ? formatSize(model.size) : 'Not downloaded'}
                  </div>
                </div>
                <div class="flex items-center gap-1 shrink-0 ml-2">
                  {#if model.exists}
                    <span class="text-[9px] px-1.5 py-0.5 rounded-full" style="background: var(--color-success-soft); color: var(--color-success);">Ready</span>
                    <button
                      class="text-[10px] px-2 py-1 rounded-lg transition-all hover:scale-105"
                      style="color: var(--color-danger);"
                      onclick={() => handleDeleteModel(model.filename, model.name)}
                      title="Delete model to free space. Re-downloads on next use."
                    >
                      Delete
                    </button>
                  {:else}
                    <span class="text-[9px] px-1.5 py-0.5 rounded-full" style="background: var(--color-surface); color: var(--color-text-muted);">Pending</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>

          {#if modelsDir}
            <div class="text-[10px] mt-2 p-2 rounded-lg" style="background: var(--color-bg); color: var(--color-text-muted);">
              <strong>Path:</strong> {modelsDir}<br/>
              Models auto-download on first use. Import custom .onnx/.gguf models to override defaults.
            </div>
          {/if}
        {/if}
      </div>

      <!-- About -->
      <div class="mb-6 p-4 rounded-xl" style="background: var(--color-surface);">
        <div class="text-[11px] uppercase tracking-wider font-semibold mb-2" style="color: var(--color-text-muted);">
          {$tStore('settings.about')}
        </div>
        <div class="text-sm font-semibold" style="color: var(--color-text-primary);">Mim v1.0</div>
        <div class="text-xs mt-2" style="color: var(--color-text-muted);">
          Named after Mimir, the Norse god of wisdom and memory.
          Face detection, AI tagging, and photo chat — all running locally.
        </div>
        <div class="text-[10px] mt-2 p-2 rounded-lg" style="background: var(--color-bg); color: var(--color-text-muted);">
          <strong>Logs:</strong> ~/.local/share/mim/logs/ (Linux) or %APPDATA%/mim/logs/ (Windows)<br/>
          <strong>Database:</strong> ~/.local/share/mim/mim_central.db<br/>
          <strong>License:</strong> MIT
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-2">
        <button
          class="flex-1 px-4 py-2.5 rounded-xl text-sm font-medium"
          style="background: var(--color-accent-soft); color: var(--color-accent);"
          onclick={close}
        >
          {$tStore('action.done')}
        </button>
        <button
          class="px-4 py-2.5 rounded-xl text-sm"
          style="background: #fee2e2; color: #dc2626;"
          onclick={handleExit}
        >
          {$tStore('settings.exit')}
        </button>
      </div>
    </div>
  </div>
{/if}
