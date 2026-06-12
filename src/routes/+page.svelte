<script lang="ts">
  import { onMount } from 'svelte';
  import { projects, refreshProjects, addProject } from '$lib/stores/projects';

  let name = $state('');
  let error = $state('');

  onMount(refreshProjects);

  async function submit(e: Event) {
    e.preventDefault();
    error = '';
    try {
      await addProject(name, null);
      name = '';
    } catch (err) {
      error = JSON.stringify(err);
    }
  }
</script>

<main style="padding: 2rem; font-family: system-ui; max-width: 32rem;">
  <h1>LaboralTracker — Proyectos</h1>

  <form onsubmit={submit} style="display: flex; gap: .5rem; margin: 1rem 0;">
    <input placeholder="Nombre del proyecto" bind:value={name} required />
    <button type="submit">Crear</button>
  </form>

  {#if error}<p style="color: crimson;">Error: {error}</p>{/if}

  <ul>
    {#each $projects as p (p.id)}
      <li><strong>{p.name}</strong> <small>{p.id}</small></li>
    {/each}
  </ul>
  {#if $projects.length === 0}<p><em>Sin proyectos todavía.</em></p>{/if}
</main>
