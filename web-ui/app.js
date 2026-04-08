// ── Constants ──────────────────────────────────────────────────────────────
const API = '/api/v1';

// ── State ──────────────────────────────────────────────────────────────────
let players = [];
let favourites = [];
let searchDebounceTimer = null;
let pendingStation = null;
let maxSlots = 4; // updated from /health on load

// ── Initialisation ─────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
  checkApi();
  refreshPlayers();
  loadFavourites();
  browseTop();
  setInterval(refreshPlayers, 5000);
});

// ── Tab switching ──────────────────────────────────────────────────────────
function switchTab(tab) {
  document.querySelectorAll('.tab-section').forEach(s => s.classList.add('hidden'));
  document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('tab-btn-active'));
  document.getElementById('section-' + tab).classList.remove('hidden');
  document.getElementById('tab-' + tab).classList.add('tab-btn-active');
  if (tab === 'favourites') loadFavourites();
}

// ── API health ─────────────────────────────────────────────────────────────
async function checkApi() {
  try {
    const r = await fetch(API + '/health');
    const data = await r.json();
    const dot = document.getElementById('pillApiDot');
    const lbl = document.getElementById('pillApiLabel');
    if (r.ok) {
      dot.className = 'pill-dot active';
      lbl.textContent = 'API v' + (data.version || '?');
      if (data.max_players) maxSlots = data.max_players;
    } else {
      dot.className = 'pill-dot error';
    }
  } catch {
    document.getElementById('pillApiDot').className = 'pill-dot error';
  }
}

// ── Players ────────────────────────────────────────────────────────────────
async function refreshPlayers() {
  try {
    const r = await fetch(API + '/players');
    players = await r.json();
    renderPlayers();
  } catch (e) {
    console.error('refreshPlayers:', e);
  }
}

function renderPlayers() {
  const el = document.getElementById('playerList');
  if (!players || players.length === 0) {
    el.innerHTML = `<div class="empty-state">
      <div class="empty-icon">📻</div>
      <div class="empty-text">No active players. Search for a station and hit ▶ to start.</div>
    </div>`;
    return;
  }
  el.innerHTML = players.map(renderPlayerCard).join('');
}

function renderPlayerCard(p) {
  const stateClass = p.state === 'playing' ? 'active' : p.state === 'error' ? 'error' : 'inactive';
  const dotClass = p.state === 'playing' ? 'pulse-active' : p.state === 'error' ? 'pulse-failed' : '';
  const stateLabel = p.state.charAt(0).toUpperCase() + p.state.slice(1);
  const vol = typeof p.volume === 'number' ? p.volume : 1.0;
  const volPct = Math.round(vol * 100);
  return `
    <div class="svc-card ${stateClass}" style="margin-bottom:8px">
      <div class="svc-header">
        <div style="display:flex;align-items:center;gap:8px">
          <span class="status-dot ${dotClass}"></span>
          <span class="svc-name">${esc(p.name)}</span>
        </div>
        <div class="svc-actions">
          <button class="btn btn-sm btn-danger" onclick="stopPlayer('${p.id}')">■ Stop</button>
        </div>
      </div>
      <div class="svc-meta" style="display:flex;gap:12px;font-size:12px;color:#6a6e73;margin-top:4px;flex-wrap:wrap">
        <span class="slot-indicator">Slot ${p.slot}</span>
        <span><span class="dante-badge">TX ${p.dante_tx_channels[0]}–${p.dante_tx_channels[1]}</span></span>
        <span class="text-muted">${esc(p.alsa_device)}</span>
        <span class="svc-status">${stateLabel}${p.error ? ': ' + esc(p.error) : ''}</span>
      </div>
      <div class="volume-row">
        <span class="volume-label">🔊 Volume</span>
        <input type="range" class="volume-slider" min="0" max="100" value="${volPct}"
          oninput="onVolumeChange(this,'${p.id}')"
          onchange="setVolume('${p.id}', this.value/100)">
        <span class="volume-value" id="vol-${p.id}">${volPct}%</span>
      </div>
    </div>`;
}

async function stopPlayer(id) {
  try {
    await fetch(API + '/players/' + id, { method: 'DELETE' });
    toast('info', 'Player stopped');
    await refreshPlayers();
  } catch (e) {
    toast('error', 'Stop failed: ' + e.message);
  }
}

function onVolumeChange(slider, playerId) {
  const el = document.getElementById('vol-' + playerId);
  if (el) el.textContent = slider.value + '%';
}

let volumeDebounce = {};
async function setVolume(playerId, volume) {
  clearTimeout(volumeDebounce[playerId]);
  volumeDebounce[playerId] = setTimeout(async () => {
    try {
      await fetch(API + '/players/' + playerId + '/volume', {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ volume: parseFloat(volume) }),
      });
    } catch (e) {
      console.warn('setVolume failed:', e);
    }
  }, 80);
}

async function quickPlay() {
  const url = document.getElementById('quickUrl').value.trim();
  const name = document.getElementById('quickName').value.trim() || 'Custom Stream';
  if (!url) { toast('error', 'Please enter a stream URL'); return; }
  await startStation({ id: 'custom-' + Date.now(), url, name, codec: '', bitrate: 0 });
  document.getElementById('quickUrl').value = '';
  document.getElementById('quickName').value = '';
}

// ── Play flow ─────────────────────────────────────────────────────────────
async function startStation(station) {
  await refreshPlayers();
  const usedSlots = new Set(players.map(p => p.slot));
  const freeSlots = [];
  for (let i = 1; i <= maxSlots; i++) {
    if (!usedSlots.has(i)) freeSlots.push(i);
  }

  if (freeSlots.length === 0 && maxSlots === 1) {
    // Only one slot and it's occupied — overwrite directly
    await createPlayer(station, 1);
    return;
  }
  if (freeSlots.length === maxSlots) {
    // No occupied slots at all
    if (maxSlots === 1) { await createPlayer(station, 1); return; }
    showSlotPicker(station, usedSlots);
    return;
  }
  if (freeSlots.length === 1 && usedSlots.size === 0) {
    await createPlayer(station, freeSlots[0]);
    return;
  }
  showSlotPicker(station, usedSlots);
}

function showSlotPicker(station, usedSlots) {
  pendingStation = station;
  document.getElementById('slotModalName').textContent = station.name;
  const grid = document.getElementById('slotGrid');
  grid.innerHTML = '';
  for (let s = 1; s <= maxSlots; s++) {
    const occupied = usedSlots.has(s);
    const ch = ((s - 1) * 2 + 1);
    const occupiedPlayer = occupied ? players.find(p => p.slot === s) : null;
    const btn = document.createElement('button');
    btn.className = 'slot-btn' + (occupied ? ' occupied' : '');
    btn.innerHTML = `<div>Slot ${s}${occupied ? ' <span class="slot-overwrite-tag">⚠ overwrite</span>' : ''}</div>
      <div class="slot-ch">TX ${ch}–${ch + 1} · iradio-${s}</div>
      ${occupied && occupiedPlayer ? `<div class="slot-current">${esc(occupiedPlayer.name)}</div>` : ''}`;
    btn.onclick = () => {
      document.getElementById('slotModal').close();
      createPlayer(station, s);
    };
    grid.appendChild(btn);
  }
  document.getElementById('slotModal').showModal();
}

async function createPlayer(station, slot) {
  try {
    const r = await fetch(API + '/players', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: station.url, name: station.name, slot }),
    });
    if (!r.ok) {
      const err = await r.json();
      toast('error', err.error || 'Failed to start player');
      return;
    }
    toast('success', `▶ Playing "${station.name}" on slot ${slot}`);
    await refreshPlayers();
    switchTab('playing');
  } catch (e) {
    toast('error', 'Error: ' + e.message);
  }
}

// ── Search ─────────────────────────────────────────────────────────────────
function debouncedSearch() {
  clearTimeout(searchDebounceTimer);
  searchDebounceTimer = setTimeout(doSearch, 300);
}

async function doSearch() {
  const q = document.getElementById('searchInput').value.trim();
  if (!q) { browseTop(); return; }
  setSearchActive('btnTop', false);
  setSearchResults('<div class="loading-text"><span class="spinner"></span>Searching…</div>');
  try {
    const r = await fetch(API + '/stations/search?q=' + encodeURIComponent(q) + '&limit=40');
    const stations = await r.json();
    renderStations(stations, 'searchResults');
  } catch (e) {
    setSearchResults('<div class="empty-state"><div class="empty-icon">⚠️</div><div class="empty-text">Search failed</div></div>');
  }
}

async function browseTop() {
  setSearchActive('btnTop', true);
  setSearchResults('<div class="loading-text"><span class="spinner"></span>Loading top stations…</div>');
  try {
    const r = await fetch(API + '/stations/top?limit=40');
    const stations = await r.json();
    renderStations(stations, 'searchResults');
  } catch (e) {
    setSearchResults('<div class="empty-state"><div class="empty-icon">⚠️</div><div class="empty-text">Could not load top stations</div></div>');
  }
}

let tagsCache = null;
async function toggleTagsDropdown() {
  const dd = document.getElementById('tagsDropdown');
  if (!dd.classList.contains('hidden')) { dd.classList.add('hidden'); return; }
  if (!tagsCache) {
    dd.innerHTML = '<div style="padding:8px 12px;color:#6a6e73">Loading…</div>';
    dd.classList.remove('hidden');
    const r = await fetch(API + '/stations/tags');
    tagsCache = await r.json();
  }
  dd.innerHTML = tagsCache.slice(0, 80).map(t =>
    `<div class="dropdown-item" onclick="browseByTag('${esc(t.name)}')">${esc(t.name)} <span class="text-muted" style="float:right">${t.stationcount}</span></div>`
  ).join('');
  dd.classList.remove('hidden');
  document.addEventListener('click', closeDropdownsOnClickOutside, { once: true });
}

let countriesCache = null;
async function toggleCountriesDropdown() {
  const dd = document.getElementById('countriesDropdown');
  if (!dd.classList.contains('hidden')) { dd.classList.add('hidden'); return; }
  if (!countriesCache) {
    dd.innerHTML = '<div style="padding:8px 12px;color:#6a6e73">Loading…</div>';
    dd.classList.remove('hidden');
    const r = await fetch(API + '/stations/countries');
    countriesCache = await r.json();
  }
  dd.innerHTML = countriesCache.slice(0, 80).map(c =>
    `<div class="dropdown-item" onclick="browseByCountry('${esc(c.iso_3166_1)}')">${esc(c.name)} <span class="text-muted" style="float:right">${c.iso_3166_1}</span></div>`
  ).join('');
  dd.classList.remove('hidden');
  document.addEventListener('click', closeDropdownsOnClickOutside, { once: true });
}

function closeDropdownsOnClickOutside(e) {
  if (!e.target.closest('[id$="Dropdown"]') && !e.target.closest('#btnTags') && !e.target.closest('#btnCountries')) {
    document.getElementById('tagsDropdown').classList.add('hidden');
    document.getElementById('countriesDropdown').classList.add('hidden');
  }
}

async function browseByTag(tag) {
  document.getElementById('tagsDropdown').classList.add('hidden');
  setSearchActive('btnTags', true);
  setSearchResults('<div class="loading-text"><span class="spinner"></span>Loading…</div>');
  const r = await fetch(API + '/stations/by-tag?tag=' + encodeURIComponent(tag) + '&limit=40');
  const stations = await r.json();
  renderStations(stations, 'searchResults');
}

async function browseByCountry(code) {
  document.getElementById('countriesDropdown').classList.add('hidden');
  setSearchActive('btnCountries', true);
  setSearchResults('<div class="loading-text"><span class="spinner"></span>Loading…</div>');
  const r = await fetch(API + '/stations/by-country?country=' + encodeURIComponent(code) + '&limit=40');
  const stations = await r.json();
  renderStations(stations, 'searchResults');
}

function setSearchActive(btnId, active) {
  ['btnTop', 'btnTags', 'btnCountries'].forEach(id => {
    document.getElementById(id).classList.toggle('tab-btn-active', id === btnId && active);
  });
}

function setSearchResults(html) {
  document.getElementById('searchResults').innerHTML = html;
}

function renderStations(stations, containerId) {
  const el = document.getElementById(containerId);
  if (!stations || stations.length === 0) {
    el.innerHTML = '<div class="empty-state"><div class="empty-icon">📻</div><div class="empty-text">No stations found</div></div>';
    return;
  }
  el.innerHTML = stations.map(s => renderStationRow(s, containerId === 'favouritesList')).join('');
}

function renderStationRow(s, isFavourite) {
  const codec = (s.codec || '').toLowerCase().replace(/[^a-z0-9]/g, '');
  const codecBadge = s.codec ? `<span class="codec-badge ${codec}">${esc(s.codec)}</span>` : '';
  const bitrateText = s.bitrate ? `${s.bitrate}k` : '';
  const favBtn = isFavourite
    ? `<button class="btn btn-danger btn-sm" onclick="removeFavourite('${esc(s.id)}')">✕ Remove</button>`
    : `<button class="btn btn-sm" onclick="addFavourite(event, ${JSON.stringify(JSON.stringify(s))})" title="Save to favourites">⭐</button>`;

  const favicon = s.favicon
    ? `<img class="station-favicon" src="${esc(s.favicon)}" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'" alt="">`
    : '';
  const placeholder = `<div class="station-favicon-placeholder" style="${s.favicon ? 'display:none' : ''}">📻</div>`;

  return `
    <div class="station-row">
      ${favicon}${placeholder}
      <div class="station-info">
        <div class="station-name">${esc(s.name)}</div>
        <div class="station-meta">
          ${codecBadge}${bitrateText ? `${bitrateText} · ` : ''}${s.country ? esc(s.country) : ''}
          ${s.tags && s.tags.length ? ' · ' + s.tags.slice(0,3).map(esc).join(', ') : ''}
        </div>
      </div>
      <div class="station-actions">
        ${favBtn}
        <button class="btn btn-primary" onclick='playStation(${JSON.stringify(s)})'>▶ Play</button>
      </div>
    </div>`;
}

function playStation(station) {
  startStation(station);
}

// ── Favourites ─────────────────────────────────────────────────────────────
async function loadFavourites() {
  try {
    const r = await fetch(API + '/favourites');
    favourites = await r.json();
    renderFavourites();
  } catch (e) {
    console.error('loadFavourites:', e);
  }
}

function renderFavourites() {
  const el = document.getElementById('favouritesList');
  if (!favourites || favourites.length === 0) {
    el.innerHTML = `<div class="empty-state">
      <div class="empty-icon">⭐</div>
      <div class="empty-text">No favourites yet. Hit ⭐ on a station to save it here.</div>
    </div>`;
    return;
  }
  renderStations(favourites, 'favouritesList');
}

async function addFavourite(event, stationJson) {
  const station = JSON.parse(stationJson);
  try {
    const r = await fetch(API + '/favourites', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(station),
    });
    if (r.status === 409) { toast('info', 'Already in favourites'); return; }
    if (!r.ok) { toast('error', 'Could not save favourite'); return; }
    toast('success', `Saved "${station.name}"`);
    favourites.push(station);
  } catch (e) {
    toast('error', 'Error: ' + e.message);
  }
}

async function removeFavourite(id) {
  try {
    await fetch(API + '/favourites/' + encodeURIComponent(id), { method: 'DELETE' });
    toast('info', 'Removed from favourites');
    await loadFavourites();
  } catch (e) {
    toast('error', 'Error: ' + e.message);
  }
}

// ── Toast system ──────────────────────────────────────────────────────────
function toast(type, message) {
  const container = document.getElementById('toastContainer');
  const el = document.createElement('div');
  el.className = `toast toast-${type}`;
  el.textContent = message;
  container.appendChild(el);
  setTimeout(() => {
    el.classList.add('toast-fade-out');
    setTimeout(() => el.remove(), 400);
  }, 3500);
}

// ── Utility ────────────────────────────────────────────────────────────────
function esc(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

// Dropdown item style (injected dynamically)
const dropdownStyle = document.createElement('style');
dropdownStyle.textContent = `.dropdown-item{padding:7px 14px;cursor:pointer;font-size:13px;white-space:nowrap}.dropdown-item:hover{background:#f0f0f0}`;
document.head.appendChild(dropdownStyle);
