<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';

  export let onSearch: (q: string, tag?: string, country?: string) => void = () => {};

  let query = '';
  let selectedTag = '';
  let selectedCountry = '';
  let debounceTimer: ReturnType<typeof setTimeout>;

  let tags: string[] = [];
  let countries: string[] = [];

  // Hardcoded fallbacks
  const FALLBACK_TAGS = ['music', 'talk', 'news', 'pop', 'rock', 'jazz', 'classical',
    'hip-hop', 'electronic', 'country', 'sport', 'comedy'];
  const FALLBACK_COUNTRIES = ['US', 'UK', 'DE', 'FR', 'ES', 'IT', 'BR', 'CA', 'AU', 'NL', 'PL', 'RU'];

  onMount(async () => {
    try {
      const t = await api.tags();
      tags = Array.isArray(t) ? t.map((x: unknown) => (typeof x === 'string' ? x : (x as {name:string}).name)).filter(Boolean).slice(0, 60) : FALLBACK_TAGS;
    } catch { tags = FALLBACK_TAGS; }
    try {
      const c = await api.countries();
      countries = Array.isArray(c) ? c.map((x: unknown) => (typeof x === 'string' ? x : (x as {name:string}).name)).filter(Boolean).slice(0, 80) : FALLBACK_COUNTRIES;
    } catch { countries = FALLBACK_COUNTRIES; }
  });

  function submit() {
    clearTimeout(debounceTimer);
    onSearch(query.trim(), selectedTag || undefined, selectedCountry || undefined);
  }

  function onInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(submit, 300);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') submit();
    if (e.key === 'Escape') clear();
  }

  function clear() {
    query = '';
    selectedTag = '';
    selectedCountry = '';
    clearTimeout(debounceTimer);
    onSearch('');
  }

  function onFilterChange() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(submit, 100);
  }
</script>

<div class="search-bar">
  <div class="search-input-wrap">
    <span class="search-icon">🔍</span>
    <input
      type="text"
      class="search-input"
      bind:value={query}
      on:input={onInput}
      on:keydown={onKeydown}
      placeholder="SEARCH STATIONS…"
      aria-label="Search stations"
    />
    {#if query}
      <button class="clear-btn" on:click={clear} aria-label="Clear search">×</button>
    {/if}
  </div>

  <div class="filter-row">
    <select class="filter-select" bind:value={selectedTag} on:change={onFilterChange} aria-label="Filter by tag">
      <option value="">ALL GENRES</option>
      {#each tags as tag}
        <option value={tag}>{tag.toUpperCase()}</option>
      {/each}
    </select>

    <select class="filter-select" bind:value={selectedCountry} on:change={onFilterChange} aria-label="Filter by country">
      <option value="">ALL COUNTRIES</option>
      {#each countries as country}
        <option value={country}>{country}</option>
      {/each}
    </select>
  </div>
</div>

<style>
  .search-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
  }
  .search-input-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0 10px;
    transition: border-color 0.15s;
  }
  .search-input-wrap:focus-within { border-color: var(--accent); }
  .search-icon { flex: none; color: var(--text-dim); font-size: 14px; }
  .search-input {
    flex: 1;
    background: none;
    border: none;
    color: var(--text);
    font-size: 13px;
    padding: 10px 0;
    outline: none;
    letter-spacing: 0.05em;
    min-height: 44px;
  }
  .search-input::placeholder { color: var(--text-muted); }
  .clear-btn {
    flex: none;
    background: none;
    border: none;
    color: var(--text-dim);
    font-size: 18px;
    padding: 0 4px;
    line-height: 1;
    min-width: 28px;
    min-height: 44px;
  }
  .clear-btn:hover { color: var(--text); }
  .filter-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .filter-select {
    flex: 1;
    min-width: 120px;
    background: var(--surface2);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 11px;
    padding: 6px 8px;
    border-radius: var(--radius);
    letter-spacing: 0.05em;
    cursor: pointer;
    min-height: 36px;
  }
  .filter-select:focus { border-color: var(--accent); outline: none; }
</style>
