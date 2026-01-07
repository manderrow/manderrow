import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Accessor, createMemo, createSignal, Setter } from "solid-js";
import { createStore, SetStoreFunction, Store } from "solid-js/store";

import { wrapInvoke } from "./api";
import { callWithErrorStack } from "../utils/utils";

export type Id = number;

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
  | { status: "Success"; success?: "Cached" }
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

const METADATA_OPTIONS = { name: "Task.metadata" };
const STATUS_OPTIONS = { name: "Task.status" };
const PROGRESS_OPTIONS = { name: "Task.progress" };
const DEPENDENCIES_OPTIONS = { name: "Task.dependencies" };
const IS_COMPLETE_OPTIONS = { name: "Task.isComplete" };

export class Task {
  readonly metadata: Store<Metadata>;
  readonly status: Store<Status>;
  readonly progress: Store<Progress>;
  readonly dependencies: Store<Id[]>;
  readonly #_isComplete: Accessor<boolean>;

  readonly _setMetadata: SetStoreFunction<Metadata>;
  readonly _setStatus: SetStoreFunction<Status>;
  readonly _setProgress: SetStoreFunction<Progress>;
  readonly _setDependencies: SetStoreFunction<Id[]>;

  listeners: Listener[];
  /**
   * Used by notifyTaskListeners and unregisterTaskListener.
   */
  _listenerIndex: number;

  constructor(initialMetadata: Metadata, initialStatus: Status, listeners: Listener[]) {
    const [metadata, setMetadata] = createStore(initialMetadata, METADATA_OPTIONS);
    this.metadata = metadata;
    this._setMetadata = setMetadata;

    const [status, setStatus] = createStore(initialStatus, STATUS_OPTIONS);
    this.status = status;
    this._setStatus = setStatus;

    const [progress, setProgress] = createStore(initProgress(), PROGRESS_OPTIONS);
    this.progress = progress;
    this._setProgress = setProgress;

    const [dependencies, setDependencies] = createStore<Id[]>([], DEPENDENCIES_OPTIONS);
    this.dependencies = dependencies;
    this._setDependencies = setDependencies;

    this.#_isComplete = createMemo(() => {
      const status = this.status.status;
      return status === "Success" || status === "Failed" || status === "Cancelled";
    }, IS_COMPLETE_OPTIONS);

    this.listeners = listeners;
    this._listenerIndex = 0;
  }

  get isComplete() {
    return this.#_isComplete();
  }
}

const _tasks = new Map<Id, Task>();
const [_tasksSignal, _setTasksSignalValue] = createSignal<ReadonlyMap<Id, Task>>(_tasks, { equals: false });

export const tasks = _tasksSignal;
export const tasksArray: Accessor<readonly Task[]> = createMemo(() => Array.from(tasks().values()), { equals: false });

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
    _setTasksSignalValue(_tasks);
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
    // adjust so that if removed during iteration, subsequent listeners are not skipped
    if (task._listenerIndex >= i) {
      task._listenerIndex--;
    }
  }
}

/**
 * @returns the id of the newly allocated task
 */
export async function allocateTask(): Promise<Id> {
  return await wrapInvoke(() => invoke("allocate_task"));
}

/**
 * Invokes `f` with the id of a newly allocated task that has `listener` registered.
 *
 * @param listener a listener for task events
 * @param f the callback to invoke
 * @returns the return value of the callback
 */
export function invokeWithListener<R>(listener: Listener, f: (taskId: Id) => Promise<R>): Promise<R> {
  return callWithErrorStack(async () => {
    const taskId = await allocateTask();
    registerTaskListener(taskId, listener);
    return await wrapInvoke(() => f(taskId));
  });
}

/**
 * Cancels the task `id`, returning without waiting for the cancellation to complete.
 */
export async function cancelTask(id: Id): Promise<void> {
  return await wrapInvoke(() => invoke("cancel_task", { id }));
}

async function notifyTaskListeners(task: Task, event: TaskEvent) {
  for (task._listenerIndex = 0; task._listenerIndex < task.listeners.length; task._listenerIndex++) {
    const listener = task.listeners[task._listenerIndex];
    listener(event);
  }
}

// Rust event listeners

listen<TaskCreatedEvent>("task_created", (event) => {
  const { id, metadata } = event.payload;
  let task = _tasks.get(id);
  if (task === undefined) {
    _tasks.set(id, new Task(metadata, { status: "Running" }, []));
    _setTasksSignalValue(_tasks);
  } else {
    if (task.status.status !== "Unstarted") {
      console.error(`Duplicate id ${id} in TaskCreatedEvent`, metadata);
      return;
    }
    task._setMetadata(metadata);
    task._setStatus({ status: "Running" });
    notifyTaskListeners(task, { ...event.payload, event: "created", progress: task.progress });
  }
});

listen<TaskProgressEvent>("task_progress", (event) => {
  const task = _tasks.get(event.payload.id);
  if (task !== undefined) {
    // enable only when you need it.
    // console.log("Received progress update", event.payload.progress);
    task._setProgress(event.payload.progress);
    notifyTaskListeners(task, { ...event.payload, event: "progress" });
  }
});

listen<TaskDependencyEvent>("task_dependency", (event) => {
  const task = _tasks.get(event.payload.id);
  if (task !== undefined) {
    task._setDependencies(task.dependencies.length, event.payload.dependency);
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
              const subTask = _tasks.get(id);
              if (subTask === undefined) continue;
              completed += subTask.progress.completed;
              total += subTask.progress.total;
            }
          }

          task._setProgress({
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
    task._setStatus(event.payload.status);
    notifyTaskListeners(task, { ...event.payload, event: "dropped" });
    task.listeners = Object.freeze<Listener[]>([]) as Listener[];
  }
});
