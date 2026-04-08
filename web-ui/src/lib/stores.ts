import { writable, derived } from 'svelte/store';
import type { PlayerInfo, SlotHealth } from './types';

export const players = writable<PlayerInfo[]>([]);
export const maxSlots = writable<number>(4);
export const defaultVolume = writable<number>(0.7);
export const apiOnline = writable<boolean>(false);
export const apiVersion = writable<string>('');

// Per-slot VU levels: slot -> { l: dBFS, r: dBFS }
export const vuLevels = writable<Record<number, { l: number; r: number }>>({});

// Per-slot ICY titles
export const icyTitles = writable<Record<number, string>>({});

// Slot health info
export const slotHealth = writable<SlotHealth[]>([]);

// Derived: active players count
export const activeCount = derived(players, $p => $p.filter(p => p.state === 'playing').length);
