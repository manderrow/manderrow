import { createSignal, createUniqueId, For, JSX, Show } from "solid-js";

import Dialog, { DismissCallback } from "./Dialog";
import { ProgressUnit, Task, tasks } from "../../api/tasks";
import { humanizeFileSize, roundedNumberFormatter } from "../../utils";
import { clearCache } from "../../api/installing";
import { SimpleAsyncButton } from "./AsyncButton";

import styles from "./TasksDialog.module.css";
import { SimpleProgressIndicator } from "./Progress";
import TabRenderer from "./TabRenderer.tsx";
import { t } from "../../i18n/i18n.ts";

export default function TasksDialog(props: { onDismiss: DismissCallback }) {
  return (
    <Dialog onDismiss={props.onDismiss}>
      <div class={styles.tasks}>
        <div class={styles.tasks__header}>
          <h2>{t("global.task_manager.title")}</h2>
          {/* <SimpleAsyncButton progress onClick={clearCache}>
            Clear Cache
          </SimpleAsyncButton> */}
        </div>

        <TabRenderer
          id="tasks"
          tabs={[
            {
              id: "active",
              name: t("global.task_manager.active_tab_name"),
              component: <TaskList active />,
            },
            {
              id: "completed",
              name: t("global.task_manager.completed_tab_name"),
              component: <TaskList />,
            },
          ]}
          styles={{
            tabs: {
              list: styles.tabs,
              list__item: styles.tabs__tab,
              list__itemActive: styles.tabs__tabActive,
            },
          }}
        />
      </div>
    </Dialog>
  );
}

function TaskList(props: { active?: boolean }) {
  return (
    <ul class={styles.list}>
      <For
        each={Array.from(tasks.entries()).filter(
          ([_, task]) => task.isComplete != !!props.active && task.status.status !== "Unstarted",
        )}
        fallback={<li>No tasks yet.</li>}
      >
        {([_, task]) => (
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
                    <span>{roundedNumberFormatter.format((task.progress.completed / task.progress.total) * 100)}%</span>
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
        )}
      </For>
    </ul>
  );
}

function ModDownloadTask() {}

function OtherTask() {}
