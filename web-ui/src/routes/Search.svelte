<script lang="ts">
  import type { Station } from '../lib/types';
  import { maxSlots, players, apiOnline } from '../lib/stores';
  import { api } from '../lib/api';
  import { toast } from '../components/Toast.svelte';
  import SearchBar from '../components/SearchBar.svelte';
  import StationRow from '../components/StationRow.svelte';
  import SlotPicker from '../components/SlotPicker.svelte';
  import { onMount } from 'svelte';

  let results: Station[] = [];
  let loading = false;
  let pendingStation: Station | null = null;
  let slotPickerOpen = false;
  let favIds = new Set<string>();

  async function loadFavIds() {
    try {
      const favs = await api.getFavourites() as Station[];
      favIds = new Set(favs.map(f => f.id));
    } catch {}
  }

  // Load top stations initially — retry once if first attempt fails (radio-browser init)
  async function loadTop() {
    loading = true;
    try {
      results = await api.topStations(40) as Station[];
    } catch {
      // Retry once after short delay (radio-browser client may still be resolving)
      await new Promise(r => setTimeout(r, 1500));
      try {
        results = await api.topStations(40) as Station[];
      } catch (err: unknown) {
        toast('error', err instanceof Error ? err.message : 'Load failed');
      }
    } finally {
      loading = false;
    }
  }

  // Load on mount (not at module eval time) so API is ready
  onMount(() => { if ($apiOnline) { loadTop(); loadFavIds(); } });
  // Re-load when API comes online
  $: if ($apiOnline && results.length === 0 && !loading) { loadTop(); loadFavIds(); }

  async function handleSearch(q: string, tag?: string, country?: string) {
    loading = true;
    results = [];
    try {
      if (tag)     results = await api.byTag(tag)         as Station[];
      else if (country) results = await api.byCountry(country) as Station[];
      else if (q)  results = await api.searchStations(q)  as Station[];
      else         results = await api.topStations(40)    as Station[];
    } catch (err: unknown) {
      toast('error', err instanceof Error ? err.message : 'Search failed');
    } finally {
      loading = false;
    }
  }

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
      // Update store immediately — don't wait for WS player_update event
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

  function handleSlotClose() {
    slotPickerOpen = false;
    pendingStation = null;
  }

  async function handleFavourite(station: Station) {
    if (favIds.has(station.id)) {
      try {
        await api.removeFavourite(station.id);
        favIds = new Set([...favIds].filter(id => id !== station.id));
        toast('info', `Removed: ${station.name}`);
      } catch (err: unknown) {
        toast('error', err instanceof Error ? err.message : 'Failed to remove');
      }
    } else {
      try {
        await api.addFavourite(station);
        favIds = new Set([...favIds, station.id]);
        toast('success', `Saved: ${station.name}`);
      } catch (err: unknown) {
        toast('error', err instanceof Error ? err.message : 'Failed to save');
      }
    }
  }
</script>

<div class="search-view">
  <SearchBar onSearch={handleSearch} />

  {#if loading}
    <div class="loading">SCANNING…</div>
  {:else if results.length === 0}
    <div class="empty">NO STATIONS FOUND</div>
  {:else}
    <div class="results-count">{results.length} STATIONS</div>
    <div class="results-list">
      {#each results as station (station.id)}
        <StationRow {station} onPlay={handlePlay} onFavourite={handleFavourite} isFav={favIds.has(station.id)} />
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
  .search-view {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .loading, .empty {
    color: var(--text-dim);
    padding: 32px;
    text-align: center;
    letter-spacing: 0.1em;
    font-size: 12px;
  }
  .results-count {
    font-size: 10px;
    color: var(--text-muted);
    letter-spacing: 0.1em;
    text-align: right;
  }
  .results-list {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
</style>
