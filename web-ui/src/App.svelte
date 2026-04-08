<script lang="ts">
  import { onMount } from 'svelte';
  import { connectWs } from './lib/ws';
  import { api } from './lib/api';
  import { players, maxSlots, defaultVolume, apiOnline, apiVersion } from './lib/stores';
  import Header from './components/Header.svelte';
  import Toast from './components/Toast.svelte';

  let activeTab: 'playing' | 'search' | 'favourites' = 'playing';

  // Lazy-loaded route components
  let PlayingRoute: any = null;
  let SearchRoute: any = null;
  let FavouritesRoute: any = null;

  onMount(async () => {
    connectWs();
    try {
      const health = await api.health();
      apiVersion.set(health.version || '2.0');
      maxSlots.set(health.max_players || 4);
      apiOnline.set(true);
    } catch {}
    try {
      const vol = await api.getVolume();
      if (typeof vol.volume === 'number') defaultVolume.set(vol.volume);
    } catch {}
    try {
      const list = await api.listPlayers();
      players.set(list);
    } catch {}

    // Lazy load routes
    PlayingRoute = (await import('./routes/Playing.svelte')).default;
    SearchRoute = (await import('./routes/Search.svelte')).default;
    FavouritesRoute = (await import('./routes/Favourites.svelte')).default;
  });

  function switchTab(tab: typeof activeTab) { activeTab = tab; }
</script>

<Toast />
<div class="app-shell">
  <Header on:tabChange={(e) => switchTab(e.detail)} {activeTab} />
  <main class="content-area">
    {#if activeTab === 'playing' && PlayingRoute}
      <svelte:component this={PlayingRoute} />
    {:else if activeTab === 'search' && SearchRoute}
      <svelte:component this={SearchRoute} />
    {:else if activeTab === 'favourites' && FavouritesRoute}
      <svelte:component this={FavouritesRoute} />
    {:else}
      <div class="loading-placeholder">LOADING…</div>
    {/if}
  </main>
  <footer class="site-footer">
    <span>INFERNO INTERNET RADIO</span>
    <span class="footer-sep">//</span>
    <a href="https://github.com/legopc/inferno-iradio" class="footer-link">legopc/inferno-iradio</a>
    <span class="footer-sep">//</span>
    <span>RUST + AXUM + SVELTE</span>
  </footer>
</div>

<style>
  :global(*) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(:root) {
    --bg: #0a0a0c;
    --surface: #141416;
    --surface2: #1c1c1f;
    --border: #2a2a2e;
    --accent: #00e5a0;
    --accent2: #e5a000;
    --danger: #e55050;
    --text: #e8e8ea;
    --text-dim: #666670;
    --text-muted: #444448;
    --vu-green: #00c060;
    --vu-yellow: #c0a000;
    --vu-red: #c02020;
    --font-mono: 'IBM Plex Mono', 'Cascadia Code', monospace;
    --font-cond: 'Barlow Condensed', sans-serif;
    --radius: 4px;
  }
  :global(body) {
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.5;
    min-height: 100vh;
  }
  :global(button) { cursor: pointer; font-family: var(--font-mono); }
  :global(input) { font-family: var(--font-mono); }
  .app-shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    max-width: 1400px;
    margin: 0 auto;
  }
  .content-area {
    flex: 1;
    padding: 16px;
  }
  .loading-placeholder {
    color: var(--text-dim);
    padding: 32px;
    text-align: center;
    letter-spacing: 0.1em;
  }
  .site-footer {
    border-top: 1px solid var(--border);
    padding: 12px 16px;
    color: var(--text-dim);
    font-size: 11px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .footer-sep { color: var(--text-muted); }
  .footer-link { color: var(--text-dim); text-decoration: none; }
  .footer-link:hover { color: var(--accent); }
</style>
