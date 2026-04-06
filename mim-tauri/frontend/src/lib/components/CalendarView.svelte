<script lang="ts">
  import { photos, activeFolder } from '$lib/stores/photos';
  import { viewMode, selectedPhotoId } from '$lib/stores/ui';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { fade, fly, scale } from 'svelte/transition';
  import type { Photo } from '$lib/api/photos';

  let selectedYear = $state(new Date().getFullYear());
  let selectedDate = $state<string | null>(null);

  // Group photos by date (YYYY-MM-DD)
  let photosByDay = $derived.by(() => {
    const map = new Map<string, Photo[]>();
    for (const photo of $photos) {
      const dateStr = photo.taken_at || photo.file_modified_at || photo.created_at;
      if (!dateStr) continue;
      const day = dateStr.slice(0, 10); // YYYY-MM-DD
      const list = map.get(day) || [];
      list.push(photo);
      map.set(day, list);
    }
    return map;
  });

  // Get all days in the selected year
  function getDaysInYear(year: number): { month: number; day: number; dateStr: string }[][] {
    const months: { month: number; day: number; dateStr: string }[][] = [];
    for (let m = 0; m < 12; m++) {
      const days: { month: number; day: number; dateStr: string }[] = [];
      const daysInMonth = new Date(year, m + 1, 0).getDate();
      for (let d = 1; d <= daysInMonth; d++) {
        const dateStr = `${year}-${String(m + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
        days.push({ month: m, day: d, dateStr });
      }
      months.push(days);
    }
    return months;
  }

  let yearGrid = $derived(getDaysInYear(selectedYear));

  function getHeatColor(count: number): string {
    if (count === 0) return 'var(--color-surface)';
    if (count <= 2) return 'rgba(var(--accent-rgb, 99, 102, 241), 0.2)';
    if (count <= 5) return 'rgba(var(--accent-rgb, 99, 102, 241), 0.4)';
    if (count <= 20) return 'rgba(var(--accent-rgb, 99, 102, 241), 0.6)';
    return 'rgba(var(--accent-rgb, 99, 102, 241), 0.85)';
  }

  function getHeatStyle(count: number): string {
    if (count === 0) return 'background: var(--color-surface); opacity: 0.5;';
    if (count <= 2) return 'background: var(--color-accent); opacity: 0.25;';
    if (count <= 5) return 'background: var(--color-accent); opacity: 0.45;';
    if (count <= 20) return 'background: var(--color-accent); opacity: 0.65;';
    return 'background: var(--color-accent); opacity: 0.9;';
  }

  function selectDay(dateStr: string) {
    const count = photosByDay.get(dateStr)?.length || 0;
    if (count > 0) {
      selectedDate = dateStr;
    }
  }

  function getSelectedPhotos(): Photo[] {
    if (!selectedDate) return [];
    return photosByDay.get(selectedDate) || [];
  }

  function getThumbnailSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    const prefix = photo.content_hash.slice(0, 2);
    return convertFileSrc(`${$activeFolder.path}/.mim/thumbnails/${prefix}/${photo.content_hash}_256.webp`);
  }

  function getFullSrc(photo: Photo): string {
    if (!$activeFolder) return '';
    return convertFileSrc(`${$activeFolder.path}/${photo.relative_path}`);
  }

  const monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  // Compute total photo count for the year
  let yearPhotoCount = $derived.by(() => {
    let count = 0;
    for (const [dateStr, photos] of photosByDay) {
      if (dateStr.startsWith(String(selectedYear))) {
        count += photos.length;
      }
    }
    return count;
  });
</script>

<div class="flex-1 overflow-y-auto p-4" style="padding-bottom: 80px;">
  {#if selectedDate}
    <!-- Day detail view -->
    <div in:fade={{ duration: 200 }}>
      <button
        class="flex items-center gap-2 mb-4 px-3 py-2 rounded-xl text-sm transition-all hover:opacity-80"
        style="color: var(--color-accent);"
        onclick={() => selectedDate = null}
      >
        &#x2190; Back to Calendar
      </button>

      <h2 class="text-lg font-semibold mb-1" style="color: var(--color-text-primary);">
        {new Date(selectedDate + 'T12:00:00').toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' })}
      </h2>
      <p class="text-sm mb-4" style="color: var(--color-text-muted);">
        {getSelectedPhotos().length} photos
      </p>

      <div class="grid gap-1.5" style="grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));">
        {#each getSelectedPhotos() as photo, i (photo.id)}
          <button
            class="relative aspect-square overflow-hidden rounded-xl group cursor-pointer"
            style="box-shadow: var(--shadow-neu-soft);"
            in:scale={{ start: 0.9, duration: 300, delay: Math.min(i * 20, 500) }}
            onclick={() => selectedPhotoId.set(photo.id)}
          >
            <img src={getThumbnailSrc(photo)} alt={photo.filename}
              class="w-full h-full object-cover" loading="lazy"
              onerror={(e) => { (e.target as HTMLImageElement).src = getFullSrc(photo); }} />
            <div class="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-transparent
              opacity-0 group-hover:opacity-100 transition-opacity rounded-xl">
              <div class="absolute bottom-2 left-2 right-2">
                <p class="text-white text-[10px] truncate">{photo.filename}</p>
              </div>
            </div>
          </button>
        {/each}
      </div>
    </div>

  {:else}
    <!-- Calendar heat map -->
    <div class="flex items-center justify-between mb-6" in:fade={{ duration: 200 }}>
      <div class="flex items-center gap-3">
        <h2 class="text-lg font-semibold" style="color: var(--color-text-primary);">
          {selectedYear}
        </h2>
        <span class="text-xs px-2 py-0.5 rounded-full"
          style="background: var(--color-accent-soft); color: var(--color-accent);">
          {yearPhotoCount} photos
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="w-8 h-8 rounded-lg flex items-center justify-center text-sm transition-all hover:scale-110"
          style="background: var(--color-surface); color: var(--color-text-secondary);"
          onclick={() => selectedYear--}
        >
          &#x2039;
        </button>
        <button
          class="px-3 py-1.5 rounded-lg text-xs font-medium"
          style="background: var(--color-surface); color: var(--color-text-secondary);"
          onclick={() => selectedYear = new Date().getFullYear()}
        >
          Today
        </button>
        <button
          class="w-8 h-8 rounded-lg flex items-center justify-center text-sm transition-all hover:scale-110"
          style="background: var(--color-surface); color: var(--color-text-secondary);"
          onclick={() => selectedYear++}
        >
          &#x203A;
        </button>
      </div>
    </div>

    <!-- Legend -->
    <div class="flex items-center gap-3 mb-4">
      <span class="text-[10px]" style="color: var(--color-text-muted);">Less</span>
      <div class="flex gap-1">
        <div class="w-3 h-3 rounded-sm" style="background: var(--color-surface); opacity: 0.5;"></div>
        <div class="w-3 h-3 rounded-sm" style="background: var(--color-accent); opacity: 0.25;"></div>
        <div class="w-3 h-3 rounded-sm" style="background: var(--color-accent); opacity: 0.45;"></div>
        <div class="w-3 h-3 rounded-sm" style="background: var(--color-accent); opacity: 0.65;"></div>
        <div class="w-3 h-3 rounded-sm" style="background: var(--color-accent); opacity: 0.9;"></div>
      </div>
      <span class="text-[10px]" style="color: var(--color-text-muted);">More</span>
    </div>

    <!-- Month grids -->
    <div class="grid gap-6" style="grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));">
      {#each yearGrid as monthDays, monthIdx}
        <div
          class="rounded-xl p-3"
          style="background: var(--color-surface); box-shadow: var(--shadow-neu-soft);"
          in:fly={{ y: 20, duration: 300, delay: monthIdx * 40 }}
        >
          <div class="text-xs font-semibold mb-2" style="color: var(--color-text-primary);">
            {monthNames[monthIdx]}
          </div>

          <!-- Day of week headers -->
          <div class="grid grid-cols-7 gap-[2px] mb-1">
            {#each ['S', 'M', 'T', 'W', 'T', 'F', 'S'] as dow}
              <div class="text-[8px] text-center" style="color: var(--color-text-muted);">{dow}</div>
            {/each}
          </div>

          <!-- Day cells -->
          <div class="grid grid-cols-7 gap-[2px]">
            <!-- Leading empty cells for first day offset -->
            {#each Array(new Date(selectedYear, monthIdx, 1).getDay()) as _}
              <div class="w-full aspect-square"></div>
            {/each}
            {#each monthDays as dayInfo}
              {@const count = photosByDay.get(dayInfo.dateStr)?.length || 0}
              <button
                class="w-full aspect-square rounded-sm transition-all duration-150 relative group"
                style={getHeatStyle(count)}
                title="{dayInfo.dateStr}: {count} photos"
                onclick={() => selectDay(dayInfo.dateStr)}
                disabled={count === 0}
              >
                {#if count > 0}
                  <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                    <span class="text-[7px] font-bold" style="color: var(--color-text-primary);">{count}</span>
                  </div>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
