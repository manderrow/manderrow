import { invoke } from "@tauri-apps/api/core";

import { wrapInvoke } from "../api";
import { listen } from "@tauri-apps/api/event";
import { createStore, SetStoreFunction, Store } from "solid-js/store";
import { createSignal, Setter, untrack } from "solid-js";

export type Id = number;

const _tasks = new Map<Id, Task>();

export const tasks: ReadonlyMap<Id, Task> = _tasks;

export type Listener = (event: TaskEvent) => void;

interface BaseMetadata {
  title: string;
  kind: Kind;
  progress_unit: ProgressUnit;
}

export type AggregateMetadata = BaseMetadata & { kind: Kind.Aggregate };
export type DownloadMetadata = BaseMetadata & { kind: Kind.Download; url: string };
export type OtherMetadata = BaseMetadata & { kind: Kind.Other };

export type Metadata = AggregateMetadata | DownloadMetadata | OtherMetadata;

export interface Progress {
  completed: number;
  total: number;
}

const SET_METADATA = Symbol();
const SET_STATUS = Symbol();
const SET_PROGRESS = Symbol();

export function initProgress(): Progress {
  return {
    completed: 0,
    total: 0,
  };
}

export function createProgressProxyStore(): [Progress, Setter<Store<Progress>>] {
  const [progress, setProgress] = createSignal<Store<Progress>>();
  return [
    Object.freeze({
      get completed() {
        return progress()?.completed ?? 0;
      },
      get total() {
        return progress()?.total ?? 0;
      },
    }),
    setProgress as Setter<Store<Progress>>,
  ];
}

export class Task {
  readonly metadata: Store<Metadata>;
  readonly status: Store<Status>;
  readonly progress: Store<Progress>;
  readonly dependencies: Store<Id[]>;

  readonly [SET_METADATA]: SetStoreFunction<Metadata>;
  readonly [SET_STATUS]: SetStoreFunction<Status>;
  readonly [SET_PROGRESS]: SetStoreFunction<Progress>;
  readonly setDependencies: SetStoreFunction<Id[]>;

  listeners: Listener[];

  constructor(initialMetadata: Metadata, initialStatus: Status, listeners: Listener[]) {
    const [metadata, setMetadata] = untrack(() => createStore(initialMetadata));
    this.metadata = metadata;
    this[SET_METADATA] = setMetadata;

    const [status, setStatus] = untrack(() => createStore(initialStatus));
    this.status = status;
    this[SET_STATUS] = setStatus;

    const [progress, setProgress] = untrack(() => createStore(initProgress()));
    this.progress = progress;
    this[SET_PROGRESS] = setProgress;

    const [dependencies, setDependencies] = untrack(() => createStore<Id[]>([]));
    this.dependencies = dependencies;
    this.setDependencies = setDependencies;

    this.listeners = listeners;
  }

  get isComplete(): boolean {
    const status = this.status.status;
    return status === "Success" || status === "Failed" || status === "Cancelled";
  }
}

export enum Kind {
  Aggregate = "Aggregate",
  Download = "Download",
  Other = "Other",
}

export enum ProgressUnit {
  Bytes = "Bytes",
  Other = "Other",
}

export type Status = { status: "Unstarted" } | { status: "Running" } | { status: "Cancelling" } | DropStatus;

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

interface TaskDependencyEvent {
  id: Id;
  dependency: Id;
}

interface TaskDroppedEvent {
  id: Id;
  status: DropStatus;
}

export type TaskEvent =
  | (Omit<TaskCreatedEvent, "id"> & { event: "created"; progress: Store<Progress> })
  | (Omit<TaskProgressEvent, "id"> & { event: "progress" })
  | (Omit<TaskDependencyEvent, "id"> & { event: "dependency" })
  | (Omit<TaskDroppedEvent, "id"> & { event: "dropped" });

listen<TaskCreatedEvent>("task_created", (event) => {
  const { id, metadata } = event.payload;
  let task = _tasks.get(id);
  if (task === undefined) {
    _tasks.set(id, new Task(metadata, { status: "Running" }, []));
  } else {
    if (task.status.status !== "Unstarted") {
      console.error(`Duplicate id ${id} in TaskCreatedEvent`, metadata);
      return;
    }
    task[SET_METADATA](metadata);
    task[SET_STATUS]({ status: "Running" });
    notifyTaskListeners(task, { ...event.payload, event: "created", progress: task.progress });
  }
});

listen<TaskProgressEvent>("task_progress", (event) => {
  const task = _tasks.get(event.payload.id);
  if (task !== undefined) {
    // enable only when you need it.
    // console.log("Received progress update", event.payload.progress);
    task[SET_PROGRESS](event.payload.progress);
    notifyTaskListeners(task, { ...event.payload, event: "progress" });
  }
});

listen<TaskDependencyEvent>("task_dependency", (event) => {
  const task = _tasks.get(event.payload.id);
  if (task !== undefined) {
    task.setDependencies(task.dependencies.length, event.payload.dependency);
    if (task.metadata.kind === "Aggregate") {
      const dependency = event.payload.dependency;
      registerTaskListener(dependency, function handler(event) {
        if (event.event === "created") {
          if (event.metadata.progress_unit !== task.metadata.progress_unit) {
            // Units don't match. We don't want to handle this.
            unregisterTaskListener(dependency, handler);
          }
        } else if (event.event === "progress") {
          // TODO: don't redo it all
          let completed = event.progress.completed;
          let total = event.progress.total;
          for (const id of task.dependencies) {
            if (id !== dependency) {
              const subTask = tasks.get(id);
              if (subTask === undefined) continue;
              completed += subTask.progress.completed;
              total += subTask.progress.total;
            }
          }

          task[SET_PROGRESS]({
            completed,
            total,
          });
        }
      });
    }
    notifyTaskListeners(task, { ...event.payload, event: "dependency" });
  }
});

listen<TaskDroppedEvent>("task_dropped", (event) => {
  const task = _tasks.get(event.payload.id);
  if (task !== undefined) {
    task[SET_STATUS](event.payload.status);
    notifyTaskListeners(task, { ...event.payload, event: "dropped" });
    task.listeners = Object.freeze<Listener[]>([]) as Listener[];
  }
});

async function notifyTaskListeners(task: Task, event: TaskEvent) {
  task.listeners.forEach((listener) => listener(event));
}

export function registerTaskListener(id: Id, listener: Listener) {
  const task = _tasks.get(id);
  if (task === undefined) {
    _tasks.set(
      id,
      new Task(
        {
          title: "",
          kind: Kind.Other,
          progress_unit: ProgressUnit.Other,
        },
        { status: "Unstarted" },
        [listener],
      ),
    );
  } else {
    task.listeners.push(listener);
  }
}

export function unregisterTaskListener(id: Id, listener: Listener) {
  const task = _tasks.get(id);
  if (task !== undefined) {
    const i = task.listeners.indexOf(listener);
    if (i !== -1) {
      task.listeners.splice(i, 1);
    }
  }
}

export async function allocateTask(): Promise<Id> {
  return await wrapInvoke(() => invoke("allocate_task"));
}

export async function invokeWithListener<R>(listener: Listener, f: (taskId: Id) => Promise<R>): Promise<R> {
  const taskId = await allocateTask();
  registerTaskListener(taskId, listener);
  return await wrapInvoke(() => f(taskId));
}

export async function cancelTask(id: Id): Promise<void> {
  return await wrapInvoke(() => invoke("cancel_task", { id }));
}
