<script lang="ts">
  import { api } from '../lib/api';
  let favs: any[] = [];
  api.getFavourites().then(r => favs = r).catch(() => {});
</script>
<div class="fav-placeholder">
  <p style="color:var(--text-dim);padding:16px;">PRESETS — {favs.length} saved</p>
</div>
