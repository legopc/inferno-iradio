const BASE = '/api/v2';

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(BASE + path, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || res.statusText);
  }
  return res.json();
}

export const api = {
  health: () => req<any>('GET', '/health'),
  config: () => req<any>('GET', '/config'),
  getVolume: () => req<any>('GET', '/volume'),
  setVolume: (volume: number) => req<any>('PUT', '/volume', { volume }),

  listPlayers: () => req<any[]>('GET', '/players'),
  createPlayer: (url: string, name: string, slot?: number, volume?: number, gain_db?: number) =>
    req<any>('POST', '/players', { url, name, slot, volume, gain_db }),
  deletePlayer: (id: string) => req<void>('DELETE', `/players/${id}`),
  setPlayerVolume: (id: string, volume: number) =>
    req<any>('PATCH', `/players/${id}/volume`, { volume }),
  setPlayerGain: (id: string, gain_db: number) =>
    req<any>('PATCH', `/players/${id}/gain`, { gain_db }),

  searchStations: (q: string, limit = 40) =>
    req<any[]>('GET', `/stations/search?q=${encodeURIComponent(q)}&limit=${limit}`),
  topStations: (limit = 40) => req<any[]>('GET', `/stations/top?limit=${limit}`),
  tags: () => req<any[]>('GET', '/stations/tags'),
  countries: () => req<any[]>('GET', '/stations/countries'),
  byTag: (tag: string, limit = 40) =>
    req<any[]>('GET', `/stations/by-tag?tag=${encodeURIComponent(tag)}&limit=${limit}`),
  byCountry: (country: string, limit = 40) =>
    req<any[]>('GET', `/stations/by-country?country=${encodeURIComponent(country)}&limit=${limit}`),

  getFavourites: () => req<any[]>('GET', '/favourites'),
  addFavourite: (station: any) => req<any>('POST', '/favourites', station),
  removeFavourite: (id: string) => req<void>('DELETE', `/favourites/${encodeURIComponent(id)}`),
};
