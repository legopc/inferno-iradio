<script lang="ts">
  import type { Station } from '../lib/types';
  import { players, maxSlots, apiOnline } from '../lib/stores';
  import { api } from '../lib/api';
  import { toast } from '../components/Toast.svelte';
  import ChannelStrip from '../components/ChannelStrip.svelte';
  import StationRow from '../components/StationRow.svelte';
  import SlotPicker from '../components/SlotPicker.svelte';
  import { onMount } from 'svelte';

  let favs: Station[] = [];
  let presetsOpen = true;
  let pendingStation: Station | null = null;
  let slotPickerOpen = false;

  async function loadFavs() {
    try { favs = await api.getFavourites() as Station[]; } catch {}
  }

  onMount(() => { if ($apiOnline) loadFavs(); });
  $: if ($apiOnline && favs.length === 0) loadFavs();

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
      // Stop any existing player in this slot first
      const existing = $players.find(p => p.slot === slot);
      if (existing) {
        await api.deletePlayer(existing.id).catch(() => {});
        players.update(list => list.filter(p => p.id !== existing.id));
      }
      const newPlayer = await api.createPlayer(s.url, s.name, slot);
      players.update(list => {
        const idx = list.findIndex(p => p.id === newPlayer.id);
        if (idx >= 0) { const copy = [...list]; copy[idx] = newPlayer; return copy; }
        return [...list, newPlayer];
      });
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

  function handleSlotClose() { slotPickerOpen = false; pendingStation = null; }
</script>

<div class="playing-view">
  <div class="channel-grid">
    {#each Array($maxSlots) as _, i}
      {@const slot = i + 1}
      {@const player = $players.find(p => p.slot === slot) ?? null}
      {#if player}
        <ChannelStrip {player} />
      {:else}
        <div class="empty-slot">
          <span class="empty-slot-num">S{slot}</span>
          <span class="empty-slot-label">EMPTY</span>
        </div>
      {/if}
    {/each}
  </div>

  {#if $players.length === 0}
    <div class="boot-hint">
      <span class="boot-cursor">▮</span>
      SCAN STATIONS TO TUNE IN
    </div>
  {/if}

  <div class="slot-info">
    {$players.length} / {$maxSlots} SLOTS ACTIVE
  </div>

  <!-- Presets panel -->
  <div class="presets-panel">
    <button class="presets-header" on:click={() => presetsOpen = !presetsOpen}>
      <span class="presets-title">★ PRESETS</span>
      <span class="presets-count">{favs.length}</span>
      <span class="presets-chevron">{presetsOpen ? '▲' : '▼'}</span>
    </button>

    {#if presetsOpen}
      <div class="presets-body">
        {#if favs.length === 0}
          <div class="presets-empty">No presets saved — star stations in the Search tab</div>
        {:else}
          {#each favs as station (station.id)}
            <div class="preset-item">
              <StationRow {station} onPlay={handlePlay} />
              <button class="preset-remove" on:click={() => removeFav(station)} title="Remove preset">★</button>
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</div>

<SlotPicker
  open={slotPickerOpen}
  maxSlots={$maxSlots}
  onSelect={handleSlotSelect}
  onClose={handleSlotClose}
/>

<style>
  .playing-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
  }
  .channel-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--strip-width), 1fr));
    gap: 10px;
    align-items: flex-start;
  }
  @media (max-width: 600px) {
    .channel-grid { grid-template-columns: 1fr; }
  }
  .boot-hint {
    color: var(--text-dim);
    padding: 8px 16px;
    text-align: center;
    letter-spacing: 0.1em;
    font-size: 11px;
  }
  .boot-cursor {
    color: var(--accent);
    animation: blink 1s step-end infinite;
    margin-right: 8px;
  }
  @keyframes blink { 50% { opacity: 0; } }
  .empty-slot {
    border: 1px dashed var(--border);
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-height: 180px;
    opacity: 0.35;
  }
  .empty-slot-num {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-muted);
  }
  .empty-slot-label {
    font-size: 9px;
    letter-spacing: 0.15em;
    color: var(--text-muted);
  }
  .slot-info {
    font-size: 10px;
    color: var(--text-muted);
    letter-spacing: 0.1em;
    text-align: right;
  }

  /* Presets panel */
  .presets-panel {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    margin-top: 4px;
  }
  .presets-header {
    width: 100%;
    background: var(--surface-elevated);
    border: none;
    padding: 10px 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    color: var(--text);
    font-family: inherit;
    font-size: 11px;
    letter-spacing: 0.1em;
    text-align: left;
    transition: background 0.1s;
  }
  .presets-header:hover { background: var(--surface); }
  .presets-title { color: var(--accent2); font-weight: 600; }
  .presets-count {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0 6px;
    font-size: 10px;
    color: var(--text-muted);
  }
  .presets-chevron { margin-left: auto; color: var(--text-muted); font-size: 10px; }
  .presets-body { border-top: 1px solid var(--border); }
  .presets-empty {
    padding: 16px 14px;
    font-size: 11px;
    color: var(--text-muted);
    letter-spacing: 0.05em;
    text-align: center;
  }
  .preset-item { display: flex; align-items: center; }
  .preset-item :global(.station-row) { flex: 1; }
  .preset-remove {
    flex: none;
    background: none;
    border: none;
    color: var(--accent2);
    font-size: 15px;
    padding: 0 12px;
    min-height: 44px;
    min-width: 44px;
    cursor: pointer;
    transition: color 0.1s;
  }
  .preset-remove:hover { color: var(--danger); }
</style>
