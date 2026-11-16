<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'command' | 'query' | 'event' | 'entity';
    children?: Snippet;
  }

  let { variant = 'command', children }: Props = $props();
</script>

<span class="badge badge-{variant}">
  {#if children}
    {@render children()}
  {:else}
    {variant.toUpperCase()}
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    padding: 0.2rem 0.5rem;
    font-size: 0.7rem;
    font-weight: 700;
    font-family: var(--font-mono);
    border-radius: var(--radius-sm);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge-command {
    background: var(--accent-orange-light);
    color: var(--accent-orange);
    border: 1px solid rgba(250, 93, 41, 0.2);
  }

  .badge-query {
    background: var(--accent-blue-bg);
    color: var(--accent-blue-text);
    border: 1px solid var(--accent-blue-border);
  }

  .badge-event {
    background: var(--accent-emerald-bg);
    color: var(--accent-emerald-text);
    border: 1px solid var(--accent-emerald-border);
  }

  .badge-entity {
    background: var(--border-subtle);
    color: var(--text-secondary);
    border: 1px solid var(--border-light);
  }
</style>
