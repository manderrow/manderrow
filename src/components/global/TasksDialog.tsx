import { createSignal, createUniqueId, For, Show } from "solid-js";

import { DefaultDialog, DismissCallback } from "./Dialog";
import { ProgressUnit, Task, tasks } from "../../api/tasks";
import { humanizeFileSize } from "../../utils";
import { clearCache } from "../../api/installing";
import { SimpleAsyncButton } from "./AsyncButton";

import styles from "./TasksDialog.module.css";
import { SimpleProgressIndicator } from "./Progress";

export default function TasksDialog(props: { onDismiss: DismissCallback }) {
  const [showingActive, setShowingActive] = createSignal(true);

  const radioGroupName = createUniqueId();
  const activeRadioId = createUniqueId();
  const completedRadioId = createUniqueId();

  return (
    <>
      <DefaultDialog onDismiss={props.onDismiss}>
        <div class={styles.tasks}>
          <div class={styles.header}>
            <h2>Tasks</h2>
            <SimpleAsyncButton onClick={clearCache}>Clear Cache</SimpleAsyncButton>
          </div>

          <input type="radio" style="display:none" name={radioGroupName} id={activeRadioId} on:change={() => setShowingActive(true)} checked={showingActive()} />
          <label for={activeRadioId}><h3>Active</h3></label>
          <Show when={showingActive()}>
            <TaskList where={(task) => !task.isComplete} />
          </Show>

          <input type="radio" style="display:none" name={radioGroupName} id={completedRadioId} on:change={() => setShowingActive(false)} checked={!showingActive()} />
          <label for={completedRadioId}><h3>Completed</h3></label>
          <Show when={!showingActive()}>
            <TaskList where={(task) => task.isComplete} />
          </Show>
        </div>
      </DefaultDialog>
    </>
  );
}
function TaskList(props: { where: (task: Task) => boolean }) {
  return (
    <ul>
      <For each={Array.from(tasks.entries())}>
        {(e) => {
          const [id, task] = e;
          return (
            <Show when={task.status.status !== "Unstarted" && props.where(task)}>
              <li>
                <h4>
                  <span>{id}</span>. {task.metadata.title}
                </h4>
                <p>
                  Kind: <span>{task.metadata.kind}</span>
                </p>
                <p>
                  Status: <span>{task.status.status}</span>
                </p>
                <Show when={!task.isComplete}>
                  <SimpleProgressIndicator progress={task.progress} />
                </Show>
                <Show when={task.metadata.progress_unit === ProgressUnit.Bytes}>
                  <Show
                    when={task.isComplete && task.progress.completed === task.progress.total}
                    fallback={
                      <p>
                        <span>{humanizeFileSize(task.progress.completed)}</span> /{" "}
                        <span>{humanizeFileSize(task.progress.total)}</span>
                      </p>
                    }
                  >
                    <p>{humanizeFileSize(task.progress.completed)}</p>
                  </Show>
                </Show>
              </li>
            </Show>
          );
        }}
      </For>
    </ul>
  );
}
