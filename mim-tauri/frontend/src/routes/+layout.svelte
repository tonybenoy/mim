<script lang="ts">
  import '../app.css';
  import TopBar from '$lib/components/TopBar.svelte';
  import BottomBar from '$lib/components/BottomBar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import PhotoDetail from '$lib/components/PhotoDetail.svelte';
  import GlobalDialog from '$lib/components/GlobalDialog.svelte';
  import { sidebarOpen, themeMode, type ThemeMode } from '$lib/stores/ui';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { children } = $props();

  // Apply theme class
  $effect(() => {
    const mode = $themeMode;
    const root = document.documentElement;
    root.classList.remove('dark', 'light');
    if (mode === 'dark') root.classList.add('dark');
    else if (mode === 'light') root.classList.add('light');
  });

  // Persist theme changes
  $effect(() => {
    invoke('set_setting', { key: 'theme', value: $themeMode }).catch(() => {});
  });

  // Load persisted theme on startup
  onMount(async () => {
    try {
      const saved = await invoke('get_setting', { key: 'theme' }) as string | null;
      if (saved && ['auto', 'light', 'dark'].includes(saved)) {
        themeMode.set(saved as ThemeMode);
      }
    } catch {}
  });
</script>

<div class="fixed inset-0 flex flex-col" style="background: var(--color-bg);">
  <TopBar />

  <div class="flex flex-1 min-h-0" style="margin-top: 56px; margin-bottom: 64px;">
    <Sidebar />

    <main
      class="flex-1 overflow-y-auto overflow-x-hidden min-h-0"
      style="margin-left: {$sidebarOpen ? '240px' : '0'}; transition: margin-left 0.35s cubic-bezier(0.25, 0.46, 0.45, 0.94);"
    >
      {@render children()}
    </main>
  </div>

  <BottomBar />
  <PhotoDetail />
  <GlobalDialog />
</div>
