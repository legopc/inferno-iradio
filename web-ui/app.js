// ── Constants ──────────────────────────────────────────────────────────────
const API = '/api/v1';

// ── State ──────────────────────────────────────────────────────────────────
let players = [];
let favourites = [];
let searchDebounceTimer = null;
let pendingStation = null;
let maxSlots = 4;
let defaultVolume = 0.7;

// ── Init ───────────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
  checkApi();
  refreshPlayers();
  loadFavourites();
  browseTop();
  setInterval(refreshPlayers, 5000);
});

// ── Volume helpers (quadratic perceptual curve) ────────────────────────────
function sliderToVol(s) { return Math.pow(s / 100, 2); }
function volToSlider(v) { return Math.round(Math.sqrt(Math.max(0, v)) * 100); }

// ── Tabs ───────────────────────────────────────────────────────────────────
function switchTab(tab) {
  document.querySelectorAll('.tab-section').forEach(s => s.classList.add('hidden'));
  document.querySelectorAll('.rail-btn').forEach(b => b.classList.remove('rail-active'));
  document.getElementById('section-' + tab).classList.remove('hidden');
  document.getElementById('tab-' + tab).classList.add('rail-active');
  if (tab === 'favourites') loadFavourites();
}

// ── API health ─────────────────────────────────────────────────────────────
async function checkApi() {
  const el = document.getElementById('apiStatus');
  try {
    const r = await fetch(API + '/health');
    const data = await r.json();
    if (r.ok) {
      el.textContent = `[ONLINE v${data.version || '?'}]`;
      el.className = 'api-status online';
      if (data.max_players) maxSlots = data.max_players;
    } else {
      el.textContent = '[DEGRADED]';
      el.className = 'api-status offline';
    }
  } catch {
    el.textContent = '[OFFLINE]';
    el.className = 'api-status offline';
  }
  // Load persisted default volume
  try {
    const r = await fetch(API + '/volume');
    if (r.ok) {
      const data = await r.json();
      defaultVolume = typeof data.volume === 'number' ? data.volume : 0.7;
      const s = volToSlider(defaultVolume);
      const sl = document.getElementById('globalVolSlider');
      const lb = document.getElementById('globalVolLabel');
      if (sl) sl.value = s;
      if (lb) lb.textContent = s + '%';
    }
  } catch {}
}

// ── Players ────────────────────────────────────────────────────────────────
async function refreshPlayers() {
  try {
    const r = await fetch(API + '/players');
    players = await r.json();
    renderPlayers();
    updateHeaderBars();
  } catch (e) {
    console.error('refreshPlayers:', e);
  }
}

function updateHeaderBars() {
  const bars = document.getElementById('headerBars');
  if (!bars) return;
  const active = players.some(p => p.state === 'playing');
  bars.classList.toggle('bars-idle', !active);
}

function renderPlayers() {
  const el = document.getElementById('playerList');
  if (!players || players.length === 0) {
    el.innerHTML = `<div class="boot-msg"><span class="boot-cursor">▮</span> NO ACTIVE CHANNELS — SCAN TO TUNE IN</div>`;
    return;
  }
  el.innerHTML = players.map(renderChannelStrip).join('');
}

function renderChannelStrip(p) {
  const isActive = p.state === 'playing';
  const isError  = p.state === 'error';
  const stripClass = isActive ? 'ch-active' : isError ? 'ch-error' : '';
  const stateLabel = p.state.toUpperCase();
  const vol = typeof p.volume === 'number' ? p.volume : 0.7;
  const sliderVal = volToSlider(vol);
  const txA = p.dante_tx_channels ? p.dante_tx_channels[0] : '?';
  const txB = p.dante_tx_channels ? p.dante_tx_channels[1] : '?';

  // 8 VU bars
  const vuBars = Array.from({length: 8}, () => `<div class="vu-bar"></div>`).join('');

  return `
    <div class="ch-strip ${stripClass}">
      <div class="ch-strip-top">
        <span class="ch-badge">CH·${String(p.slot).padStart(2,'0')}</span>
        <span class="ch-tx-badge">TX ${txA}–${txB}</span>
        <button class="ch-stop-btn" onclick="stopPlayer('${p.id}')">■ STOP</button>
      </div>
      <div class="ch-strip-body">
        <div class="ch-station-name">${esc(p.name)}</div>
        <div class="ch-state-line">${stateLabel}${isError && p.error ? ': ' + esc(p.error.slice(0,40)) : ''}</div>
        <div class="vu-meter">${vuBars}</div>
        <div class="ch-vol-row">
          <span class="ch-vol-label">VOL</span>
          <input type="range" class="ch-vol-slider" min="0" max="100" value="${sliderVal}"
            oninput="onVolumeChange(this,'${p.id}')"
            onchange="setVolume('${p.id}', sliderToVol(this.value))">
          <span class="ch-vol-val" id="vol-${p.id}">${sliderVal}%</span>
        </div>
      </div>
    </div>`;
}

async function stopPlayer(id) {
  try {
    await fetch(API + '/players/' + id, { method: 'DELETE' });
    toast('info', '[INFO] Player stopped');
    await refreshPlayers();
  } catch (e) {
    toast('error', '[ERR] Stop failed: ' + e.message);
  }
}

let globalVolDebounce = null;
function onGlobalVolumeChange(slider) {
  defaultVolume = sliderToVol(slider.value);
  const el = document.getElementById('globalVolLabel');
  if (el) el.textContent = slider.value + '%';
  clearTimeout(globalVolDebounce);
  globalVolDebounce = setTimeout(async () => {
    try {
      await fetch(API + '/volume', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ volume: defaultVolume }),
      });
    } catch {}
  }, 300);
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
  if (!url) { toast('error', '[ERR] Enter a stream URL'); return; }
  await startStation({ id: 'custom-' + Date.now(), url, name, codec: '', bitrate: 0 });
  document.getElementById('quickUrl').value = '';
  document.getElementById('quickName').value = '';
}

// ── Play flow ──────────────────────────────────────────────────────────────
async function startStation(station) {
  await refreshPlayers();
  const usedSlots = new Set(players.map(p => p.slot));
  const freeSlots = [];
  for (let i = 1; i <= maxSlots; i++) {
    if (!usedSlots.has(i)) freeSlots.push(i);
  }
  if (freeSlots.length === 0 && maxSlots === 1) {
    await createPlayer(station, 1); return;
  }
  if (freeSlots.length === maxSlots && maxSlots === 1) {
    await createPlayer(station, 1); return;
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
    btn.innerHTML = `
      <div class="slot-btn-id">CH·${String(s).padStart(2,'0')}${occupied ? ' <span class="slot-overwrite-tag">⚠ OVERWRITE</span>' : ''}</div>
      <div class="slot-btn-ch">TX ${ch}–${ch+1} · IRADIO-${s}</div>
      ${occupiedPlayer ? `<div class="slot-btn-current">${esc(occupiedPlayer.name)}</div>` : ''}`;
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
      body: JSON.stringify({ url: station.url, name: station.name, slot, volume: defaultVolume }),
    });
    if (!r.ok) {
      const err = await r.json();
      toast('error', '[ERR] ' + (err.error || 'Failed to start'));
      return;
    }
    toast('success', `[OK] TX ${station.name} → CH·${String(slot).padStart(2,'0')}`);
    await refreshPlayers();
    switchTab('playing');
  } catch (e) {
    toast('error', '[ERR] ' + e.message);
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
  setFilterActive('btnTop', false);
  setSearchResults(loadingHtml('SCANNING…'));
  try {
    const r = await fetch(API + '/stations/search?q=' + encodeURIComponent(q) + '&limit=40');
    const stations = await r.json();
    renderStations(stations, 'searchResults');
  } catch {
    setSearchResults(emptyHtml('SCAN ERROR'));
  }
}

async function browseTop() {
  setFilterActive('btnTop', true);
  setSearchResults(loadingHtml('LOADING TOP STATIONS…'));
  try {
    const r = await fetch(API + '/stations/top?limit=40');
    const stations = await r.json();
    renderStations(stations, 'searchResults');
  } catch {
    setSearchResults(emptyHtml('LOAD FAILED'));
  }
}

let tagsCache = null;
async function toggleTagsDropdown() {
  const dd = document.getElementById('tagsDropdown');
  if (!dd.classList.contains('hidden')) { dd.classList.add('hidden'); return; }
  document.getElementById('countriesDropdown').classList.add('hidden');
  if (!tagsCache) {
    dd.innerHTML = '<div class="dropdown-item" style="color:var(--text-dim)">Loading…</div>';
    dd.classList.remove('hidden');
    const r = await fetch(API + '/stations/tags');
    tagsCache = await r.json();
  }
  dd.innerHTML = tagsCache.slice(0, 80).map(t =>
    `<div class="dropdown-item" onclick="browseByTag('${esc(t.name)}')">${esc(t.name)}<span class="dropdown-count">${t.stationcount}</span></div>`
  ).join('');
  dd.classList.remove('hidden');
  document.addEventListener('click', closeDropdownsOnClick, { once: true });
}

let countriesCache = null;
async function toggleCountriesDropdown() {
  const dd = document.getElementById('countriesDropdown');
  if (!dd.classList.contains('hidden')) { dd.classList.add('hidden'); return; }
  document.getElementById('tagsDropdown').classList.add('hidden');
  if (!countriesCache) {
    dd.innerHTML = '<div class="dropdown-item" style="color:var(--text-dim)">Loading…</div>';
    dd.classList.remove('hidden');
    const r = await fetch(API + '/stations/countries');
    countriesCache = await r.json();
  }
  dd.innerHTML = countriesCache.slice(0, 80).map(c =>
    `<div class="dropdown-item" onclick="browseByCountry('${esc(c.iso_3166_1)}')">${esc(c.name)}<span class="dropdown-count">${esc(c.iso_3166_1)}</span></div>`
  ).join('');
  dd.classList.remove('hidden');
  document.addEventListener('click', closeDropdownsOnClick, { once: true });
}

function closeDropdownsOnClick(e) {
  const inDrop = e.target.closest('#tagsDropdown,#countriesDropdown,#btnTagsDrop,#btnCountriesDrop');
  if (!inDrop) {
    document.getElementById('tagsDropdown').classList.add('hidden');
    document.getElementById('countriesDropdown').classList.add('hidden');
  }
}

async function browseByTag(tag) {
  document.getElementById('tagsDropdown').classList.add('hidden');
  setFilterActive('btnTagsDrop', true);
  setSearchResults(loadingHtml('SCANNING TAG: ' + tag));
  const r = await fetch(API + '/stations/by-tag?tag=' + encodeURIComponent(tag) + '&limit=40');
  renderStations(await r.json(), 'searchResults');
}

async function browseByCountry(code) {
  document.getElementById('countriesDropdown').classList.add('hidden');
  setFilterActive('btnCountriesDrop', true);
  setSearchResults(loadingHtml('SCANNING COUNTRY: ' + code));
  const r = await fetch(API + '/stations/by-country?country=' + encodeURIComponent(code) + '&limit=40');
  renderStations(await r.json(), 'searchResults');
}

function setFilterActive(btnId, active) {
  ['btnTop','btnTagsDrop','btnCountriesDrop'].forEach(id => {
    document.getElementById(id)?.classList.toggle('filter-active', id === btnId && active);
  });
}

function setSearchResults(html) {
  document.getElementById('searchResults').innerHTML = html;
}

function renderStations(stations, containerId) {
  const el = document.getElementById(containerId);
  if (!stations || stations.length === 0) {
    el.innerHTML = emptyHtml('NO STATIONS FOUND');
    return;
  }
  const isFav = containerId === 'favouritesList';
  el.innerHTML = stations.map(s => renderStationRow(s, isFav)).join('');
}

function renderStationRow(s, isFavourite) {
  const codec = (s.codec || '').toLowerCase().replace(/[^a-z0-9]/g, '');
  const codecBadge = s.codec ? `<span class="codec-badge ${codec}">${esc(s.codec.toUpperCase())}</span>` : '';
  const bitrateText = s.bitrate ? `${s.bitrate}k` : '';
  const meta = [codecBadge, bitrateText, s.country ? esc(s.country) : '',
    s.tags && s.tags.length ? s.tags.slice(0,3).map(esc).join(', ') : '']
    .filter(Boolean).join(' · ');

  const favBtn = isFavourite
    ? `<button class="unfav-btn" onclick="removeFavourite('${esc(s.id)}')">✕ REMOVE</button>`
    : `<button class="fav-btn" onclick="addFavourite(event, ${JSON.stringify(JSON.stringify(s))})" title="Save preset">★</button>`;

  const favicon = s.favicon
    ? `<img class="station-favicon" src="${esc(s.favicon)}" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'" alt="">`
    : '';
  const placeholder = `<div class="station-favicon-placeholder" style="${s.favicon ? 'display:none' : ''}">◈</div>`;

  return `
    <div class="station-row">
      ${favicon}${placeholder}
      <div class="station-info">
        <div class="station-name">${esc(s.name)}</div>
        <div class="station-meta">${meta}</div>
      </div>
      <div class="station-actions">
        ${favBtn}
        <button class="play-btn" onclick='playStation(${JSON.stringify(s)})'>▶ TX</button>
      </div>
    </div>`;
}

function playStation(station) { startStation(station); }

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
    el.innerHTML = emptyHtml('NO PRESETS — HIT ★ ON ANY STATION');
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
    if (r.status === 409) { toast('info', '[INFO] Already a preset'); return; }
    if (!r.ok) { toast('error', '[ERR] Could not save preset'); return; }
    toast('success', `[OK] Preset saved: "${station.name}"`);
    favourites.push(station);
  } catch (e) {
    toast('error', '[ERR] ' + e.message);
  }
}

async function removeFavourite(id) {
  try {
    await fetch(API + '/favourites/' + encodeURIComponent(id), { method: 'DELETE' });
    toast('info', '[INFO] Preset removed');
    await loadFavourites();
  } catch (e) {
    toast('error', '[ERR] ' + e.message);
  }
}

// ── Toasts ─────────────────────────────────────────────────────────────────
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

// ── Helpers ────────────────────────────────────────────────────────────────
function esc(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#x27;');
}

function loadingHtml(msg) {
  return `<div class="manifest-loading"><span class="spin"></span>${msg}</div>`;
}

function emptyHtml(msg) {
  return `<div class="manifest-empty"><span class="manifest-empty-icon">◈</span>${msg}</div>`;
}
