import { faCopy, faDownload, faListCheck } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { For, Show, createSelector, createSignal } from "solid-js";

import { clearCache } from "../../api/installing";
import { DownloadMetadata, Kind, ProgressUnit, tasksArray } from "../../api/tasks";
import { t } from "../../i18n/i18n.ts";
import { humanizeFileSize, roundedNumberFormatter } from "../../utils";


import { SimpleAsyncButton } from "./AsyncButton";
import Dialog, { DismissCallback } from "./Dialog";
import { SimpleProgressIndicator } from "./Progress";
import TabRenderer, { Tab, TabContent } from "./TabRenderer.tsx";

import styles from "./TasksDialog.module.css";

type TabId = "active" | "completed";

export default function TasksDialog(props: { onDismiss: DismissCallback }) {
  const [tab, setTab] = createSignal<TabId>("active");
  const isCurrentTab = createSelector(tab);

  const tabs: readonly Tab<TabId>[] = [
    {
      id: "active",
      name: t("global.task_manager.active_tab_name"),
      component: () => <TaskList active />,
    },
    {
      id: "completed",
      name: t("global.task_manager.completed_tab_name"),
      component: () => <TaskList />,
    },
  ];

  return (
    <Dialog onDismiss={props.onDismiss}>
      <div class={styles.tasks}>
        <div class={styles.tasks__header}>
          <h2>{t("global.task_manager.title")}</h2>
        </div>

        <div>
          <TabRenderer<TabId>
            id="tasks"
            tabs={tabs}
            styles={{
              tabs: {
                container: styles.tabs,
                list: styles.tabs__list,
                list__item: styles.tabs__tab,
                list__itemActive: styles.tabs__tabActive,
              },
            }}
            setter={(tab) => setTab(tab.id)}
          />

          <div class={styles.tabs__trailing}>
            <SimpleAsyncButton progress onClick={clearCache}>
              Clear Cache
            </SimpleAsyncButton>
          </div>
        </div>

        <TabContent isCurrentTab={isCurrentTab} tabs={tabs} />
      </div>
    </Dialog>
  );
}

function TaskKindIcon(kind: Kind) {
  switch (kind) {
    case Kind.Aggregate:
      return <Fa icon={faListCheck} />;
    case Kind.Download:
      return <Fa icon={faDownload} />;
    case Kind.Other:
      return <></>;
  }
}

function TaskList(props: { active?: boolean }) {
  return (
    <ul class={styles.list}>
      <For
        each={tasksArray().filter((task) => task.isComplete != !!props.active && task.status.status !== "Unstarted")}
        fallback={<li>No tasks yet.</li>}
      >
        {(task) => (
          <li>
            <Show when={!task.isComplete}>
              <SimpleProgressIndicator progress={task.progress} />
            </Show>
            <div>
              <div>
                <h4>
                  {TaskKindIcon(task.metadata.kind)} {task.metadata.title}
                </h4>

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

                  <Show when={task.status.status === "Success" ? task.status.success : undefined}>
                    {(info) => <span>{info()}</span>}
                  </Show>
                </p>

                <Show when={task.metadata.kind === Kind.Download}>
                  <DownloadUrlLine url={(task.metadata as DownloadMetadata).url} />
                </Show>
              </div>
            </div>
          </li>
        )}
      </For>
    </ul>
  );
}

function DownloadUrlLine(props: { url: string }) {
  return <div class={styles.downloadUrl}>
    <a href={props.url} title={props.url}>Source</a>
    <button onClick={() => {
      navigator.clipboard.writeText(props.url);
    }}>
      <Fa icon={faCopy} />
    </button>
  </div>;
}

function ModDownloadTask() {}

function OtherTask() {}
