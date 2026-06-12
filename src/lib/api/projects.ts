import { invoke } from '@tauri-apps/api/core';
import type { ProjectDto } from '$lib/bindings/ProjectDto';

export function createProject(name: string, color: string | null): Promise<ProjectDto> {
  return invoke<ProjectDto>('create_project', { name, color });
}

export function listProjects(): Promise<ProjectDto[]> {
  return invoke<ProjectDto[]>('list_projects');
}
