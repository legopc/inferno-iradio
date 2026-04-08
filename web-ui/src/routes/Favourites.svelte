<script lang="ts">
  import type { Station } from '../lib/types';
  import { maxSlots } from '../lib/stores';
  import { api } from '../lib/api';
  import { toast } from '../components/Toast.svelte';
  import StationRow from '../components/StationRow.svelte';
  import SlotPicker from '../components/SlotPicker.svelte';

  let favs: Station[] = [];
  let loading = true;
  let pendingStation: Station | null = null;
  let slotPickerOpen = false;

  async function load() {
    loading = true;
    try {
      favs = await api.getFavourites() as Station[];
    } catch (err: unknown) {
      toast('error', err instanceof Error ? err.message : 'Failed to load presets');
    } finally {
      loading = false;
    }
  }

  load();

  function handlePlay(station: Station) {
    pendingStation = station;
    slotPickerOpen = true;
  }

  async function handleSlotSelect(slot: number) {
    if (!pendingStation) return;
    const s = pendingStation;
    slotPickerOpen = false;
    pendingStation = null;
    try {
      await api.createPlayer(s.url, s.name, slot);
      toast('success', `Playing: ${s.name}`);
    } catch (err: unknown) {
      toast('error', err instanceof Error ? err.message : 'Failed to start player');
    }
  }

  async function removeFav(station: Station) {
    try {
      await api.removeFavourite(station.id);
      favs = favs.filter(f => f.id !== station.id);
      toast('info', `Removed: ${station.name}`);
    } catch (err: unknown) {
      toast('error', err instanceof Error ? err.message : 'Failed to remove');
    }
  }

  function handleSlotClose() {
    slotPickerOpen = false;
    pendingStation = null;
  }
</script>

<div class="favs-view">
  {#if loading}
    <div class="loading">LOADING PRESETS…</div>
  {:else if favs.length === 0}
    <div class="empty">
      <div class="empty-icon">★</div>
      <div>NO SAVED PRESETS</div>
      <div class="empty-sub">Search for stations and add them to presets</div>
    </div>
  {:else}
    <div class="favs-count">{favs.length} PRESET{favs.length !== 1 ? 'S' : ''}</div>
    <div class="favs-list">
      {#each favs as station (station.id)}
        <div class="fav-item">
          <StationRow {station} onPlay={handlePlay} />
          <button
            class="remove-btn"
            on:click={() => removeFav(station)}
            title="Remove from presets"
            aria-label="Remove {station.name} from presets"
          >★</button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<SlotPicker
  open={slotPickerOpen}
  maxSlots={$maxSlots}
  onSelect={handleSlotSelect}
  onClose={handleSlotClose}
/>

<style>
  .favs-view {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .loading {
    color: var(--text-dim);
    padding: 32px;
    text-align: center;
    letter-spacing: 0.1em;
    font-size: 12px;
  }
  .empty {
    color: var(--text-dim);
    padding: 48px 32px;
    text-align: center;
    letter-spacing: 0.1em;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .empty-icon { font-size: 32px; color: var(--text-muted); }
  .empty-sub { font-size: 11px; color: var(--text-muted); }
  .favs-count {
    font-size: 10px;
    color: var(--text-muted);
    letter-spacing: 0.1em;
    text-align: right;
  }
  .favs-list {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .fav-item {
    display: flex;
    align-items: center;
  }
  .fav-item :global(.station-row) { flex: 1; }
  .remove-btn {
    flex: none;
    background: none;
    border: none;
    color: var(--accent2);
    font-size: 16px;
    padding: 0 12px;
    min-height: 44px;
    min-width: 44px;
    transition: color 0.1s;
  }
  .remove-btn:hover { color: var(--danger); }
</style>
