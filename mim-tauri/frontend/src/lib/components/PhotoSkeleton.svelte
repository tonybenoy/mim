<script lang="ts">
  let { delay = 0 }: { delay?: number } = $props();
</script>

<div
  class="aspect-square rounded-xl animate-shimmer"
  style="animation-delay: {delay}ms;"
></div>
