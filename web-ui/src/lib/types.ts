export interface PlayerInfo {
  id: string;
  slot: number;
  name: string;
  url: string;
  state: 'buffering' | 'playing' | 'error' | 'stopped';
  dante_tx_channels: [number, number];
  alsa_device: string;
  started_at: string;
  volume: number;
  gain_db: number;
  icy_title?: string;
  error?: string;
}

export interface Station {
  id: string;
  name: string;
  url: string;
  codec: string;
  bitrate: number;
  country: string;
  tags: string[];
  favicon: string;
}

export interface SlotHealth {
  slot: number;
  connects: number;
  errors: number;
  last_error?: string;
}

export interface HealthResponse {
  version: string;
  active_players: number;
  max_players: number;
  slot_health: SlotHealth[];
}

export interface VolumeResponse {
  volume: number;
  slots: number[];
}

export type WsEvent =
  | { type: 'snapshot'; players: PlayerInfo[] }
  | { type: 'vu_batch'; levels: Record<string, { l: number; r: number }> }
  | { type: 'vu'; slot: number; l: number; r: number }
  | { type: 'icy_meta'; slot: number; title: string }
  | { type: 'player_update'; player: PlayerInfo }
  | { type: 'player_stopped'; id: string }
  | { type: 'health'; active: number; max: number }
  | { type: 'slot_health'; slot: number; connects: number; errors: number; last_error?: string };
