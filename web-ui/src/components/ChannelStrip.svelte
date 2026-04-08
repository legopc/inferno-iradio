<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { PlayerInfo } from '../lib/types';
  import { icyTitles } from '../lib/stores';
  import { api } from '../lib/api';
  import { toast } from './Toast.svelte';
  import VuMeter from './VuMeter.svelte';

  export let player: PlayerInfo;

  // Volume: slider 0–100 → linear 0–1 via cubic taper for natural audio feel
  // 30% slider → 0.027 linear (-31dBFS), 50% → 0.125 (-18dBFS), 75% → 0.42 (-8dBFS)
  function sliderToVol(s: number): number { return Math.pow(s / 100, 3); }
  function volToSlider(v: number): number { return Math.round(Math.cbrt(Math.max(0, v)) * 100); }

  let sliderPos = volToSlider(player.volume ?? 0.5);
  let volDebounce: ReturnType<typeof setTimeout>;

  // Gain input: -6 to +6 dB
  let gainVal = player.gain_db ?? 0;
  let gainDebounce: ReturnType<typeof setTimeout>;

  // ICY title from store
  let icy = '';
  const unsubIcy = icyTitles.subscribe(t => { icy = t[player.slot] ?? ''; });
  onDestroy(unsubIcy);

  $: stateLabel = player.state === 'playing' ? 'LIVE'
                : player.state === 'buffering' ? 'BUFFERING'
                : player.state === 'error' ? 'ERROR'
                : 'STOPPED';

  function onVolInput(e: Event) {
    sliderPos = +(e.target as HTMLInputElement).value;
    const vol = sliderToVol(sliderPos);
    clearTimeout(volDebounce);
    volDebounce = setTimeout(() => {
      api.setPlayerVolume(player.id, vol).catch((err: Error) => toast('error', err.message));
    }, 200);
  }

  function onGainInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (isNaN(v)) return;
    gainVal = Math.max(-6, Math.min(6, v));
    clearTimeout(gainDebounce);
    gainDebounce = setTimeout(() => {
      api.setPlayerGain(player.id, gainVal).catch((err: Error) => toast('error', err.message));
    }, 500);
  }

  async function stop() {
    if (player.state === 'playing' && !confirm(`Stop "${player.name}"?`)) return;
    try {
      await api.deletePlayer(player.id);
      toast('info', `Stopped: ${player.name}`);
    } catch (err: unknown) {
      toast('error', err instanceof Error ? err.message : 'Failed to stop');
    }
  }
</script>

<div class="channel-strip" class:state-error={player.state === 'error'}>
  <!-- Slot badge + stop button -->
  <div class="strip-top">
    <span class="slot-badge">S{player.slot}</span>
    <button class="stop-btn" on:click={stop} title="Stop player" aria-label="Stop {player.name}">×</button>
  </div>

  <!-- VU meter -->
  <div class="vu-wrap">
    <VuMeter slot={player.slot} />
  </div>

  <!-- Station name -->
  <div class="station-name" title={player.name}>{player.name}</div>

  <!-- ICY title ticker -->
  <div class="icy-wrap">
    {#if icy && icy.length > 24}
      <div class="ticker-outer">
        <span class="ticker-inner">{icy}</span>
      </div>
    {:else}
      <div class="icy-static">{icy || '—'}</div>
    {/if}
  </div>

  <!-- State badge -->
  <span class="state-badge state-{player.state}">{stateLabel}</span>

  <!-- Volume slider (vertical) -->
  <div class="vol-section">
    <span class="ctrl-lbl">VOL</span>
    <div class="vol-wrap">
      <input
        type="range"
        class="vol-slider"
        min="0" max="100" step="1"
        value={sliderPos}
        on:input={onVolInput}
        aria-label="Volume"
      />
    </div>
    <span class="vol-val">{sliderPos}%</span>
  </div>

  <!-- Gain input -->
  <div class="gain-section">
    <span class="ctrl-lbl">GAIN</span>
    <input
      type="number"
      class="gain-input"
      min="-6" max="6" step="0.5"
      value={gainVal}
      on:change={onGainInput}
      aria-label="Gain dB"
    />
    <span class="ctrl-lbl">dB</span>
  </div>
</div>

<style>
  .channel-strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 10px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    width: 120px;
    min-width: 110px;
    height: 100%;
    min-height: 340px;
    transition: border-color 0.2s;
  }
  .channel-strip.state-error { border-color: var(--error); }

  .strip-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }
  .slot-badge {
    font-size: 10px;
    background: var(--accent);
    color: #000;
    padding: 2px 5px;
    border-radius: 2px;
    font-weight: 700;
    letter-spacing: 0.05em;
  }
  .stop-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-dim);
    width: 22px;
    height: 22px;
    border-radius: 2px;
    font-size: 14px;
    line-height: 1;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s, color 0.1s;
  }
  .stop-btn:hover { background: var(--error); color: #fff; border-color: var(--error); }

  .vu-wrap {
    width: 100%;
    height: 100px;
    flex: none;
  }

  .station-name {
    width: 100%;
    font-size: 11px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    letter-spacing: 0.03em;
  }

  .icy-wrap {
    width: 100%;
    height: 14px;
    overflow: hidden;
  }
  .ticker-outer {
    width: 100%;
    overflow: hidden;
    white-space: nowrap;
  }
  .ticker-inner {
    display: inline-block;
    font-size: 10px;
    color: var(--text-dim);
    padding-right: 2em;
    animation: ticker-scroll 14s linear infinite;
    will-change: transform;
  }
  @keyframes ticker-scroll {
    from { transform: translateX(100%); }
    to   { transform: translateX(-100%); }
  }
  .icy-static {
    font-size: 10px;
    color: var(--text-dim);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .state-badge {
    font-size: 10px;
    padding: 2px 7px;
    border-radius: 2px;
    letter-spacing: 0.1em;
    font-weight: 700;
  }
  .state-playing   { background: var(--vu-green); color: #000; }
  .state-buffering { background: var(--warning); color: #000; animation: pulse 1s ease-in-out infinite; }
  .state-error     { background: var(--error); color: #fff; }
  .state-stopped   { background: var(--text-muted); color: var(--text-dim); }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.45; } }

  /* Vertical volume slider */
  .vol-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    width: 100%;
  }
  .vol-wrap {
    height: 80px;
    width: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: visible;
  }
  .vol-slider {
    width: 80px;
    height: 20px;
    transform: rotate(-90deg);
    transform-origin: center;
    accent-color: var(--accent);
    cursor: pointer;
  }
  .vol-val { font-size: 10px; color: var(--text-dim); }

  /* Gain input */
  .gain-section {
    display: flex;
    align-items: center;
    gap: 3px;
    width: 100%;
    justify-content: center;
  }
  .gain-input {
    width: 48px;
    background: var(--surface2);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 2px;
    text-align: center;
  }
  .gain-input:focus { border-color: var(--accent); outline: none; }

  .ctrl-lbl { font-size: 9px; color: var(--text-dim); letter-spacing: 0.08em; }

  /* Mobile: horizontal layout */
  @media (max-width: 600px) {
    .channel-strip {
      flex-direction: row;
      width: 100%;
      min-height: unset;
      height: auto;
      padding: 8px 12px;
      align-items: center;
      flex-wrap: wrap;
      gap: 8px;
    }
    .vu-wrap { width: 60px; height: 60px; flex: none; }
    .vol-section { flex-direction: row; width: auto; }
    .vol-wrap { height: 44px; }
    .vol-slider { width: 60px; }
    .strip-top { width: auto; order: -1; }
    .station-name { flex: 1; min-width: 80px; text-align: left; }
    .icy-wrap { width: 100%; order: 10; }
  }
</style>
