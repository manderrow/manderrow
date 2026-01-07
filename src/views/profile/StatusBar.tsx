import { createMemo } from "solid-js";

import { tasksArray } from "../../api/tasks";
import { t } from "../../i18n/i18n";

import TasksDialog from "../../components/TasksDialog";
import { DialogTrigger } from "../../widgets/Dialog";

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

  return (
    <div class={styles.statusBar}>
      {/* <div aria-hidden="true" class={styles.statusBarBorderCover}></div> */}
      <div class={styles.statusBar__content}>
        <TasksDialog
          trigger={
            <DialogTrigger class={styles.taskManagerBtn}>
              <span class={styles.statusBar__chunk}>
                {t("status_bar.downloads_tracker", { count: counts().downloads })}
              </span>
              <span class={styles.statusBar__chunk}>
                {t("status_bar.other_tasks_tracker", { count: counts().other })}
              </span>
            </DialogTrigger>
          }
        />
      </div>
    </div>
  );
}
