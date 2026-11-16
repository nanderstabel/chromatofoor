<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    leading?: Snippet;
    center?: Snippet;
    trailing?: Snippet;
    onDragStart?: (e: MouseEvent) => void;
  }

  let { leading, center, trailing, onDragStart }: Props = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
  onmousedown={onDragStart}
  class="shrink-0 h-14 bg-surface-container-lowest/95 dark:bg-petroleum-surface/95 backdrop-blur-md border-b border-contour-line-light dark:border-contour-line-dark px-4 lg:px-6 flex items-center justify-between gap-4 select-none"
>
  <div class="flex items-center gap-3 shrink-0">{@render leading?.()}</div>
  {#if center}<div class="flex items-center">{@render center()}</div>{/if}
  <div class="flex items-center gap-2.5 shrink-0">{@render trailing?.()}</div>
</header>