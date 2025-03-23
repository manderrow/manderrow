import { createSignal, createUniqueId, Show } from "solid-js";

import { faCaretDown } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import Dropdown, { DropdownOptions } from "./Dropdown";

import styles from "./TogglableDropdown.module.css";

export interface TogglableDropdownOptions extends Omit<DropdownOptions, "ref"> {
  label: string;
  labelClass?: string;
  dropdownClass?: string;
  buttonId?: string;
}

export default function TogglableDropdown(options: TogglableDropdownOptions) {
  const id = createUniqueId();
  const [open, setOpen] = createSignal(false);
  const [dropdownElement, setDropdownElement] = createSignal<HTMLElement>();

  return (
    <div
      id={id}
      classList={{ [styles.container]: true, [options.class || ""]: true }}
      on:focusout={(event) => {
        if (event.relatedTarget != null) {
          if (!(event.relatedTarget instanceof HTMLElement)) return;
          if (event.relatedTarget?.closest("#" + id) != null || dropdownElement()?.contains(event.relatedTarget)) {
            // keep it focused. TODO: detect clicks outside the dropdown element from inside Dropdown and allow focusing its content
            event.target.focus();
            return;
          }
        }
        setOpen(false);
      }}
    >
      <button
        type="button"
        id={options.buttonId}
        class={styles.toggle}
        data-btn
        classList={{ [styles.label]: true, [options.labelClass || styles.labelDefault]: true }}
        role="checkbox"
        aria-checked={open()}
        on:click={() => setOpen((checked) => !checked)}
        tabindex="-1"
      >
        <Fa icon={faCaretDown} class={styles.toggle__icon} />
        {options.label}
      </button>
      <Show when={open()}>
        <Dropdown align={options.align} class={options.dropdownClass} ref={setDropdownElement}>
          {options.children}
        </Dropdown>
      </Show>
    </div>
  );
}
