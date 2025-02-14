import { createSignal, createUniqueId, For, Show } from "solid-js";
import styles from "./SelectDropdown.module.css";
import { faCaretDown, faCheck } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import Dropdown, { DropdownOptions } from "./Dropdown";
import { createStore } from "solid-js/store";

interface Option<T> {
  value: T;
  selected?: boolean;
}

type LabelTextValue = {
  labelText: "value";
};
type LabelTextPreset = {
  labelText: "preset";
  preset: string;
};
type LabelText = LabelTextValue | LabelTextPreset;

interface SelectDropdownOptions<T> extends Omit<DropdownOptions, "children"> {
  label: LabelText;
  options: Record<string, Option<T>>;
  onChanged: (value: T, selected: boolean) => void;
  multiselect?: boolean;
}

export default function SelectDropdown<T>(options: SelectDropdownOptions<T>) {
  const [open, setOpen] = createSignal(false);
  const [selected, setSelected] = createStore(options.options);
  const [labelValue, setLabelValue] = createSignal(
    options.label.labelText === "preset"
      ? options.label.preset
      : Object.entries(options.options).find(([key, value]) => value.selected)?.[0] ?? "Select...",
  );

  return (
    <div
      classList={{ [styles.container]: true, [options.class || ""]: true }}
      on:focusout={(event) => {
        console.log(event);
        if (event.relatedTarget != null) {
          if (!(event.relatedTarget instanceof HTMLElement)) return;
          if (event.relatedTarget.closest("." + styles.container) != null) return;
        }
        setOpen(false);
      }}
    >
      <button
        type="button"
        class={styles.toggle}
        data-btn
        role="checkbox"
        aria-checked={open()}
        on:click={() => setOpen((checked) => !checked)}
      >
        <Fa icon={faCaretDown} class={styles.toggle__icon} />
        {labelValue()}
      </button>
      <Show when={open()}>
        <Dropdown align={options.align} class={styles.dropdown}>
          <ul class={styles.options}>
            <For each={Object.entries(options.options)}>
              {([key, option]) => {
                let ref!: HTMLLIElement;

                function onSelect() {
                  // use the cached value here, so the action performed by the
                  // UI is **never** out of sync with the displayed value.
                  const isSelected = ref.ariaChecked! !== "true";
                  setSelected(key, "selected", isSelected);
                  options.onChanged(option.value, isSelected);

                  if (!options.multiselect && isSelected) {
                    for (const other in options.options) {
                      if (options.options[other].value !== option.value) {
                        setSelected(other, "selected", false);
                      }
                    }
                  }
                  if (options.label.labelText === "value") setLabelValue(key);
                }

                return (
                  <li
                    tabIndex={0}
                    role="checkbox"
                    class={styles.option}
                    aria-checked={selected[key].selected}
                    on:click={onSelect}
                    on:keydown={(event) => {
                      if (event.key === "Enter" || event.key === " ") onSelect();
                    }}
                    ref={ref}
                  >
                    <Fa icon={faCheck} class={styles.option__check} />
                    {key}
                  </li>
                );
              }}
            </For>
          </ul>
        </Dropdown>
      </Show>
    </div>
  );
}
