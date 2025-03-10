import { invoke } from "@tauri-apps/api/core";

import { wrapInvoke } from "../api";
import { listen } from "@tauri-apps/api/event";

export type Id = number;

const _tasks = new Map<Id, Task>();

export const tasks: ReadonlyMap<Id, Task> = _tasks;

export interface Metadata {
  title: string;
  kind: Kind;
}

export interface Progress {
  completed_steps: number;
  total_steps: number;
  progress: number;
}

export interface Task extends Metadata {
  status: Status;
  progress: Progress;
}

export enum Kind {
  Download = "Download",
  Other = "Other",
}

export type Status = { status: "Running" } | { status: "Cancelling" } | DropStatus;

export type DropStatus =
  | { status: "Success" }
  | { status: "Failed"; error: string }
  | { status: "Cancelled"; direct: boolean };

interface TaskCreatedEvent {
  id: Id;
  metadata: Metadata;
}

interface TaskProgressEvent {
  id: Id;
  progress: Progress;
}

interface TaskDroppedEvent {
  id: Id;
  status: DropStatus;
}

listen<TaskCreatedEvent>("task_created", (event) => {
  _tasks.set(event.payload.id, {
    ...event.payload.metadata,
    status: {
      status: "Running",
    },
    progress: {
      completed_steps: 0,
      total_steps: 0,
      progress: 0,
    },
  });
});

listen<TaskProgressEvent>("task_dropped", (event) => {
  const task = _tasks.get(event.payload.id);
  if (task !== undefined) {
    task.progress = event.payload.progress;
  }
});

listen<TaskDroppedEvent>("task_dropped", (event) => {
  const task = _tasks.get(event.payload.id);
  if (task !== undefined) {
    task.status = event.payload.status;
  }
});

export async function cancelTask(id: Id): Promise<void> {
  return await wrapInvoke(() => invoke("cancel_task", { id }));
}
