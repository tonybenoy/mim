<script lang="ts">
  import { currentSection, type AppSection } from '$lib/stores/ui';
  import { fly } from 'svelte/transition';
  import { spring } from 'svelte/motion';
  import { tStore } from '$lib/i18n';

  const sectionDefs: { id: AppSection; labelKey: string; icon: string }[] = [
    { id: 'library', labelKey: 'nav.library', icon: '◈' },
    { id: 'people', labelKey: 'nav.people', icon: '◉' },
    { id: 'chat', labelKey: 'nav.chat', icon: '✦' },
    { id: 'albums', labelKey: 'nav.albums', icon: '▣' },
    { id: 'search', labelKey: 'nav.search', icon: '⌕' },
  ];

  let sections = $derived(sectionDefs.map(s => ({ ...s, label: $tStore(s.labelKey) })));

  let indicatorX = spring(0, { stiffness: 0.12, damping: 0.65 });
  let indicatorScale = spring(1, { stiffness: 0.3, damping: 0.7 });

  function switchSection(section: AppSection, index: number) {
    currentSection.set(section);
    indicatorX.set(index * 100);
    // Bounce effect
    indicatorScale.set(1.4);
    setTimeout(() => indicatorScale.set(1), 150);
  }

  $: {
    const idx = sections.findIndex(s => s.id === $currentSection);
    if (idx >= 0) indicatorX.set(idx * 100);
  }
</script>

<nav
  class="glass-heavy fixed bottom-0 left-0 right-0 z-50 flex items-center justify-around px-2"
  style="border-top: 1px solid var(--color-border-glass); height: 64px;"
  in:fly={{ y: 60, duration: 500, delay: 200 }}
>
  <!-- Animated indicator with glow -->
  <div
    class="absolute top-0 rounded-full tab-indicator"
    style="
      width: calc({100 / sections.length}% - 24px);
      height: 3px;
      left: calc({$indicatorX / sections.length}% + 12px);
      background: var(--color-accent);
      transform: scaleX({$indicatorScale});
    "
  ></div>

  {#each sections as section, i}
    <button
      class="relative flex flex-col items-center gap-0.5 px-4 py-2 rounded-2xl transition-all duration-300 group"
      style="
        color: {$currentSection === section.id ? 'var(--color-accent)' : 'var(--color-text-primary)'};
        {$currentSection === section.id ? 'background: var(--color-accent-soft);' : ''}
      "
      onclick={() => switchSection(section.id, i)}
    >
      <span class="text-xl leading-none transition-all duration-300"
        style="
          transform: {$currentSection === section.id ? 'scale(1.15) translateY(-1px)' : 'scale(1)'};
          filter: {$currentSection === section.id ? 'drop-shadow(0 0 6px var(--color-accent-glow))' : 'none'};
        ">
        {section.icon}
      </span>
      <span class="text-[10px] font-semibold tracking-wide transition-all duration-300"
        style="opacity: {$currentSection === section.id ? '1' : '0.6'}; color: {$currentSection === section.id ? 'var(--color-accent)' : 'var(--color-text-secondary)'};">
        {section.label}
      </span>
    </button>
  {/each}
</nav>
