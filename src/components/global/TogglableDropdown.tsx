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
    <div
      id={id}
      classList={{ [styles.container]: true, [options.class || ""]: true }}
      on:focusout={(event) => {
        if (event.relatedTarget != null) {
          if (!(event.relatedTarget instanceof HTMLElement)) return;
          if (event.relatedTarget.closest("#" + id) != null) return;
        }
        setOpen(false);
      }}
    >
      <button
        type="button"
        class={styles.toggle}
        data-btn
        classList={{ [styles.label]: true, [options.labelClass || styles.labelDefault]: true }}
        role="checkbox"
        aria-checked={open()}
        on:click={() => setOpen((checked) => !checked)}
      >
        <Fa icon={faCaretDown} class={styles.toggle__icon} />
        {options.label}
      </button>
      <Show when={open()}>
        <Dropdown align={options.align}>{options.children}</Dropdown>
      </Show>
    </div>
  );
}
