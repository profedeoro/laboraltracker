<script lang="ts">
  import { onMount } from 'svelte';
  import { projects, refreshProjects, addProject } from '$lib/stores/projects';
  import { tasks, selectedProjectId, loadTasks, addTask } from '$lib/stores/tasks';

  let projectName = $state('');
  let taskName = $state('');
  let error = $state('');

  onMount(refreshProjects);

  async function submitProject(e: Event) {
    e.preventDefault();
    error = '';
    try {
      await addProject(projectName, null);
      projectName = '';
    } catch (err) {
      error = JSON.stringify(err);
    }
  }

  async function selectProject(id: string) {
    error = '';
    try {
      await loadTasks(id);
    } catch (err) {
      error = JSON.stringify(err);
    }
  }

  async function submitTask(e: Event) {
    e.preventDefault();
    error = '';
    const pid = $selectedProjectId;
    if (!pid) return;
    try {
      await addTask(pid, taskName);
      taskName = '';
    } catch (err) {
      error = JSON.stringify(err);
    }
  }
</script>

<main style="padding: 2rem; font-family: system-ui; max-width: 40rem;">
  <h1>LaboralTracker — Proyectos y Tareas</h1>

  <form onsubmit={submitProject} style="display: flex; gap: .5rem; margin: 1rem 0;">
    <input placeholder="Nombre del proyecto" bind:value={projectName} required />
    <button type="submit">Crear proyecto</button>
  </form>

  {#if error}<p style="color: crimson;">Error: {error}</p>{/if}

  <ul>
    {#each $projects as p (p.id)}
      <li>
        <button
          type="button"
          onclick={() => selectProject(p.id)}
          style="font-weight: {$selectedProjectId === p.id ? 'bold' : 'normal'};"
        >
          {p.name}
        </button>
        <small>{p.id}</small>
      </li>
    {/each}
  </ul>
  {#if $projects.length === 0}<p><em>Sin proyectos todavía.</em></p>{/if}

  {#if $selectedProjectId}
    {@const selected = $projects.find((p) => p.id === $selectedProjectId)}
    <hr style="margin: 1.5rem 0;" />
    <h2>Tareas de <span style="color: #2563eb;">{selected?.name ?? ''}</span></h2>
    <form onsubmit={submitTask} style="display: flex; gap: .5rem; margin: 1rem 0;">
      <input
        placeholder="Nueva tarea para {selected?.name ?? 'el proyecto'}"
        bind:value={taskName}
        required
      />
      <button type="submit">Crear tarea</button>
    </form>
    <ul>
      {#each $tasks as t (t.id)}
        <li><strong>{t.name}</strong> {t.completed ? '✓' : ''}</li>
      {/each}
    </ul>
    {#if $tasks.length === 0}<p><em>Este proyecto no tiene tareas todavía.</em></p>{/if}
  {/if}
</main>
