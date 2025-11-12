import { createUniqueId, JSX } from "solid-js";
import styles from "./Checkbox.module.css";
import Fa from "solid-fa";
import { faCheck } from "@fortawesome/free-solid-svg-icons";

interface CheckboxProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  boxClass?: string;
  containerClass?: string;
  iconContainerClass?: string;
}

export default function Checkbox(props: CheckboxProps & { id?: string }) {
  const id = props.id ?? createUniqueId();

  return (
    <CheckboxWrapper id={id} containerClass={props.containerClass}>
      <CheckboxBox
        id={id}
        checked={props.checked}
        onChange={props.onChange}
        boxClass={props.boxClass}
        iconContainerClass={props.iconContainerClass}
      />
    </CheckboxWrapper>
  );
}

export function CheckboxBox(props: CheckboxProps & { id: string }) {
  return (
    <div class={`${styles.switch} ${props.boxClass || styles.boxDefault}`}>
      <input
        type="checkbox"
        id={props.id}
        checked={props.checked}
        class="phantom"
        tabIndex={-1}
        onChange={(e) => props.onChange(e.currentTarget.checked)}
      />
      <div class={`${styles.iconContainer} ${props.iconContainerClass || ""}`}>
        <Fa icon={faCheck} class={styles.switch__icon} />
      </div>
    </div>
  );
}

export function CheckboxWrapper(props: { id: string; children: JSX.Element; containerClass?: string }) {
  return (
    <label for={props.id} class={props.containerClass || ""}>
      {props.children}
    </label>
  );
}
