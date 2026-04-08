<script lang="ts">
  import { api } from '../lib/api';
  import { maxSlots } from '../lib/stores';
  import StationRow from '../components/StationRow.svelte';
  import SearchBar from '../components/SearchBar.svelte';
  import SlotPicker from '../components/SlotPicker.svelte';
  import type { Station } from '../lib/types';
  import { onMount } from 'svelte';

  export let preselectedSlot: number | undefined = undefined;

  let results: Station[] = [];
  let loading = false;
  let error: string | null = null;
  let searchQuery = '';
  let showSlotPicker = false;
  let selectedStation: Station | null = null;

  onMount(async () => {
    await loadTopStations();
  });

  async function loadTopStations() {
    loading = true;
    error = null;
    try {
      results = await api.topStations(20);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load stations';
      dispatchToast('error', error);
    } finally {
      loading = false;
    }
  }

  async function handleSearch() {
    if (!searchQuery.trim()) {
      await loadTopStations();
      return;
    }

    loading = true;
    error = null;
    try {
      results = await api.searchStations(searchQuery, 40);
      if (results.length === 0) {
        error = 'No results found';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Search failed';
      dispatchToast('error', error);
    } finally {
      loading = false;
    }
  }

  function handleStationPlay(station: Station) {
    selectedStation = station;
    if (preselectedSlot !== undefined) {
      playStation(station, preselectedSlot);
    } else {
      showSlotPicker = true;
    }
  }

  async function playStation(station: Station, slot: number) {
    try {
      await api.createPlayer(station.url, station.name, slot);
      dispatchToast('success', `Now playing: ${station.name}`);
      showSlotPicker = false;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to play station';
      error = message;
      dispatchToast('error', message);
    }
  }

  function handleSlotSelected(e: CustomEvent<number>) {
    if (selectedStation) {
      playStation(selectedStation, e.detail);
    }
  }

  function closeSlotPicker() {
    showSlotPicker = false;
    selectedStation = null;
  }

  function dispatchToast(type: 'success' | 'error' | 'info', message: string) {
    window.dispatchEvent(
      new CustomEvent('show-toast', {
        detail: { type, message }
      })
    );
  }
</script>

<div class="container">
  <SearchBar
    bind:value={searchQuery}
    placeholder="SEARCH STATIONS…"
    on:keydown={(e) => {
      if (e.key === 'Enter') handleSearch();
    }}
  />
  <button class="btn-search" on:click={handleSearch} disabled={loading}>
    {loading ? 'Searching…' : 'Search'}
  </button>

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <p>Loading stations…</p>
    </div>
  {:else if error && results.length === 0}
    <div class="empty-state">
      <p class="error-message">{error}</p>
      <button class="btn-secondary" on:click={loadTopStations}>Load Top Stations</button>
    </div>
  {:else if results.length === 0}
    <div class="empty-state">
      <p class="empty-message">No results found</p>
    </div>
  {:else}
    <div class="results-header">
      <p class="results-count">{results.length} stations found</p>
    </div>
    <div class="results-list">
      {#each results as station (station.id)}
        <div class="station-item">
          <StationRow {station} />
          <button
            class="btn-play"
            on:click={() => handleStationPlay(station)}
            title="Play this station"
          >
            ▶
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if showSlotPicker && selectedStation}
    <div class="modal-overlay" on:click={closeSlotPicker}>
      <div class="modal-content" on:click|stopPropagation>
        <h3>Select Slot for "{selectedStation.name}"</h3>
        <SlotPicker on:slot-selected={handleSlotSelected} max={$maxSlots} />
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

  .btn-search {
    padding: 8px 16px;
    background: var(--accent);
    color: var(--surface);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn-search:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-search:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 64px 32px;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .loading p {
    color: var(--text-dim);
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

  .error-message {
    color: var(--error, #ff6b6b);
    font-size: 1em;
  }

  .results-header {
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .results-count {
    color: var(--text-dim);
    font-size: 0.9em;
    margin: 0;
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 600px;
    overflow-y: auto;
  }

  .station-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: background 0.2s;
  }

  .station-item:hover {
    background: var(--surface2);
  }

  .station-item > :global(*:first-child) {
    flex: 1;
  }

  .btn-play {
    padding: 6px 12px;
    background: var(--accent);
    color: var(--surface);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .btn-play:hover {
    opacity: 0.9;
  }

  .btn-secondary {
    padding: 8px 16px;
    background: var(--surface2);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
  }

  .btn-secondary:hover {
    background: var(--surface3);
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

  @media (max-width: 768px) {
    .results-list {
      max-height: 400px;
    }
  }
</style>
