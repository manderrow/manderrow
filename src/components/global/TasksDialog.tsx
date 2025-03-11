import { For, Show } from "solid-js";

import { DefaultDialog, DismissCallback } from "./Dialog";
import { ProgressUnit, tasks } from "../../api/tasks";
import { humanizeFileSize } from "../../utils";
import { clearCache } from "../../api/installing";
import { SimpleAsyncButton } from "./AsyncButton";

import styles from "./TasksDialog.module.css";
import { SimpleProgressIndicator } from "./Progress";

export default function TasksDialog(props: { onDismiss: DismissCallback }) {
  return (
    <>
      <DefaultDialog onDismiss={props.onDismiss}>
        <div class={styles.tasks}>
          <div class={styles.header}>
            <h2>Tasks</h2>
            <SimpleAsyncButton onClick={clearCache}>Clear Cache</SimpleAsyncButton>
          </div>

          <ul>
            <For each={Array.from(tasks.entries())}>
              {(e) => {
                const [id, task] = e;
                return (
                  <Show when={task.status.status !== "Unstarted"}>
                    <li>
                      <h3>
                        <span>{id}</span>. {task.metadata.title}
                      </h3>
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
                          when={task.isComplete && task.progress.completed_progress === task.progress.total_progress}
                          fallback={
                            <p>
                              <span>{humanizeFileSize(task.progress.completed_progress)}</span> /{" "}
                              <span>{humanizeFileSize(task.progress.total_progress)}</span>
                            </p>
                          }
                        >
                          <p>{humanizeFileSize(task.progress.completed_progress)}</p>
                        </Show>
                      </Show>
                    </li>
                  </Show>
                );
              }}
            </For>
          </ul>
        </div>
      </DefaultDialog>
    </>
  );
}
