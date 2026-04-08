<script lang="ts">
  import { players } from '../lib/stores';

  export let open: boolean = false;
  export let maxSlots: number = 4;
  export let onSelect: (slot: number) => void = () => {};
  export let onClose: () => void = () => {};

  $: occupiedSlots = new Set($players.map(p => p.slot));

  function pick(slot: number) {
    onSelect(slot);
    onClose();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  function onBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('slot-backdrop')) onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div
    class="slot-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="Select a slot"
    on:click={onBackdrop}
    on:keydown={onKeydown}
  >
    <div class="slot-dialog">
      <div class="slot-dialog-header">
        <span class="slot-dialog-title">SELECT SLOT</span>
        <button class="slot-close-btn" on:click={onClose} aria-label="Close">×</button>
      </div>
      <div class="slot-grid">
        {#each Array(maxSlots) as _, i}
          {@const slot = i + 1}
          {@const occupied = occupiedSlots.has(slot)}
          <button
            class="slot-btn"
            class:slot-occupied={occupied}
            class:slot-free={!occupied}
            on:click={() => pick(slot)}
          >
            <span class="slot-num">{slot}</span>
            <span class="slot-status">{occupied ? 'REPLACE' : 'FREE'}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .slot-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    z-index: 500;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .slot-dialog {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 20px;
    width: min(400px, 95vw);
    max-height: 80vh;
    overflow-y: auto;
  }
  .slot-dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }
  .slot-dialog-title {
    font-size: 12px;
    letter-spacing: 0.15em;
    color: var(--text);
    font-weight: 700;
  }
  .slot-close-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 20px;
    line-height: 1;
    padding: 0 4px;
    min-height: 44px;
    min-width: 44px;
  }
  .slot-close-btn:hover { color: var(--text); }
  .slot-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(80px, 1fr));
    gap: 8px;
  }
  .slot-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 8px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface2);
    min-height: 64px;
    transition: background 0.1s, border-color 0.1s;
  }
  .slot-free {
    border-color: var(--accent);
    color: var(--accent);
  }
  .slot-free:hover { background: color-mix(in srgb, var(--accent) 15%, transparent); }
  .slot-occupied {
    border-color: var(--warning);
    color: var(--warning);
    opacity: 0.85;
  }
  .slot-occupied:hover { background: color-mix(in srgb, var(--warning) 15%, transparent); }
  .slot-num {
    font-size: 22px;
    font-weight: 700;
    line-height: 1;
  }
  .slot-status {
    font-size: 9px;
    letter-spacing: 0.1em;
  }

  /* Mobile: bottom sheet */
  @media (max-width: 600px) {
    .slot-backdrop {
      align-items: flex-end;
    }
    .slot-dialog {
      width: 100%;
      border-radius: var(--radius) var(--radius) 0 0;
      max-height: 60vh;
    }
  }
</style>
