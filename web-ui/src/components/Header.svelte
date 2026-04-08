<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { apiOnline, apiVersion, defaultVolume, activeCount } from '../lib/stores';
  import { api } from '../lib/api';

  export let activeTab: 'playing' | 'search' = 'playing';

  const dispatch = createEventDispatcher();

  function sliderToVol(s: number) { return Math.pow(s / 100, 3); }
  function volToSlider(v: number) { return Math.round(Math.cbrt(Math.max(0, v)) * 100); }

  let sliderVal = volToSlider(0.7);
  defaultVolume.subscribe(v => { sliderVal = volToSlider(v); });

  let debounce: ReturnType<typeof setTimeout>;
  function onVolChange(e: Event) {
    const val = +(e.target as HTMLInputElement).value;
    const vol = sliderToVol(val);
    defaultVolume.set(vol);
    clearTimeout(debounce);
    debounce = setTimeout(() => api.setVolume(vol).catch(() => {}), 300);
  }
</script>

<header class="site-header">
  <div class="header-brand">
    <div class="brand-bars" class:bars-active={$activeCount > 0}>
      <span/><span/><span/><span/><span/>
    </div>
    <div class="brand-text">
      <span class="brand-name">INFERNO<span class="brand-dot">·</span>IRADIO</span>
      <span class="brand-sub">AoIP BROADCAST BRIDGE</span>
    </div>
  </div>
  <div class="header-status">
    <span class="api-status" class:online={$apiOnline} class:offline={!$apiOnline}>
      {$apiOnline ? `[ONLINE v${$apiVersion}]` : '[OFFLINE]'}
    </span>
    <div class="vol-wrap">
      <span class="vol-label">DEFAULT VOL</span>
      <input type="range" class="vol-slider" min="0" max="100" value={sliderVal} on:input={onVolChange} />
      <span class="vol-val">{sliderVal}%</span>
    </div>
  </div>
</header>
<nav class="tab-rail">
  <button class="rail-btn" class:rail-active={activeTab === 'playing'} on:click={() => dispatch('tabChange', 'playing')}>NOW PLAYING</button>
  <button class="rail-btn" class:rail-active={activeTab === 'search'} on:click={() => dispatch('tabChange', 'search')}>SCAN STATIONS</button>
</nav>

<style>
  .site-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    gap: 16px;
    flex-wrap: wrap;
  }
  .header-brand { display: flex; align-items: center; gap: 12px; }
  .brand-bars {
    display: flex; gap: 2px; align-items: flex-end; height: 20px;
  }
  .brand-bars span {
    width: 3px; background: var(--border); border-radius: 1px;
    transition: background 0.3s;
  }
  .brand-bars span:nth-child(1) { height: 8px; }
  .brand-bars span:nth-child(2) { height: 14px; }
  .brand-bars span:nth-child(3) { height: 20px; }
  .brand-bars span:nth-child(4) { height: 12px; }
  .brand-bars span:nth-child(5) { height: 6px; }
  .brand-bars.bars-active span { background: var(--accent); }
  .brand-name {
    font-family: var(--font-mono);
    font-size: clamp(14px, 3vw, 20px);
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--text);
  }
  .brand-dot { color: var(--accent); }
  .brand-sub { font-size: 10px; color: var(--text-muted); letter-spacing: 0.15em; display: block; }
  .header-status { display: flex; align-items: center; gap: 16px; flex-wrap: wrap; }
  .api-status { font-size: 11px; letter-spacing: 0.1em; color: var(--text-muted); }
  .api-status.online  { color: var(--vu-green); }
  .api-status.offline { color: var(--error); }
  .vol-wrap { display: flex; align-items: center; gap: 8px; }
  .vol-label { font-size: 11px; color: var(--text-muted); letter-spacing: 0.08em; }
  .vol-slider { width: clamp(80px, 12vw, 140px); accent-color: var(--accent); }
  .vol-val { font-size: 11px; color: var(--text-muted); min-width: 30px; }
  .tab-rail {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    overflow-x: auto;
  }
  .rail-btn {
    padding: 12px 20px;
    background: none;
    border: none;
    border-radius: 0;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: clamp(11px, 2vw, 13px);
    letter-spacing: 0.1em;
    border-bottom: 2px solid transparent;
    white-space: nowrap;
    min-height: 44px;
    transition: color 0.15s, border-color 0.15s;
  }
  .rail-btn:hover { color: var(--text); }
  .rail-btn.rail-active { color: var(--accent); border-bottom-color: var(--accent); }
</style>
