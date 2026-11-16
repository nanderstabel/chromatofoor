<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost';
    disabled?: boolean;
    type?: 'button' | 'submit' | 'reset';
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
  }

  let {
    variant = 'primary',
    disabled = false,
    type = 'button',
    onclick,
    children
  }: Props = $props();
</script>

<button
  {type}
  {disabled}
  {onclick}
  class="btn btn-{variant}"
>
  {#if children}
    {@render children()}
  {/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.6rem 1.25rem;
    font-size: 0.9rem;
    font-weight: 600;
    border-radius: var(--radius-md);
    border: 1px solid transparent;
    transition: all 0.15s ease-in-out;
  }

  .btn-primary {
    background: var(--accent-orange);
    color: white;
    box-shadow: var(--shadow-button);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-orange-hover);
  }

  .btn-secondary {
    background: var(--bg-card);
    color: var(--text-main);
    border-color: var(--border-light);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-app);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
  }

  .btn-ghost:hover:not(:disabled) {
    background: var(--border-subtle);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
