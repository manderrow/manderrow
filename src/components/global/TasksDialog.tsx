import { createSignal, createUniqueId, For, JSX, Show } from "solid-js";

import { DefaultDialog, DismissCallback } from "./Dialog";
import { ProgressUnit, Task, tasks } from "../../api/tasks";
import { humanizeFileSize, roundedNumberFormatter } from "../../utils";
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

          <TaskSectionHeader
            radioGroupName={radioGroupName}
            radioId={activeRadioId}
            onSelect={() => setShowingActive(true)}
            selected={showingActive()}
          >
            Active
          </TaskSectionHeader>
          <Show when={showingActive()}>
            <TaskList where={(task) => !task.isComplete} />
          </Show>

          <TaskSectionHeader
            radioGroupName={radioGroupName}
            radioId={completedRadioId}
            onSelect={() => setShowingActive(false)}
            selected={!showingActive()}
          >
            Completed
          </TaskSectionHeader>
          <Show when={!showingActive()}>
            <TaskList where={(task) => task.isComplete} />
          </Show>
        </div>
      </DefaultDialog>
    </>
  );
}

function TaskSectionHeader(props: {
  radioGroupName: string;
  radioId: string;
  onSelect: () => void;
  selected: boolean;
  children: JSX.Element;
}) {
  return (
    <>
      <input
        type="radio"
        style="display:none"
        name={props.radioGroupName}
        id={props.radioId}
        on:change={props.onSelect}
        checked={props.selected}
      />
      <label class={styles.section} for={props.radioId}>
        <h3>{props.children}</h3>
      </label>
    </>
  );
}

function TaskList(props: { where: (task: Task) => boolean }) {
  return (
    <ul class={styles.list}>
      <For each={Array.from(tasks.entries())}>
        {(e) => {
          const [id, task] = e;
          return (
            <Show when={task.status.status !== "Unstarted" && props.where(task)}>
              <li>
                <Show when={!task.isComplete}>
                  <SimpleProgressIndicator progress={task.progress} />
                </Show>
                <div>
                  <div>
                    <h4>
                      <Show when={task.metadata.kind === "Download"}>Download</Show> {task.metadata.title}
                    </h4>
                    <p>
                      status=<span>{task.status.status}</span>
                    </p>

                    <p class={styles.status_line}>
                      <Show when={task.status.status !== "Running" || task.progress.total === 0}>
                        <span>{task.status.status}</span>
                      </Show>
                      <Show when={!task.isComplete && task.progress.total !== 0}>
                        <span>
                          {roundedNumberFormatter.format((task.progress.completed / task.progress.total) * 100)}%
                        </span>
                      </Show>

                      <Show when={task.metadata.progress_unit === ProgressUnit.Bytes}>
                        <span>
                          <Show
                            when={task.isComplete && task.progress.completed === task.progress.total}
                            fallback={
                              <>
                                <span>{humanizeFileSize(task.progress.completed)}</span> /{" "}
                                <span>{humanizeFileSize(task.progress.total)}</span>
                              </>
                            }
                          >
                            {humanizeFileSize(task.progress.completed)}
                          </Show>
                        </span>
                      </Show>
                    </p>
                  </div>
                </div>
              </li>
            </Show>
          );
        }}
      </For>
    </ul>
  );
}
