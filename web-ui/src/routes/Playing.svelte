<script lang="ts">
  import { players, maxSlots } from '../lib/stores';
  import ChannelStrip from '../components/ChannelStrip.svelte';
  import SlotPicker from '../components/SlotPicker.svelte';
  import { onMount } from 'svelte';

  let showSlotPicker = false;
  let selectedSlot: number | null = null;

  $: playersArray = $players;

  function openAddStationDialog() {
    showSlotPicker = true;
  }

  function handleSlotSelected(e: CustomEvent<number>) {
    selectedSlot = e.detail;
    showSlotPicker = false;
    // Navigate to Search tab with preselected slot
    // This would typically trigger a parent component to show Search with preselectedSlot
    window.dispatchEvent(new CustomEvent('navigate-to-search', { detail: { slot: selectedSlot } }));
  }

  function closeSlotPicker() {
    showSlotPicker = false;
  }
</script>

<div class="container">
  {#if playersArray.length === 0}
    <div class="empty-state">
      <div class="empty-message">
        <span class="boot-cursor">▮</span> NO STREAMS PLAYING — GO TO SEARCH TO ADD ONE
      </div>
      <button class="btn-primary" on:click={openAddStationDialog}>
        Add Station
      </button>
    </div>
  {:else}
    <div class="header">
      <h2>Now Playing</h2>
      <button class="btn-secondary" on:click={openAddStationDialog}>
        + Add Station
      </button>
    </div>
    <div class="channel-grid">
      {#each playersArray as player (player.id)}
        <ChannelStrip {player} />
      {/each}
    </div>
  {/if}

  {#if showSlotPicker}
    <div class="modal-overlay" on:click={closeSlotPicker}>
      <div class="modal-content" on:click|stopPropagation>
        <h3>Select Slot</h3>
        <SlotPicker
          on:slot-selected={handleSlotSelected}
          max={$maxSlots}
          availableSlots={playersArray.map(p => p.slot)}
        />
        <button class="btn-secondary" on:click={closeSlotPicker}>Cancel</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 24px;
    padding: 64px 32px;
    text-align: center;
  }

  .empty-message {
    color: var(--text-dim);
    font-size: 1.1em;
  }

  .boot-cursor {
    color: var(--accent);
    animation: blink 1s step-end infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0;
    }
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .header h2 {
    margin: 0;
    font-size: 1.3em;
    color: var(--text);
  }

  .channel-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal-content {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 24px;
    max-width: 400px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .modal-content h3 {
    margin: 0 0 16px 0;
    color: var(--text);
  }

  .btn-primary,
  .btn-secondary {
    padding: 8px 16px;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn-primary {
    background: var(--accent);
    color: var(--surface);
  }

  .btn-primary:hover {
    opacity: 0.9;
  }

  .btn-secondary {
    background: var(--surface2);
    color: var(--text);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover {
    background: var(--surface3);
  }

  @media (max-width: 768px) {
    .channel-grid {
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    }
  }

  @media (max-width: 480px) {
    .channel-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
