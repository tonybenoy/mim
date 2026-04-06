<script lang="ts">
  import { dialogState } from '$lib/dialog';
  import { fade, fly } from 'svelte/transition';

  let inputValue = $state('');
  let inputEl: HTMLInputElement;

  $effect(() => {
    if ($dialogState.show) {
      inputValue = $dialogState.defaultValue;
      setTimeout(() => inputEl?.focus(), 100);
    }
  });

  function handleConfirm() {
    const type = $dialogState.type;
    if (type === 'prompt' || type === 'password') {
      $dialogState.resolve(inputValue);
    } else {
      $dialogState.resolve(true);
    }
    dialogState.update(s => ({ ...s, show: false }));
  }

  function handleCancel() {
    const type = $dialogState.type;
    if (type === 'prompt' || type === 'password') {
      $dialogState.resolve(null);
    } else if (type === 'alert') {
      $dialogState.resolve(true);
    } else {
      $dialogState.resolve(false);
    }
    dialogState.update(s => ({ ...s, show: false }));
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!$dialogState.show) return;
    if (e.key === 'Escape') handleCancel();
    if (e.key === 'Enter') handleConfirm();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $dialogState.show}
  <div
    class="fixed inset-0 z-[300] flex items-center justify-center"
    style="background: rgba(0,0,0,0.5); backdrop-filter: blur(8px);"
    transition:fade={{ duration: 150 }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={handleCancel}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="glass-heavy w-[400px] rounded-2xl p-6"
      style="box-shadow: 0 20px 60px rgba(0,0,0,0.3);"
      onclick={(e) => e.stopPropagation()}
      transition:fly={{ y: 10, duration: 200 }}
    >
      <h3 class="text-base font-semibold mb-2" style="color: var(--color-text-primary);">
        {$dialogState.title}
      </h3>

      {#if $dialogState.message}
        <p class="text-sm mb-4 whitespace-pre-line leading-relaxed" style="color: var(--color-text-secondary);">
          {$dialogState.message}
        </p>
      {/if}

      {#if $dialogState.type === 'prompt' || $dialogState.type === 'password'}
        <input
          bind:this={inputEl}
          type={$dialogState.type === 'password' ? 'password' : 'text'}
          bind:value={inputValue}
          placeholder={$dialogState.placeholder}
          class="w-full px-4 py-3 rounded-xl text-sm mb-4 border-none outline-none"
          style="background: var(--color-surface); color: var(--color-text-primary); box-shadow: var(--shadow-neu-pressed);"
        />
      {/if}

      <div class="flex gap-2 justify-end">
        {#if $dialogState.type !== 'alert'}
          <button
            class="neu-button px-5 py-2.5 rounded-xl text-sm font-medium"
            style="background: var(--color-surface); color: var(--color-text-secondary);"
            onclick={handleCancel}
          >
            {$dialogState.cancelText}
          </button>
        {/if}
        <button
          class="px-5 py-2.5 rounded-xl text-sm font-medium transition-all hover:scale-105"
          style="background: {$dialogState.dangerous ? 'var(--color-danger-soft)' : 'var(--color-accent-soft)'}; color: {$dialogState.dangerous ? 'var(--color-danger)' : 'var(--color-accent)'};"
          onclick={handleConfirm}
        >
          {$dialogState.confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}
