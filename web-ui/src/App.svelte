<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { connectWs } from './lib/ws';
  import { api } from './lib/api';
  import { players, maxSlots, defaultVolume, apiOnline, apiVersion } from './lib/stores';
  import Header from './components/Header.svelte';
  import Toast from './components/Toast.svelte';

  let activeTab: 'playing' | 'search' = 'playing';

  // Lazy-loaded route components
  let PlayingRoute: any = null;
  let SearchRoute: any = null;

  let healthInterval: ReturnType<typeof setInterval>;

  async function pollHealth() {
    try {
      const health = await api.health();
      apiVersion.set(health.version || '2.0');
      maxSlots.set(health.max_players || 4);
      apiOnline.set(true);
    } catch {
      apiOnline.set(false);
    }
  }

  onMount(async () => {
    connectWs();
    await pollHealth();
    healthInterval = setInterval(pollHealth, 10000);

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
  });

  onDestroy(() => clearInterval(healthInterval));

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
    /* Core palette */
    --bg: #1a1a1a;
    --surface: #222;
    --surface-elevated: #2a2a2a;
    --border: #333;
    --accent: #00e5a0;
    --accent-dim: #009968;
    --text: #e0e0e0;
    --text-muted: #888;
    --error: #e05555;
    --warning: #e0a020;
    /* VU colours */
    --vu-green: #00e060;
    --vu-yellow: #e0c020;
    --vu-red: #e03030;
    --vu-off: #1e2e1e;
    /* Typography */
    --font-mono: 'IBM Plex Mono', 'Courier New', monospace;
    /* Layout */
    --radius: 4px;
    --strip-width: 120px;
    /* Legacy aliases so existing component refs keep working */
    --surface2: var(--surface-elevated);
    --surface3: #2e2e2e;
    --text-dim: var(--text-muted);
    --danger: var(--error);
    --accent2: var(--warning);
    --font-cond: var(--font-mono);
  }

  :global(body) {
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.5;
    min-height: 100vh;
  }

  /* ── Custom scrollbar ──────────────────────────────────────────────── */
  :global(::-webkit-scrollbar) { width: 6px; height: 6px; }
  :global(::-webkit-scrollbar-track) { background: var(--surface); }
  :global(::-webkit-scrollbar-thumb) { background: var(--accent-dim); border-radius: 3px; }
  :global(::-webkit-scrollbar-thumb:hover) { background: var(--accent); }
  :global(*) { scrollbar-width: thin; scrollbar-color: var(--accent-dim) var(--surface); }

  /* ── Focus ─────────────────────────────────────────────────────────── */
  :global(*:focus-visible) { outline: 1px solid var(--accent); outline-offset: 2px; }
  :global(*:focus:not(:focus-visible)) { outline: none; }

  /* ── Element resets ────────────────────────────────────────────────── */
  :global(button) { cursor: pointer; font-family: var(--font-mono); }
  :global(input)  { font-family: var(--font-mono); }

  /* ── Global range input (horizontal) ──────────────────────────────── */
  :global(input[type="range"]) {
    -webkit-appearance: none;
    appearance: none;
    background: transparent;
    cursor: pointer;
    min-height: 44px;
  }
  :global(input[type="range"]::-webkit-slider-runnable-track) {
    background: var(--border);
    height: 4px;
    border-radius: 2px;
  }
  :global(input[type="range"]::-webkit-slider-thumb) {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent);
    margin-top: -5px;
    box-shadow: 0 0 6px rgba(0,229,160,0.4);
  }
  :global(input[type="range"]::-moz-range-track) {
    background: var(--border);
    height: 4px;
    border-radius: 2px;
  }
  :global(input[type="range"]::-moz-range-thumb) {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent);
    border: none;
    box-shadow: 0 0 6px rgba(0,229,160,0.4);
  }

  /* ── Responsive channel grid ───────────────────────────────────────── */
  :global(.channel-grid) {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--strip-width), 1fr));
    gap: 12px;
    padding: 12px;
  }
  @media (min-width: 1024px) {
    :global(.channel-grid) { grid-template-columns: repeat(4, 1fr); }
  }
  @media (min-width: 600px) and (max-width: 1023px) {
    :global(.channel-grid) { grid-template-columns: repeat(2, 1fr); }
  }
  @media (max-width: 599px) {
    :global(.channel-grid) { grid-template-columns: 1fr; }
  }

  /* ── Keyframe animations ───────────────────────────────────────────── */
  @keyframes buf-pulse {
    0%, 100% { opacity: 0.8; }
    50%       { opacity: 0.3; }
  }
  @keyframes blink {
    50% { opacity: 0; }
  }
  @keyframes toast-in {
    from { transform: translateX(120%); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
  }

  /* ── App shell ─────────────────────────────────────────────────────── */
  .app-shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    max-width: 1400px;
    margin: 0 auto;
  }
  .content-area {
    flex: 1;
    padding: 0;
  }
  .loading-placeholder {
    color: var(--text-muted);
    padding: 32px;
    text-align: center;
    letter-spacing: 0.1em;
  }
  .site-footer {
    border-top: 1px solid var(--border);
    padding: 12px 16px;
    color: var(--text-muted);
    font-size: 11px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .footer-sep  { color: var(--border); }
  .footer-link { color: var(--text-muted); text-decoration: none; }
  .footer-link:hover { color: var(--accent); }
</style>
