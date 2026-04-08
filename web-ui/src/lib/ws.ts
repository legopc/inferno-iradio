import type { WsEvent } from './types';
import { players, vuLevels, icyTitles, slotHealth, maxSlots, apiOnline } from './stores';

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1000;

export function connectWs() {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  socket = new WebSocket(`${proto}//${location.host}/api/v2/ws`);

  socket.onopen = () => {
    reconnectDelay = 1000;
    apiOnline.set(true);
  };

  socket.onmessage = (ev) => {
    try {
      const event: WsEvent = JSON.parse(ev.data);
      handleEvent(event);
    } catch {}
  };

  socket.onclose = () => {
    socket = null;
    // Don't immediately flip offline — let the health-check polling in App.svelte handle that.
    // Just schedule reconnect.
    reconnectTimer = setTimeout(() => {
      reconnectDelay = Math.min(reconnectDelay * 1.5, 10000);
      connectWs();
    }, reconnectDelay);
  };

  socket.onerror = () => {
    socket?.close();
  };
}

function handleEvent(event: WsEvent) {
  switch (event.type) {
    case 'snapshot':
      players.set(event.players);
      break;
    case 'vu_batch':
      vuLevels.update(v => {
        const next = { ...v };
        for (const [slotStr, lvl] of Object.entries(event.levels)) {
          next[+slotStr] = lvl;
        }
        return next;
      });
      break;
    case 'vu':
      // legacy single-slot format (fallback)
      vuLevels.update(v => ({ ...v, [event.slot]: { l: event.l, r: event.r } }));
      break;
    case 'icy_meta':
      icyTitles.update(t => ({ ...t, [event.slot]: event.title }));
      break;
    case 'player_update':
      players.update(list => {
        const idx = list.findIndex(p => p.id === event.player.id);
        if (idx >= 0) { list[idx] = event.player; return [...list]; }
        return [...list, event.player];
      });
      break;
    case 'player_stopped':
      players.update(list => list.filter(p => p.id !== event.id));
      break;
    case 'health':
      maxSlots.set(event.max);
      break;
    case 'slot_health':
      slotHealth.update(h => {
        const idx = h.findIndex(s => s.slot === event.slot);
        const item = { slot: event.slot, connects: event.connects, errors: event.errors, last_error: event.last_error };
        if (idx >= 0) { h[idx] = item; return [...h]; }
        return [...h, item];
      });
      break;
  }
}
