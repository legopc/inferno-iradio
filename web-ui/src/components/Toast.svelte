<script lang="ts" context="module">
  import { writable } from 'svelte/store';

  interface ToastItem { id: number; type: 'success' | 'error' | 'info'; message: string; }
  const toasts = writable<ToastItem[]>([]);
  let nextId = 0;

  export function toast(type: ToastItem['type'], message: string) {
    const id = nextId++;
    toasts.update(t => [...t, { id, type, message }]);
    setTimeout(() => toasts.update(t => t.filter(x => x.id !== id)), 3500);
  }
</script>

<script lang="ts">
  import { fly } from 'svelte/transition';
</script>

<div class="toast-container">
  {#each $toasts as t (t.id)}
    <div class="toast toast-{t.type}" transition:fly={{ y: 20, duration: 200 }}>
      {t.message}
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 16px;
    right: 16px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    padding: 10px 16px;
    border-radius: var(--radius);
    font-size: 12px;
    letter-spacing: 0.05em;
    border: 1px solid var(--border);
    background: var(--surface2);
  }
  .toast-success { border-color: var(--vu-green); color: var(--vu-green); }
  .toast-error { border-color: var(--danger); color: var(--danger); }
  .toast-info { border-color: var(--accent2); color: var(--accent2); }
</style>
