<script lang="ts">
  import { sidebarOpen, currentSection, mlStatus, mlStatusText, mlProgress } from '$lib/stores/ui';
  import { folders, activeFolder } from '$lib/stores/photos';
  import { addFolder, scanFolder, getFolders, getPhotos, getPhotoCount } from '$lib/api/photos';
  import { processFaces, clusterFaces, onFaceProcessingProgress } from '$lib/api/faces';
  import { tagPhotos, onTaggingProgress, onGemmaStatus } from '$lib/api/gemma';
  import { isScanning } from '$lib/stores/ui';
  import { photos as photosStore } from '$lib/stores/photos';
  import { fly } from 'svelte/transition';
  import { open } from '@tauri-apps/plugin-dialog';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import type { FolderSource } from '$lib/api/photos';

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
        mlStatusText.set('Downloading face models...');
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
      if (status === 'loading-model') {
        globalProcessing = true;
        globalText = 'Loading Gemma model...';
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
    globalText = 'Processing all folders...';
    for (const folder of $folders) {
      globalText = `Scanning ${folder.path.split('/').pop()}...`;
      await handleScanFolder(folder);
      globalText = `Detecting faces in ${folder.path.split('/').pop()}...`;
      await handleProcessFolder(folder);
    }
    globalProcessing = false;
    globalText = 'All done!';
    setTimeout(() => { globalText = ''; }, 3000);
  }

  function selectFolder(folder: FolderSource) {
    activeFolder.set(folder);
  }

  // Load folders on mount
  $effect(() => {
    getFolders().then(f => {
      folders.set(f);
      if (f.length > 0 && !$activeFolder) activeFolder.set(f[0]);
    }).catch(() => {});
  });
</script>

{#if $sidebarOpen}
  <aside
    class="glass fixed left-0 z-40 flex flex-col gap-1 p-3 overflow-y-auto"
    style="top: 56px; bottom: 64px; width: 240px; border-right: 1px solid var(--color-border-glass);"
    transition:fly={{ x: -240, duration: 300 }}
  >
    {#if $currentSection === 'library' || $currentSection === 'chat' || $currentSection === 'search'}
      <div class="flex items-center justify-between px-2 py-1">
        <div class="text-[11px] font-semibold uppercase tracking-widest"
          style="color: var(--color-text-muted);">
          Folders
        </div>
        <button
          class="text-[10px] px-2 py-1 rounded-lg transition-all hover:scale-105"
          style="background: var(--color-accent-soft); color: var(--color-accent);"
          onclick={handleAddFolder}
        >
          + Add
        </button>
      </div>

      <!-- Folder tree -->
      {#each $folders as folder, i}
        <div class="rounded-xl transition-all duration-200 mb-1"
          style="background: {$activeFolder?.id === folder.id ? 'var(--color-accent-soft)' : 'transparent'};"
          in:fly={{ x: -20, duration: 200, delay: i * 40 }}
        >
          <!-- Folder row -->
          <button
            class="flex items-center gap-2 px-3 py-2 w-full text-left text-sm rounded-xl transition-all duration-200"
            style="color: {$activeFolder?.id === folder.id ? 'var(--color-accent)' : 'var(--color-text-primary)'};"
            onclick={() => selectFolder(folder)}
          >
            <span class="text-xs">{$activeFolder?.id === folder.id ? '◆' : '◇'}</span>
            <span class="truncate flex-1">{folder.label || folder.path.split('/').pop()}</span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full"
              style="background: var(--color-surface); color: var(--color-text-muted);">
              {folderCounts[folder.path] ?? '—'}
            </span>
          </button>

          <!-- Per-folder status -->
          {#if folderStatus[folder.path]}
            {@const status = folderStatus[folder.path]}
            <div class="px-3 pb-2">
              <div class="flex items-center gap-1.5 text-[10px] rounded-lg px-2 py-1.5"
                style="background: var(--color-surface); color: {status.stage === 'error' ? 'var(--color-danger)' : status.stage === 'done' ? 'var(--color-success)' : 'var(--color-accent)'};">
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
                ↻ Scan
              </button>
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-lg transition-all hover:scale-105"
                style="background: var(--color-surface); color: var(--color-text-secondary);"
                onclick={() => handleProcessFolder(folder)}
                disabled={!!folderStatus[folder.path]}
                title="Detect faces"
              >
                ◉ Faces
              </button>
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-lg transition-all hover:scale-105"
                style="background: var(--color-surface); color: var(--color-text-secondary);"
                onclick={() => handleTagFolder(folder)}
                disabled={!!folderStatus[folder.path]}
                title="AI tag photos"
              >
                ✦ Tag
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
            <span>{globalProcessing ? 'Processing...' : '▶ Process All Folders'}</span>
          </button>
          {#if globalText}
            <div class="text-[10px] text-center mt-1.5 px-2" style="color: var(--color-text-muted);">
              {globalText}
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

    {:else if $currentSection === 'people'}
      <div class="text-[11px] font-semibold uppercase tracking-widest px-3 py-2"
        style="color: var(--color-text-muted);">
        People
      </div>
      <div class="px-3 py-4 text-center text-xs" style="color: var(--color-text-muted);">
        Select a folder and run face detection to see people here
      </div>

    {:else if $currentSection === 'albums'}
      <div class="text-[11px] font-semibold uppercase tracking-widest px-3 py-2"
        style="color: var(--color-text-muted);">
        Albums
      </div>
    {/if}
  </aside>
{/if}
