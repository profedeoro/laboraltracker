import { writable } from 'svelte/store';
import type { TimeSessionDto } from '$lib/bindings/TimeSessionDto';
import * as api from '$lib/api/timer';

/** Sesión actualmente en curso, o null si el cronómetro está parado. */
export const running = writable<TimeSessionDto | null>(null);

export async function refreshRunning(): Promise<void> {
  running.set(await api.runningTimer());
}

export async function start(taskId: string): Promise<void> {
  running.set(await api.startTimer(taskId));
}

export async function stop(): Promise<void> {
  await api.stopTimer();
  running.set(null);
}
