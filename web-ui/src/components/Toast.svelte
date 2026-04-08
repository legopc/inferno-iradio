<script lang="ts" context="module">
  import { writable } from 'svelte/store';

  interface ToastItem { id: number; type: 'success' | 'error' | 'info'; message: string; }
  const toasts = writable<ToastItem[]>([]);
  let nextId = 0;

  export function toast(type: ToastItem['type'], message: string) {
    const id = nextId++;
    toasts.update(t => {
      const next = [...t, { id, type, message }];
      return next.slice(-3); // keep at most 3, oldest drops off
    });
    setTimeout(() => toasts.update(t => t.filter(x => x.id !== id)), 4000);
  }
</script>

<script lang="ts">
  import { fly } from 'svelte/transition';
</script>

<div class="toast-container" aria-live="polite">
  {#each $toasts as t (t.id)}
    <div class="toast toast-{t.type}" transition:fly={{ x: 120, duration: 220 }}>
      <span class="toast-icon">
        {#if t.type === 'success'}✔{:else if t.type === 'error'}✖{:else}ℹ{/if}
      </span>
      <span class="toast-msg">{t.message}</span>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-radius: var(--radius);
    font-size: 12px;
    letter-spacing: 0.05em;
    border: 1px solid var(--border);
    background: var(--surface-elevated);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    min-width: 220px;
    max-width: 340px;
  }
  .toast-icon { flex: none; font-size: 14px; }
  .toast-msg { flex: 1; }
  .toast-success { border-color: var(--vu-green); color: var(--vu-green); }
  .toast-error   { border-color: var(--error);     color: var(--error); }
  .toast-info    { border-color: var(--warning);   color: var(--warning); }
</style>
