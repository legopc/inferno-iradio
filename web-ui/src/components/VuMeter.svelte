<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { vuLevels } from '../lib/stores';

  export let slot: number = 0;

  const NUM_SEGS = 12;
  const PEAK_HOLD_MS = 1500;

  // Control points: [dBFS, seg-count lit from bottom]
  // -60=seg0, -40=seg3, -20=seg6, -12=seg8, -6=seg9, -3=seg10, 0=seg11
  const PTS: [number, number][] = [
    [-60, 1], [-40, 4], [-20, 7], [-12, 9], [-6, 10], [-3, 11], [0, 12]
  ];

  function dbToCount(db: number): number {
    if (db < -60) return 0;
    if (db >= 0)  return 12;
    for (let i = 1; i < PTS.length; i++) {
      const [db0, c0] = PTS[i - 1];
      const [db1, c1] = PTS[i];
      if (db <= db1) {
        const t = (db - db0) / (db1 - db0);
        return Math.round(c0 + t * (c1 - c0));
      }
    }
    return 12;
  }

  function segColor(idx: number): string {
    if (idx >= 11) return 'var(--vu-red)';
    if (idx >= 9)  return 'var(--vu-yellow)';
    return 'var(--vu-green)';
  }

  // Raw levels from WebSocket (updated by store sub)
  let rawL = -60, rawR = -60;

  // Display state driven by RAF
  let lCount = 0, rCount = 0;
  let lPeak  = 0, rPeak  = 0;
  let lPeakExpiry = 0, rPeakExpiry = 0;

  const unsub = vuLevels.subscribe(v => {
    const entry = v[slot];
    rawL = entry ? entry.l : -60;
    rawR = entry ? entry.r : -60;
  });

  let rafId: number;

  function frame(ts: number) {
    const lC = dbToCount(rawL);
    const rC = dbToCount(rawR);

    if (lC > lPeak) {
      lPeak = lC; lPeakExpiry = ts + PEAK_HOLD_MS;
    } else if (ts > lPeakExpiry && lPeak > lC) {
      lPeak = Math.max(lC, lPeak - 1);
    }
    if (rC > rPeak) {
      rPeak = rC; rPeakExpiry = ts + PEAK_HOLD_MS;
    } else if (ts > rPeakExpiry && rPeak > rC) {
      rPeak = Math.max(rC, rPeak - 1);
    }

    lCount = lC;
    rCount = rC;
    rafId = requestAnimationFrame(frame);
  }

  onMount(() => { rafId = requestAnimationFrame(frame); });
  onDestroy(() => { unsub(); cancelAnimationFrame(rafId); });
</script>

<div class="vu-meter" aria-label="VU meter slot {slot}">
  {#each ['l', 'r'] as ch}
    {@const count = ch === 'l' ? lCount : rCount}
    {@const peak  = ch === 'l' ? lPeak  : rPeak}
    <div class="vu-col">
      {#each Array(NUM_SEGS) as _, i}
        {@const idx    = NUM_SEGS - 1 - i}
        {@const lit    = idx < count}
        {@const isPeak = !lit && idx === peak - 1 && peak > 0}
        <div
          class="vu-seg"
          style:background={lit || isPeak ? segColor(idx) : 'var(--vu-off)'}
          style:opacity={lit ? '1' : isPeak ? '0.75' : '0.12'}
        ></div>
      {/each}
      <span class="vu-lbl">{ch.toUpperCase()}</span>
    </div>
  {/each}
</div>

<style>
  .vu-meter {
    --vu-off: #1e2e1e;
    display: flex;
    gap: 3px;
    min-width: 40px;
    width: 100%;
    height: 100%;
  }
  .vu-col {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 1px;
  }
  .vu-seg {
    flex: 1;
    border-radius: 1px;
    min-height: 4px;
  }
  .vu-lbl {
    font-size: 8px;
    color: var(--text-dim);
    text-align: center;
    margin-top: 2px;
    letter-spacing: 0.05em;
    flex: none;
  }
</style>
