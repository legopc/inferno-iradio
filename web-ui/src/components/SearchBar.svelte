<script lang="ts">
  export let value: string = '';
  export let placeholder: string = 'SEARCH…';
</script>
<input class="search-bar" type="text" bind:value {placeholder} />
<style>
  .search-bar {
    background: var(--surface2);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 8px 12px;
    border-radius: var(--radius);
    width: 100%;
  }
</style>
