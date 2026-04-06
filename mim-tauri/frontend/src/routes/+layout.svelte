<script lang="ts">
  import '../app.css';
  import TopBar from '$lib/components/TopBar.svelte';
  import BottomBar from '$lib/components/BottomBar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import PhotoDetail from '$lib/components/PhotoDetail.svelte';
  import { sidebarOpen, themeMode } from '$lib/stores/ui';

  let { children } = $props();

  $effect(() => {
    const mode = $themeMode;
    const root = document.documentElement;
    root.classList.remove('dark', 'light');
    if (mode === 'dark') root.classList.add('dark');
    else if (mode === 'light') root.classList.add('light');
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
</div>
