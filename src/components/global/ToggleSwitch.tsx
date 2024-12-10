import { createUniqueId } from "solid-js";
import styles from "./ToggleSwitch.module.css";

export default function ToggleSwitch(props: { name: string; on: boolean }) {
  const id = createUniqueId();

  return (
    <label for={id} class={styles.switch}>
      <input type="checkbox" name={props.name} id={id} checked={props.on} />
    </label>
  );
}
