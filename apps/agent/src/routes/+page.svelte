<script lang="ts">
  import { onMount } from 'svelte';
  import { projects, refreshProjects, addProject } from '$lib/stores/projects';
  import { tasks, selectedProjectId, loadTasks, addTask } from '$lib/stores/tasks';
  import { running, refreshRunning, start, stop } from '$lib/stores/timer';

  let projectName = $state('');
  let taskName = $state('');
  let error = $state('');
  // Reloj de presentación: tickea cada segundo para el transcurrido en vivo.
  let now = $state(Date.now());

  onMount(() => {
    refreshProjects();
    refreshRunning();
    const id = setInterval(() => {
      now = Date.now();
    }, 1000);
    const hb = setInterval(() => {
      if ($running) import('$lib/api/timer').then((m) => m.heartbeat());
    }, 30000);
    return () => {
      clearInterval(id);
      clearInterval(hb);
    };
  });

  function formatElapsed(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 1000));
    const hh = String(Math.floor(total / 3600)).padStart(2, '0');
    const mm = String(Math.floor((total % 3600) / 60)).padStart(2, '0');
    const ss = String(total % 60).padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
  }

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

  async function startTask(taskId: string) {
    error = '';
    try {
      await start(taskId);
    } catch (err) {
      error = JSON.stringify(err);
    }
  }

  async function stopTask() {
    error = '';
    try {
      await stop();
    } catch (err) {
      error = JSON.stringify(err);
    }
  }
</script>

<main style="padding: 2rem; font-family: system-ui; max-width: 40rem;">
  <h1>LaboralTracker — Proyectos y Tareas</h1>

  {#if $running}
    <p style="background:#ecfdf5; border:1px solid #16a34a; padding:.5rem .75rem; border-radius:.375rem;">
      ⏱ Cronómetro corriendo —
      <strong style="font-variant-numeric: tabular-nums;">{formatElapsed(now - $running.startedAt)}</strong>
      <button type="button" onclick={stopTask} style="margin-left:.5rem;">Parar</button>
    </p>
  {/if}

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
        <li style="display: flex; align-items: center; gap: .5rem;">
          <strong>{t.name}</strong>
          {#if $running && $running.taskId === t.id}
            <span style="font-variant-numeric: tabular-nums; color: #16a34a;">
              {formatElapsed(now - $running.startedAt)}
            </span>
            <button type="button" onclick={stopTask}>Parar</button>
          {:else}
            <button type="button" onclick={() => startTask(t.id)}>Iniciar</button>
          {/if}
        </li>
      {/each}
    </ul>
    {#if $tasks.length === 0}<p><em>Este proyecto no tiene tareas todavía.</em></p>{/if}
  {/if}
</main>
