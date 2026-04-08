<script lang="ts">
  import type { Station } from '../lib/types';

  export let station: Station;
  export let onPlay: (s: Station) => void = () => {};
  export let onFavourite: ((s: Station) => void) | null = null;
  export let isFav: boolean = false;

  let imgError = false;

  function handleImgError() { imgError = true; }
</script>

<div class="station-row" role="row">
  <div class="station-icon">
    {#if station.favicon && !imgError}
      <img src={station.favicon} alt="" on:error={handleImgError} loading="lazy" />
    {:else}
      <span class="station-icon-fallback">📻</span>
    {/if}
  </div>

  <div class="station-info">
    <span class="station-name">{station.name}</span>
    <div class="station-meta">
      {#if station.codec || station.bitrate}
        <span class="badge badge-codec">{station.codec}{station.bitrate ? ` ${station.bitrate}k` : ''}</span>
      {/if}
      {#if station.country}
        <span class="badge badge-country">{station.country}</span>
      {/if}
      {#each station.tags.slice(0, 2) as tag}
        <span class="badge badge-tag">{tag}</span>
      {/each}
    </div>
  </div>

  <button class="play-btn" on:click={() => onPlay(station)} title="Play {station.name}">
    ▶
  </button>
  {#if onFavourite}
    <button
      class="fav-btn"
      class:fav-active={isFav}
      on:click={() => onFavourite && onFavourite(station)}
      title={isFav ? 'Remove from presets' : 'Add to presets'}
    >★</button>
  {/if}
</div>

<style>
  .station-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    min-height: 44px;
    transition: background 0.1s;
  }
  .station-row:hover, .station-row:focus-within {
    background: var(--surface-elevated);
  }
  .station-icon {
    width: 28px;
    height: 28px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .station-icon img {
    width: 28px;
    height: 28px;
    object-fit: contain;
    border-radius: var(--radius);
    filter: grayscale(30%);
  }
  .station-icon-fallback { font-size: 18px; line-height: 1; opacity: 0.6; }
  .station-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .station-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    letter-spacing: 0.03em;
  }
  .station-meta {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .badge {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 2px;
    letter-spacing: 0.06em;
    white-space: nowrap;
  }
  .badge-codec   { background: var(--surface); color: var(--accent-dim); border: 1px solid var(--accent-dim); }
  .badge-country { background: var(--surface); color: var(--text-muted); border: 1px solid var(--border); }
  .badge-tag     { background: var(--surface); color: var(--text-muted); border: 1px solid var(--border); border-radius: 10px; }
  .play-btn {
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: var(--radius);
    background: none;
    border: 1px solid var(--accent-dim);
    color: var(--accent);
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s, border-color 0.12s;
    min-height: 44px;
    min-width: 44px;
  }
  .play-btn:hover { background: var(--accent); border-color: var(--accent); color: #000; }
  .fav-btn {
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: var(--radius);
    background: none;
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 15px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s, border-color 0.12s, color 0.12s;
    min-height: 44px;
    min-width: 44px;
  }
  .fav-btn:hover { border-color: var(--accent2); color: var(--accent2); }
  .fav-btn.fav-active { color: var(--accent2); border-color: var(--accent2); }
</style>
