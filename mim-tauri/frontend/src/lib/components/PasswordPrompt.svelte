<script lang="ts">
  import { fade, fly } from 'svelte/transition';

  let {
    title = 'Enter Password',
    folderName = '',
    onsubmit,
    oncancel,
  }: {
    title?: string;
    folderName?: string;
    onsubmit: (password: string) => void;
    oncancel: () => void;
  } = $props();

  let password = $state('');
  let error = $state('');
  let submitting = $state(false);

  function handleSubmit() {
    if (!password.trim()) {
      error = 'Password is required';
      return;
    }
    error = '';
    submitting = true;
    onsubmit(password);
  }

  export function setError(msg: string) {
    error = msg;
    submitting = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') oncancel();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[200] flex items-center justify-center"
  style="background: rgba(0,0,0,0.6); backdrop-filter: blur(10px);"
  transition:fade={{ duration: 200 }}
  onclick={oncancel}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="glass-heavy w-[380px] rounded-2xl p-6"
    style="box-shadow: 0 20px 60px rgba(0,0,0,0.3);"
    onclick={(e) => e.stopPropagation()}
    transition:fly={{ y: 20, duration: 300 }}
  >
    <div class="flex items-center gap-3 mb-4">
      <div class="w-10 h-10 rounded-xl flex items-center justify-center text-lg"
        style="background: var(--color-accent-soft); color: var(--color-accent);">
        &#x1F512;
      </div>
      <div>
        <h3 class="text-base font-semibold" style="color: var(--color-text-primary);">{title}</h3>
        {#if folderName}
          <p class="text-xs" style="color: var(--color-text-muted);">{folderName}</p>
        {/if}
      </div>
    </div>

    <div class="mb-4">
      <input
        type="password"
        bind:value={password}
        placeholder="Enter password"
        class="w-full px-3 py-2.5 rounded-xl text-sm border-none outline-none"
        style="background: var(--color-surface); color: var(--color-text-primary);"
        onkeydown={(e) => e.key === 'Enter' && handleSubmit()}
        autofocus
      />
      {#if error}
        <p class="text-xs mt-1.5 px-1" style="color: #dc2626;">{error}</p>
      {/if}
    </div>

    <div class="flex gap-2 justify-end">
      <button
        class="px-4 py-2 rounded-xl text-sm"
        style="background: var(--color-surface); color: var(--color-text-secondary);"
        onclick={oncancel}
      >
        Cancel
      </button>
      <button
        class="px-4 py-2 rounded-xl text-sm font-medium"
        style="background: var(--color-accent-soft); color: var(--color-accent);"
        onclick={handleSubmit}
        disabled={submitting}
      >
        {submitting ? 'Verifying...' : 'Unlock'}
      </button>
    </div>
  </div>
</div>
