import { createUniqueId } from "solid-js";
import Fa from "solid-fa";
import { faCheck } from "@fortawesome/free-solid-svg-icons";

import styles from "./Checkbox.module.css";

export default function Checkbox(props: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  labelClass?: string;
  iconContainerClass?: string;
}) {
  const id = createUniqueId();

  return (
    <label for={id} class={`${styles.switch} ${props.labelClass || styles.switchDefault}`}>
      <input
        type="checkbox"
        id={id}
        checked={props.checked}
        class="phantom"
        tabIndex={-1}
        onChange={(e) => props.onChange(e.currentTarget.checked)}
      />
      <div class={`${styles.iconContainer} ${props.iconContainerClass}`}>
        <Fa icon={faCheck} class={styles.switch__icon} />
      </div>
    </label>
  );
}
