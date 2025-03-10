import { For } from "solid-js";
import { DefaultDialog, DismissCallback } from "./Dialog";
import styles from "./TasksDialog.module.css";
import { tasks } from "../../api/tasks";

export default function TasksDialog(props: { onDismiss: DismissCallback }) {
  return (
    <>
      <DefaultDialog onDismiss={props.onDismiss}>
        <div class={styles.tasks}>
          <h2>Tasks</h2>

          <ul>
            <For each={Array.from(tasks.entries())}>
              {(e) => {
                const [id, task] = e;
                return (
                  <li>
                    <h3>
                      <span>{id}</span>. {task.title}
                    </h3>
                    <p>
                      Kind: <span>{task.kind}</span>
                    </p>
                    <p>
                      Status: <span>{task.status.status}</span>
                    </p>
                  </li>
                );
              }}
            </For>
          </ul>
        </div>
      </DefaultDialog>
    </>
  );
}
