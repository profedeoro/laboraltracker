import { writable } from 'svelte/store';
import type { ProjectDto } from '$lib/bindings/ProjectDto';
import * as api from '$lib/api/projects';

export const projects = writable<ProjectDto[]>([]);

export async function refreshProjects(): Promise<void> {
  projects.set(await api.listProjects());
}

export async function addProject(name: string, color: string | null): Promise<void> {
  await api.createProject(name, color);
  await refreshProjects();
}
