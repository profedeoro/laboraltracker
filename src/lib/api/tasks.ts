import { invoke } from '@tauri-apps/api/core';
import type { TaskDto } from '$lib/bindings/TaskDto';

export function createTask(projectId: string, name: string): Promise<TaskDto> {
  return invoke<TaskDto>('create_task', { projectId, name });
}

export function listTasks(projectId: string): Promise<TaskDto[]> {
  return invoke<TaskDto[]>('list_tasks', { projectId });
}
