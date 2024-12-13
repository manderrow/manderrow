import { JSX } from "solid-js";
import { Portal } from "solid-js/web";

import styles from "./Dialog.module.css";

export default function Dialog(props: { children: JSX.Element }) {
  return (
    <Portal>
      <div class={styles.dialog}>
        <div>{props.children}</div>
      </div>
    </Portal>
  );
}
