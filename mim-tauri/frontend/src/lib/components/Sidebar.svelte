<script lang="ts">
  import { sidebarOpen, currentSection, mlStatus, mlStatusText, mlProgress } from '$lib/stores/ui';
  import { tStore } from '$lib/i18n';
  import { folders, activeFolder } from '$lib/stores/photos';
  import { addFolder, removeFolder, scanFolder, getFolders, getPhotos, getPhotoCount, lockFolder, unlockFolder, verifyFolderPassword, openLockedFolder } from '$lib/api/photos';
  import { processFaces, clusterFaces, onFaceProcessingProgress } from '$lib/api/faces';
  import { tagPhotos, onTaggingProgress, onGemmaStatus } from '$lib/api/gemma';
  import { analyzeFolder } from '$lib/api/analysis';
  import { isScanning } from '$lib/stores/ui';
  import { photos as photosStore } from '$lib/stores/photos';
  import { fly } from 'svelte/transition';
  import { open } from '@tauri-apps/plugin-dialog';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import type { FolderSource } from '$lib/api/photos';
  import PasswordPrompt from './PasswordPrompt.svelte';

  // File watcher notification state
  let watcherNotification = $state<{ folder: string; count: number; name: string } | null>(null);

  // Locked folder state
  let lockedFolders = $state<Set<string>>(new Set());
  let unlockedFolders = $state<Set<string>>(new Set());
  let showPasswordPrompt = $state(false);
  let pendingLockedFolder = $state<FolderSource | null>(null);
  let passwordError = $state('');

  // Lock/unlock menu
  let showLockMenu = $state<string | null>(null);

  function handleGlobalClick(e: MouseEvent) {
    if (showLockMenu && !(e.target as HTMLElement)?.closest('[data-lock-menu]')) {
      showLockMenu = null;
    }
  }

  // Per-folder status tracking
  let folderStatus = $state<Record<string, {
    stage: string;
    text: string;
    progress?: number;
  }>>({});

  // Global status
  let globalProcessing = $state(false);
  let globalText = $state('');

  // Download progress
  let downloadInfo = $state<{ filename: string; downloaded: number; total: number; pct: number } | null>(null);

  let unlistens: (() => void)[] = [];

  onMount(() => {
    onFaceProcessingProgress((progress) => {
      mlProgress.set(progress);
      mlStatusText.set(`${progress.processed}/${progress.total} photos — ${progress.faces_found} faces`);
    }).then(fn => unlistens.push(fn));

    listen<any>('face-status', (event) => {
      if (event.payload === 'downloading-models') {
        mlStatus.set('downloading');
        mlStatusText.set('Downloading face models (first run only)...');
      } else if (event.payload === 'loading-models') {
        mlStatus.set('downloading');
        mlStatusText.set('Loading face models into memory...');
      } else if (event.payload === 'models-ready') {
        mlStatusText.set('Models loaded, starting detection...');
      } else if (event.payload === 'detecting') {
        mlStatus.set('detecting');
        mlStatusText.set('Detecting faces...');
      }
    }).then(fn => unlistens.push(fn));

    listen<any>('scan-status', (event) => {
      const { folder, stage, total, new: newCount } = event.payload;
      const name = folder?.split('/').pop() || folder;
      if (stage === 'scanning') {
        folderStatus = { ...folderStatus, [folder]: { stage: 'scanning', text: 'Scanning files...' } };
      } else if (stage === 'thumbnails') {
        folderStatus = { ...folderStatus, [folder]: { stage: 'thumbnails', text: `${newCount} new / ${total} total — thumbnails...` } };
      } else if (stage === 'done') {
        folderStatus = { ...folderStatus, [folder]: { stage: 'done', text: `${newCount} new photos found` } };
        setTimeout(() => {
          const s = { ...folderStatus };
          delete s[folder];
          folderStatus = s;
        }, 3000);
      }
    }).then(fn => unlistens.push(fn));

    onTaggingProgress((progress) => {
      globalText = `Tagging: ${progress.processed}/${progress.total} — ${progress.tagged} tagged`;
    }).then(fn => unlistens.push(fn));

    onGemmaStatus((status) => {
      if (status === 'downloading-models') {
        globalProcessing = true;
        globalText = 'Downloading AI models (first run only)...';
      } else if (status === 'loading-model') {
        globalProcessing = true;
        globalText = 'Loading Gemma into memory (may take 30-60s)...';
      } else if (status === 'model-loaded') {
        globalText = 'AI model ready, starting tagging...';
      } else if (status === 'tagging') {
        globalText = 'AI tagging in progress...';
      }
    }).then(fn => unlistens.push(fn));

    listen<any>('download-progress', (event) => {
      const { filename, downloaded, total, pct } = event.payload;
      if (total > 0) {
        const mb = (downloaded / 1048576).toFixed(0);
        const totalMb = (total / 1048576).toFixed(0);
        downloadInfo = { filename, downloaded, total, pct };
        globalProcessing = true;
        globalText = `Downloading ${filename} — ${mb}/${totalMb} MB (${pct}%)`;
      } else {
        downloadInfo = null;
      }
    }).then(fn => unlistens.push(fn));

    // Listen for file watcher events
    listen<any>('folder-files-changed', (event) => {
      const { folder, count } = event.payload;
      const name = folder?.split('/').pop() || folder;
      watcherNotification = { folder, count, name };
      // Auto-dismiss after 10 seconds
      setTimeout(() => {
        if (watcherNotification?.folder === folder) {
          watcherNotification = null;
        }
      }, 10000);
    }).then(fn => unlistens.push(fn));

    // Auto-start watchers for all existing folders
    getFolders().then(f => {
      for (const folder of f) {
        invoke('watch_folder', { folderPath: folder.path }).catch(() => {});
      }
    }).catch(() => {});

    return () => unlistens.forEach(fn => fn());
  });

  // Folder counts
  let folderCounts = $state<Record<string, number>>({});
  $effect(() => {
    for (const folder of $folders) {
      getPhotoCount(folder.path).then(c => {
        folderCounts = { ...folderCounts, [folder.path]: c };
      }).catch(() => {});
    }
  });

  async function handleAddFolder() {
    let selected: string | null = null;
    try {
      const result = await open({ directory: true, multiple: false });
      if (result && typeof result === 'string') selected = result;
    } catch {}
    if (!selected) selected = window.prompt('Enter folder path:');
    if (!selected) return;

    try {
      const source = await addFolder(selected);
      folders.update(f => [...f, source]);
      activeFolder.set(source);
      await handleScanFolder(source);
      // Start watching the folder for new files
      invoke('watch_folder', { folderPath: source.path }).catch(() => {});
    } catch (e) {
      console.error('Failed to add folder:', e);
    }
  }

  async function handleScanFolder(folder: FolderSource) {
    folderStatus = { ...folderStatus, [folder.path]: { stage: 'scanning', text: 'Starting scan...' } };
    try {
      await scanFolder(folder.path);
      // Reload photos if this is the active folder
      if ($activeFolder?.id === folder.id) {
        const p = await getPhotos(folder.path, 500, 0);
        photosStore.set(p);
      }
      const c = await getPhotoCount(folder.path);
      folderCounts = { ...folderCounts, [folder.path]: c };
    } catch (e) {
      folderStatus = { ...folderStatus, [folder.path]: { stage: 'error', text: `Error: ${e}` } };
    }
  }

  async function handleProcessFolder(folder: FolderSource) {
    folderStatus = { ...folderStatus, [folder.path]: { stage: 'faces', text: 'Detecting faces...' } };
    try {
      const result = await processFaces(folder.path);
      if (result.faces_found > 0) {
        folderStatus = { ...folderStatus, [folder.path]: { stage: 'clustering', text: 'Clustering faces...' } };
        await clusterFaces(folder.path);
      }
      folderStatus = { ...folderStatus, [folder.path]: { stage: 'done', text: `${result.faces_found} faces found` } };
      setTimeout(() => { const s = { ...folderStatus }; delete s[folder.path]; folderStatus = s; }, 3000);
    } catch (e) {
      folderStatus = { ...folderStatus, [folder.path]: { stage: 'error', text: `${e}` } };
    }
  }

  async function handleTagFolder(folder: FolderSource) {
    folderStatus = { ...folderStatus, [folder.path]: { stage: 'tagging', text: 'AI tagging...' } };
    try {
      const result = await tagPhotos(folder.path);
      folderStatus = { ...folderStatus, [folder.path]: { stage: 'done', text: `${result.tagged} photos tagged` } };
      setTimeout(() => { const s = { ...folderStatus }; delete s[folder.path]; folderStatus = s; }, 3000);
    } catch (e) {
      folderStatus = { ...folderStatus, [folder.path]: { stage: 'error', text: `${e}` } };
    }
  }

  async function handleProcessAll() {
    globalProcessing = true;

    // Scan all folders in parallel (I/O bound, safe to parallelize)
    globalText = 'Scanning all folders...';
    await Promise.allSettled($folders.map(f => handleScanFolder(f)));

    // Run AI sequentially per folder (GPU/memory bound)
    for (const folder of $folders) {
      globalText = `Detecting faces in ${folder.path.split(/[/\\]/).pop()}...`;
      await handleProcessFolder(folder);
    }

    globalProcessing = false;
    globalText = 'All done!';
    setTimeout(() => { globalText = ''; }, 3000);
  }

  function selectFolder(folder: FolderSource) {
    if (lockedFolders.has(folder.path) && !unlockedFolders.has(folder.path)) {
      pendingLockedFolder = folder;
      passwordError = '';
      showPasswordPrompt = true;
      return;
    }
    activeFolder.set(folder);
  }

  async function handlePasswordSubmit(password: string) {
    if (!pendingLockedFolder) return;
    try {
      const valid = await verifyFolderPassword(pendingLockedFolder.path, password);
      if (valid) {
        await openLockedFolder(pendingLockedFolder.path, password);
        unlockedFolders = new Set([...unlockedFolders, pendingLockedFolder.path]);
        activeFolder.set(pendingLockedFolder);
        showPasswordPrompt = false;
        pendingLockedFolder = null;
      } else {
        passwordError = 'Incorrect password';
      }
    } catch (e) {
      passwordError = `${e}`;
    }
  }

  function handlePasswordCancel() {
    showPasswordPrompt = false;
    pendingLockedFolder = null;
    passwordError = '';
  }

  async function handleLockFolder(folder: FolderSource) {
    const password = window.prompt('Set a password for this folder:');
    if (!password) return;
    const confirm = window.prompt('Confirm password:');
    if (password !== confirm) {
      window.alert('Passwords do not match');
      return;
    }
    try {
      await lockFolder(folder.path, password);
      lockedFolders = new Set([...lockedFolders, folder.path]);
      showLockMenu = null;
    } catch (e) {
      console.error('Failed to lock folder:', e);
    }
  }

  async function handleUnlockFolder(folder: FolderSource) {
    const password = window.prompt('Enter password to remove lock:');
    if (!password) return;
    try {
      await unlockFolder(folder.path, password);
      const next = new Set(lockedFolders);
      next.delete(folder.path);
      lockedFolders = next;
      const nextU = new Set(unlockedFolders);
      nextU.delete(folder.path);
      unlockedFolders = nextU;
      showLockMenu = null;
    } catch (e) {
      console.error('Failed to unlock folder:', e);
      window.alert('Incorrect password or failed to unlock');
    }
  }

  async function handleAnalyzeFolder(folder: FolderSource) {
    folderStatus = { ...folderStatus, [folder.path]: { stage: 'analyzing', text: 'Analyzing photos...' } };
    try {
      const result = await analyzeFolder(folder.path);
      folderStatus = { ...folderStatus, [folder.path]: { stage: 'done', text: `Analyzed ${result.processed} photos` } };
      setTimeout(() => { const s = { ...folderStatus }; delete s[folder.path]; folderStatus = s; }, 3000);
    } catch (e) {
      folderStatus = { ...folderStatus, [folder.path]: { stage: 'error', text: `${e}` } };
    }
  }

  async function handleRemoveFolder(folder: FolderSource) {
    if (!window.confirm(`Remove "${folder.label || folder.path.split(/[/\\]/).pop()}" from Mim?\n\nThis will NOT delete your photos — only remove it from Mim's library.`)) return;
    try {
      await removeFolder(folder.id);
      folders.update(f => f.filter(fo => fo.id !== folder.id));
      if ($activeFolder?.id === folder.id) {
        const remaining = $folders;
        activeFolder.set(remaining.length > 0 ? remaining[0] : null);
      }
      showLockMenu = null;
    } catch (e) {
      console.error('Failed to remove folder:', e);
    }
  }

  // Load folders on mount
  $effect(() => {
    getFolders().then(f => {
      folders.set(f);
      if (f.length > 0 && !$activeFolder) activeFolder.set(f[0]);
    }).catch(() => {});
  });
</script>

<svelte:window onclick={handleGlobalClick} />

{#if $sidebarOpen}
  <aside
    class="glass fixed left-0 z-40 flex flex-col gap-1 p-3 overflow-y-auto"
    style="top: 56px; bottom: 64px; width: 240px; border-right: 1px solid var(--color-border-glass);"
    transition:fly={{ x: -240, duration: 300 }}
  >
    {#if $currentSection === 'library' || $currentSection === 'chat' || $currentSection === 'search' || $currentSection === 'trash' || $currentSection === 'map' || $currentSection === 'memories'}
      <div class="flex items-center justify-between px-2 py-1">
        <div class="text-[11px] font-semibold uppercase tracking-widest"
          style="color: var(--color-text-muted);">
          {$tStore('sidebar.folders')}
        </div>
        <button
          class="text-[10px] px-2 py-1 rounded-lg transition-all hover:scale-105"
          style="background: var(--color-accent-soft); color: var(--color-accent);"
          onclick={handleAddFolder}
        >
          {$tStore('sidebar.add')}
        </button>
      </div>

      <!-- Folder tree -->
      {#each $folders as folder, i}
        <div class="rounded-xl transition-all duration-200 mb-1"
          style="background: {$activeFolder?.id === folder.id ? 'var(--color-accent-soft)' : 'transparent'};"
          in:fly={{ x: -20, duration: 200, delay: i * 40 }}
        >
          <!-- Folder row -->
          <div class="flex items-center">
            <button
              class="flex items-center gap-2 px-3 py-2 flex-1 text-left text-sm rounded-xl transition-all duration-200"
              style="color: {$activeFolder?.id === folder.id ? 'var(--color-accent)' : 'var(--color-text-primary)'};"
              onclick={() => selectFolder(folder)}
            >
              <span class="text-xs">{$activeFolder?.id === folder.id ? '&#x25C6;' : '&#x25C7;'}</span>
              {#if lockedFolders.has(folder.path) && !unlockedFolders.has(folder.path)}
                <span class="text-xs">&#x1F512;</span>
              {/if}
              <span class="truncate flex-1">{folder.label || folder.path.split(/[/\\]/).pop()}</span>
              <span class="text-[10px] px-1.5 py-0.5 rounded-full"
                style="background: var(--color-surface); color: var(--color-text-muted);">
                {folderCounts[folder.path] ?? '&#x2014;'}
              </span>
            </button>
            <!-- Lock/unlock toggle -->
            <div class="relative" data-lock-menu>
              <button
                class="text-[10px] px-1 py-1 rounded-lg transition-all hover:scale-110"
                style="color: var(--color-text-muted);"
                onclick={() => showLockMenu = showLockMenu === folder.path ? null : folder.path}
                title="Lock settings"
              >
                &#x22EE;
              </button>
              {#if showLockMenu === folder.path}
                <div
                  class="absolute right-0 top-full mt-1 z-50 rounded-xl p-1.5 min-w-[140px]"
                  style="background: var(--color-surface-elevated); box-shadow: 0 8px 24px rgba(0,0,0,0.3); border: 1px solid var(--color-border-glass);"
                >
                  {#if lockedFolders.has(folder.path)}
                    <button
                      class="w-full text-left text-[11px] px-2 py-1.5 rounded-md transition-colors hover:opacity-80"
                      style="color: var(--color-text-secondary);"
                      onclick={() => handleUnlockFolder(folder)}
                    >
                      Remove Lock
                    </button>
                  {:else}
                    <button
                      class="w-full text-left text-[11px] px-2 py-1.5 rounded-md transition-colors hover:opacity-80"
                      style="color: var(--color-text-secondary);"
                      onclick={() => handleLockFolder(folder)}
                    >
                      Lock Folder
                    </button>
                  {/if}
                  <button
                    class="w-full text-left text-[11px] px-2 py-1.5 rounded-md transition-colors hover:opacity-80"
                    style="color: var(--color-danger);"
                    onclick={() => handleRemoveFolder(folder)}
                  >
                    Remove from Mim
                  </button>
                </div>
              {/if}
            </div>
          </div>

          <!-- Per-folder status -->
          {#if folderStatus[folder.path]}
            {@const status = folderStatus[folder.path]}
            <div class="px-3 pb-2">
              <div class="flex items-center gap-1.5 text-[10px] rounded-lg px-2 py-1.5 cursor-default"
                style="background: var(--color-surface); color: {status.stage === 'error' ? 'var(--color-danger)' : status.stage === 'done' ? 'var(--color-success)' : 'var(--color-accent)'};"
                title={status.text}>
                {#if status.stage !== 'done' && status.stage !== 'error'}
                  <span class="animate-spin">◌</span>
                {/if}
                <span class="truncate">{status.text}</span>
              </div>
            </div>
          {/if}

          <!-- Per-folder actions (when active) -->
          {#if $activeFolder?.id === folder.id}
            <div class="flex gap-1 px-3 pb-2">
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-lg transition-all hover:scale-105"
                style="background: var(--color-surface); color: var(--color-text-secondary);"
                onclick={() => handleScanFolder(folder)}
                disabled={!!folderStatus[folder.path]}
                title="Rescan for new photos"
              >
                ↻ {$tStore('sidebar.scan')}
              </button>
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-lg transition-all hover:scale-105"
                style="background: var(--color-surface); color: var(--color-text-secondary);"
                onclick={() => handleProcessFolder(folder)}
                disabled={!!folderStatus[folder.path]}
                title="Detect faces"
              >
                ◉ {$tStore('sidebar.faces')}
              </button>
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-lg transition-all hover:scale-105"
                style="background: var(--color-surface); color: var(--color-text-secondary);"
                onclick={() => handleTagFolder(folder)}
                disabled={!!folderStatus[folder.path]}
                title="AI tag photos"
              >
                ✦ {$tStore('sidebar.tag')}
              </button>
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-lg transition-all hover:scale-105"
                style="background: var(--color-surface); color: var(--color-text-secondary);"
                onclick={() => handleAnalyzeFolder(folder)}
                disabled={!!folderStatus[folder.path]}
                title="Analyze photos (colors, blur, events)"
              >
                ◎ {$tStore('sidebar.analyze')}
              </button>
            </div>
          {/if}
        </div>
      {/each}

      <!-- Global actions -->
      {#if $folders.length > 0}
        <div class="mt-2 pt-2" style="border-top: 1px solid var(--color-border-glass);">
          <button
            class="neu-button flex items-center justify-center gap-2 px-3 py-2.5 rounded-xl text-xs w-full"
            style="background: var(--color-accent-soft); color: var(--color-accent);"
            onclick={handleProcessAll}
            disabled={globalProcessing}
          >
            {#if globalProcessing}
              <span class="animate-spin">◌</span>
            {/if}
            <span>{globalProcessing ? $tStore('sidebar.processing') : '▶ ' + $tStore('sidebar.processAll')}</span>
          </button>
          {#if globalText}
            <div class="text-[10px] text-center mt-1.5 px-2" style="color: var(--color-text-muted);">
              {globalText}
            </div>
          {/if}

          <!-- File watcher notification -->
          {#if watcherNotification}
            <div class="mt-2 px-1 py-2 rounded-xl text-xs" style="background: var(--color-accent-soft); color: var(--color-accent);">
              <div class="px-2 mb-1">{watcherNotification.count} new file{watcherNotification.count !== 1 ? 's' : ''} detected in {watcherNotification.name}</div>
              <button
                class="w-full py-1.5 rounded-lg text-[10px] font-medium transition-all hover:scale-[1.02]"
                style="background: var(--color-accent); color: white;"
                onclick={() => {
                  const folder = $folders.find(f => f.path === watcherNotification?.folder);
                  if (folder) handleScanFolder(folder);
                  watcherNotification = null;
                }}
              >
                Scan Now
              </button>
            </div>
          {/if}

          <!-- Download progress bar -->
          {#if downloadInfo}
            <div class="mt-2 px-1">
              <div class="w-full rounded-full h-1.5 overflow-hidden" style="background: var(--color-border-glass);">
                <div
                  class="h-full rounded-full transition-all duration-300"
                  style="width: {downloadInfo.pct}%; background: var(--color-accent);"
                ></div>
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Extra navigation links -->
      <div class="mt-auto pt-3" style="border-top: 1px solid var(--color-border-glass);">
        <div class="text-[10px] font-semibold uppercase tracking-widest px-3 py-1"
          style="color: var(--color-text-muted);">
          {$tStore('sidebar.more')}
        </div>
        <button
          class="flex items-center gap-2 px-3 py-2 w-full text-left text-xs rounded-lg transition-all"
          style="color: {$currentSection === 'trash' ? 'var(--color-accent)' : 'var(--color-text-secondary)'}; background: {$currentSection === 'trash' ? 'var(--color-accent-soft)' : 'transparent'};"
          onclick={() => currentSection.set('trash')}
        >
          <span>&#x2298;</span> {$tStore('sidebar.trash')}
        </button>
        <button
          class="flex items-center gap-2 px-3 py-2 w-full text-left text-xs rounded-lg transition-all"
          style="color: {$currentSection === 'map' ? 'var(--color-accent)' : 'var(--color-text-secondary)'}; background: {$currentSection === 'map' ? 'var(--color-accent-soft)' : 'transparent'};"
          onclick={() => currentSection.set('map')}
        >
          <span>&#x25CE;</span> {$tStore('sidebar.places')}
        </button>
        <button
          class="flex items-center gap-2 px-3 py-2 w-full text-left text-xs rounded-lg transition-all"
          style="color: {$currentSection === 'memories' ? 'var(--color-accent)' : 'var(--color-text-secondary)'}; background: {$currentSection === 'memories' ? 'var(--color-accent-soft)' : 'transparent'};"
          onclick={() => currentSection.set('memories')}
        >
          <span>&#x274B;</span> {$tStore('sidebar.memories')}
        </button>
      </div>

    {:else if $currentSection === 'people'}
      <div class="text-[11px] font-semibold uppercase tracking-widest px-3 py-2"
        style="color: var(--color-text-muted);">
        {$tStore('nav.people')}
      </div>
      <div class="px-3 py-4 text-center text-xs" style="color: var(--color-text-muted);">
        {$tStore('sidebar.people_hint')}
      </div>

    {:else if $currentSection === 'albums'}
      <div class="text-[11px] font-semibold uppercase tracking-widest px-3 py-2"
        style="color: var(--color-text-muted);">
        {$tStore('nav.albums')}
      </div>

    {:else if $currentSection === 'trash'}
      <div class="text-[11px] font-semibold uppercase tracking-widest px-3 py-2"
        style="color: var(--color-text-muted);">
        {$tStore('sidebar.trash')}
      </div>
      <div class="px-3 py-4 text-center text-xs" style="color: var(--color-text-muted);">
        {$tStore('sidebar.trash_hint')}
      </div>
    {/if}
  </aside>
{/if}

{#if showPasswordPrompt && pendingLockedFolder}
  <PasswordPrompt
    title="Unlock Folder"
    folderName={pendingLockedFolder.label || pendingLockedFolder.path.split(/[/\\]/).pop() || ''}
    onsubmit={handlePasswordSubmit}
    oncancel={handlePasswordCancel}
  />
{/if}
