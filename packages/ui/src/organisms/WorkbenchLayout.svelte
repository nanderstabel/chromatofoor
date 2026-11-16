<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    brandName?: string;
    filterTerm?: string;
    onSearch?: (term: string) => void;
    sidebar?: Snippet;
    children?: Snippet;
  }

  let {
    brandName = 'Chromatofoor',
    filterTerm = $bindable(''),
    sidebar,
    children
  }: Props = $props();
</script>

<div class="app-layout">
  <aside>
    <div class="brand-header">
      <div class="brand-logo">C</div>
      <span class="brand-title">{brandName}</span>
      <div class="brand-status">
        <span class="status-dot"></span>
        Connected
      </div>
    </div>
    <div class="search-box">
      <input
        type="text"
        class="search-input"
        placeholder="🔍 Search domain primitives..."
        bind:value={filterTerm}
      />
    </div>
    <div class="nav-container">
      {#if sidebar}
        {@render sidebar()}
      {/if}
    </div>
  </aside>
  <main>
    {#if children}
      {@render children()}
    {/if}
  </main>
</div>

<style>
  .app-layout {
    display: flex;
    min-height: 100vh;
    background-color: var(--bg-app);
  }

  aside {
    width: 280px;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border-light);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .brand-header {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .brand-logo {
    width: 36px;
    height: 36px;
    background: linear-gradient(135deg, var(--accent-orange), var(--accent-orange-hover));
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-brand);
    font-weight: 800;
    font-size: 1.25rem;
    color: white;
    box-shadow: var(--shadow-button);
  }

  .brand-title {
    font-family: var(--font-brand);
    font-weight: 800;
    font-size: 1.1rem;
    color: var(--text-title);

  }

  .brand-status {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--accent-emerald-text);
    background: var(--accent-emerald-bg);
    padding: 0.25rem 0.6rem;
    border-radius: 20px;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-emerald-text);
  }

  .search-box {
    padding: 1rem 1.25rem;
  }

  .search-input {
    width: 100%;
    padding: 0.6rem 0.85rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-light);
    background: var(--bg-app);
    font-size: 0.85rem;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .search-input:focus {
    border-color: var(--accent-orange);
  }

  .nav-container {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem 1rem;
  }

  main {
    flex: 1;
    padding: 2rem;
    overflow-y: auto;
  }
</style>
