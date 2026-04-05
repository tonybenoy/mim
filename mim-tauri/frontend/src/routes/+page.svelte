<script lang="ts">
  import PhotoGrid from '$lib/components/PhotoGrid.svelte';
  import PeopleView from '$lib/components/PeopleView.svelte';
  import SearchView from '$lib/components/SearchView.svelte';
  import AlbumsView from '$lib/components/AlbumsView.svelte';
  import ChatView from '$lib/components/ChatView.svelte';
  import { currentSection } from '$lib/stores/ui';
  import { fade } from 'svelte/transition';
</script>

{#if $currentSection === 'library'}
  <div in:fade={{ duration: 200 }}>
    <PhotoGrid />
  </div>
{:else if $currentSection === 'people'}
  <div in:fade={{ duration: 200 }}>
    <PeopleView />
  </div>
{:else if $currentSection === 'albums'}
  <div in:fade={{ duration: 200 }}>
    <AlbumsView />
  </div>
{:else if $currentSection === 'chat'}
  <div class="h-full" in:fade={{ duration: 200 }}>
    <ChatView />
  </div>
{:else if $currentSection === 'search'}
  <div in:fade={{ duration: 200 }}>
    <SearchView />
  </div>
{/if}
