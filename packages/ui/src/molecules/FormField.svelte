<script lang="ts">
  interface Props {
    label: string;
    type?: string;
    placeholder?: string;
    value?: any;
    error?: string;
    required?: boolean;
    oninput?: (e: Event) => void;
  }

  let {
    label,
    type = 'text',
    placeholder = '',
    value = $bindable(''),
    error,
    required = false,
    oninput
  }: Props = $props();
</script>

<div class="form-field">
  <label class="field-label">
    {label}
    {#if required}<span class="required-star">*</span>{/if}
  </label>
  <input
    {type}
    {placeholder}
    {oninput}
    bind:value
    class="field-input {error ? 'has-error' : ''}"
  />
  {#if error}
    <span class="field-error">{error}</span>
  {/if}
</div>

<style>
  .form-field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-bottom: 1rem;
  }

  .field-label {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-title);
  }

  .required-star {
    color: var(--accent-rose);
    margin-left: 0.2rem;
  }

  .field-input {
    width: 100%;
    padding: 0.65rem 0.85rem;
    font-size: 0.9rem;
    font-family: var(--font-sans);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    background: var(--bg-card);
    color: var(--text-main);
    outline: none;
    transition: all 0.15s ease-in-out;
  }

  .field-input:focus {
    border-color: var(--accent-orange);
    box-shadow: 0 0 0 3px rgba(250, 93, 41, 0.12);
  }

  .field-input.has-error {
    border-color: var(--accent-rose);
  }

  .field-error {
    font-size: 0.75rem;
    color: var(--accent-rose);
  }
</style>
