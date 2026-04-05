<script lang="ts">
  import { searchQuery, sidebarOpen, currentSection } from '$lib/stores/ui';
  import { fly, fade } from 'svelte/transition';
  import Settings from './Settings.svelte';

  let searchFocused = $state(false);
  let showSettings = $state(false);

  function handleSearchFocus() {
    searchFocused = true;
    currentSection.set('search');
  }
</script>

<header
  class="glass-heavy fixed top-0 left-0 right-0 z-50 flex items-center gap-4 px-5"
  style="border-bottom: 1px solid var(--color-border-glass); height: 56px;"
  in:fly={{ y: -40, duration: 500 }}
>
  <!-- Logo with hover glow -->
  <button
    class="flex items-center gap-2.5 shrink-0 group"
    onclick={() => sidebarOpen.update(v => !v)}
  >
    <div class="w-8 h-8 rounded-xl flex items-center justify-center text-base font-bold transition-all duration-300 group-hover:scale-110 group-hover:rotate-3 accent-gradient"
      style="color: white; box-shadow: 0 2px 12px var(--color-accent-glow);">
      M
    </div>
    <span class="text-base font-semibold tracking-tight transition-colors duration-200" style="color: var(--color-text-primary);">
      Mim
    </span>
  </button>

  <!-- Search bar with expanding animation -->
  <div class="flex-1 max-w-xl mx-auto relative">
    <div
      class="flex items-center gap-2 px-4 py-2 rounded-2xl transition-all duration-300"
      style="
        background: {searchFocused ? 'var(--color-surface-elevated)' : 'var(--color-surface)'};
        box-shadow: {searchFocused ? '0 0 0 2px var(--color-accent-soft), var(--shadow-neu-soft)' : 'var(--shadow-neu-soft)'};
        transform: {searchFocused ? 'scale(1.02)' : 'scale(1)'};
      "
    >
      <span class="text-sm transition-all duration-300" style="color: {searchFocused ? 'var(--color-accent)' : 'var(--color-text-muted)'};">⌕</span>
      <input
        type="text"
        placeholder="Search photos, people, places..."
        class="flex-1 bg-transparent outline-none text-sm"
        style="color: var(--color-text-primary);"
        bind:value={$searchQuery}
        onfocus={handleSearchFocus}
        onblur={() => searchFocused = false}
      />
      {#if $searchQuery}
        <button
          class="text-xs px-1.5 py-0.5 rounded-lg hover:opacity-80 transition-all duration-200 hover:scale-110"
          style="color: var(--color-text-muted); background: var(--color-surface);"
          onclick={() => searchQuery.set('')}
          transition:fade={{ duration: 150 }}
        >
          ✕
        </button>
      {/if}
    </div>
  </div>

  <!-- Settings -->
  <button
    class="neu-button w-9 h-9 flex items-center justify-center rounded-xl shrink-0 transition-all duration-300 hover:rotate-90"
    style="background: var(--color-surface); color: var(--color-text-secondary);"
    onclick={() => showSettings = true}
    title="Settings"
  >
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="3"/>
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
    </svg>
  </button>
</header>

<Settings bind:show={showSettings} />
