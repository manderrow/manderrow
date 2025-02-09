import { createSignal, createUniqueId, JSX, Show } from "solid-js";

import { faCaretDown } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import Dropdown, { Alignment, DropdownOptions } from "./Dropdown";

import styles from "./TogglableDropdown.module.css";

interface TogglableDropdownOptions extends DropdownOptions {
  label: string;
  labelClass?: string;
}

export default function TogglableDropdown(options: TogglableDropdownOptions) {
  const id = createUniqueId();

  const [open, setOpen] = createSignal(false);

  return (
    <div class={styles.container}>
      <label for={id} classList={{ [styles.label]: true, [options.labelClass || styles.labelDefault]: true }}>
        <Fa icon={faCaretDown} rotate={open() ? 180 : 0} />
        {options.label}
        <input type="checkbox" name="Toggle" id={id} class="phantom" onInput={(event) => setOpen(event.target.checked)} />
      </label>
      <Show when={open()}>
        <Dropdown align={options.align}>{options.children}</Dropdown>
      </Show>
    </div>
  );
}
