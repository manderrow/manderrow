import { createEffect, createSignal, createUniqueId, JSX, Show } from "solid-js";

import { faCaretDown } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";

import styles from "./TogglableDropdown.module.css";
import { FloatingElement } from "./FloatingElement";
import { UseFloatingOptions } from "solid-floating-ui";
import { flip, offset, OffsetOptions, shift } from "@floating-ui/dom";

export interface TogglableDropdownOptions {
  label: string;
  labelClass?: JSX.HTMLAttributes<HTMLElement>["class"];
  dropdownClass?: JSX.HTMLAttributes<HTMLElement>["class"];
  buttonId?: string;
  children: JSX.Element;
  dropdownOptions?: UseFloatingOptions<HTMLElement, HTMLElement>;
  offset?: OffsetOptions;
}

export default function TogglableDropdown(options: TogglableDropdownOptions) {
  const id = createUniqueId();
  const [open, setOpen] = createSignal(false);
  const [dropdownElement, setDropdownElement] = createSignal<HTMLElement>();

  let dropdownContainer!: HTMLDivElement;

  createEffect(() => {
    if (open()) dropdownContainer.focus();
  });

  return (
    <FloatingElement
      ref={setDropdownElement}
      content={
        <Show when={open()}>
          <div
            class={options.dropdownClass || styles.toggleDefault}
            id={id}
            on:focusout={(event) => {
              if (event.relatedTarget != null) {
                if (event.relatedTarget instanceof HTMLElement && event.relatedTarget.dataset.labelBtn === id) {
                  return; // don't fire here if focus is moved to the toggle button, let it close through its click handler
                }
              }
              if (dropdownElement()!.matches(":focus-within")) return;

              setOpen(false);
            }}
            tabindex="0"
            ref={dropdownContainer}
          >
            {options.children}
          </div>
        </Show>
      }
      options={{
        middleware: [flip(), shift(), offset(options.offset)],
        ...options.dropdownOptions,
      }}
    >
      <button
        type="button"
        id={options.buttonId}
        classList={{ [styles.toggle]: true, [options.labelClass || styles.labelDefault]: true }}
        role="checkbox"
        data-label-btn={id}
        aria-checked={open()}
        on:click={() => setOpen((checked) => !checked)}
        tabindex="-1"
      >
        <Fa icon={faCaretDown} class={styles.toggle__icon} />
        {options.label}
      </button>
    </FloatingElement>
  );
}
