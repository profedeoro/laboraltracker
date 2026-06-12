import { writable } from 'svelte/store';
import type { TaskDto } from '$lib/bindings/TaskDto';
import * as api from '$lib/api/tasks';

export const tasks = writable<TaskDto[]>([]);
export const selectedProjectId = writable<string | null>(null);

export async function loadTasks(projectId: string): Promise<void> {
  selectedProjectId.set(projectId);
  tasks.set(await api.listTasks(projectId));
}

export async function addTask(projectId: string, name: string): Promise<void> {
  await api.createTask(projectId, name);
  await loadTasks(projectId);
}
