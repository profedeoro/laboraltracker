import { invoke } from '@tauri-apps/api/core';
import type { TimeSessionDto } from '$lib/bindings/TimeSessionDto';

export function startTimer(taskId: string): Promise<TimeSessionDto> {
  return invoke<TimeSessionDto>('start_timer', { taskId });
}

export function stopTimer(): Promise<TimeSessionDto> {
  return invoke<TimeSessionDto>('stop_timer');
}

export function runningTimer(): Promise<TimeSessionDto | null> {
  return invoke<TimeSessionDto | null>('running_timer');
}
