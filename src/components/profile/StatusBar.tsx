import { Show, createMemo, createSignal } from "solid-js";

import { tasksArray } from "../../api/tasks";

import TasksDialog from "../global/TasksDialog";

import styles from "./StatusBar.module.css";

export default function StatusBar() {
  const counts = createMemo(
    () => {
      let downloads = 0;
      let other = 0;
      for (const task of tasksArray()) {
        if (task.isComplete) continue;
        if (task.status.status === "Unstarted") continue;
        if (task.metadata.kind === "Download") {
          downloads++;
        } else {
          other++;
        }
      }
      return { downloads, other };
    },
    { equals: false },
  );

  const [tasksDialogOpen, setTasksDialogOpen] = createSignal(false);

  return (
    <div class={styles.statusBar}>
      <button class={styles.taskManagerBtn} on:click={() => setTasksDialogOpen(true)}>
        <span class={styles.statusBar__chunk}>{counts().downloads} downloads</span>
        <span class={styles.statusBar__chunk}>{counts().other} other tasks</span>
      </button>

      <Show when={tasksDialogOpen()}>
        <TasksDialog onDismiss={() => setTasksDialogOpen(false)} />
      </Show>
    </div>
  );
}
