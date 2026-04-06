<script lang="ts">
  import { fade, fly } from 'svelte/transition';

  let {
    show = $bindable(false),
    title = 'Confirm',
    message = '',
    type = 'confirm' as 'confirm' | 'prompt' | 'password' | 'alert',
    placeholder = '',
    defaultValue = '',
    confirmText = 'OK',
    cancelText = 'Cancel',
    dangerous = false,
    onconfirm = (value?: string) => {},
    oncancel = () => {},
  } = $props();

  let inputValue = $state(defaultValue);
  let inputEl: HTMLInputElement;

  $effect(() => {
    if (show) {
      inputValue = defaultValue;
      setTimeout(() => inputEl?.focus(), 100);
    }
  });

  function handleConfirm() {
    if (type === 'prompt' || type === 'password') {
      onconfirm(inputValue);
    } else {
      onconfirm();
    }
    show = false;
  }

  function handleCancel() {
    oncancel();
    show = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') handleCancel();
    if (e.key === 'Enter') handleConfirm();
  }
</script>

{#if show}
  <div
    class="fixed inset-0 z-[300] flex items-center justify-center"
    style="background: rgba(0,0,0,0.5); backdrop-filter: blur(8px);"
    transition:fade={{ duration: 150 }}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={handleCancel}
    onkeydown={handleKeydown}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="glass-heavy w-[380px] rounded-2xl p-5"
      style="box-shadow: 0 20px 60px rgba(0,0,0,0.3);"
      onclick={(e) => e.stopPropagation()}
      transition:fly={{ y: 10, duration: 200 }}
    >
      <h3 class="text-sm font-semibold mb-2" style="color: var(--color-text-primary);">
        {title}
      </h3>

      {#if message}
        <p class="text-xs mb-4 whitespace-pre-line" style="color: var(--color-text-secondary);">
          {message}
        </p>
      {/if}

      {#if type === 'prompt' || type === 'password'}
        <input
          bind:this={inputEl}
          type={type === 'password' ? 'password' : 'text'}
          bind:value={inputValue}
          {placeholder}
          class="w-full px-3 py-2.5 rounded-xl text-sm mb-4 border-none outline-none"
          style="background: var(--color-surface); color: var(--color-text-primary);"
          onkeydown={(e) => e.key === 'Enter' && handleConfirm()}
        />
      {/if}

      <div class="flex gap-2 justify-end">
        {#if type !== 'alert'}
          <button
            class="px-4 py-2 rounded-xl text-xs font-medium transition-all"
            style="background: var(--color-surface); color: var(--color-text-secondary);"
            onclick={handleCancel}
          >
            {cancelText}
          </button>
        {/if}
        <button
          class="px-4 py-2 rounded-xl text-xs font-medium transition-all"
          style="background: {dangerous ? 'var(--color-danger-soft)' : 'var(--color-accent-soft)'}; color: {dangerous ? 'var(--color-danger)' : 'var(--color-accent)'};"
          onclick={handleConfirm}
        >
          {confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}
