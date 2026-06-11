<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let status = $state('—');

  async function checkHealth() {
    status = await invoke<string>('health');
  }
</script>

<main style="padding: 2rem; font-family: system-ui;">
  <h1>LaboralTracker</h1>
  <button onclick={checkHealth}>Comprobar backend</button>
  <p>Estado: <strong>{status}</strong></p>
</main>
