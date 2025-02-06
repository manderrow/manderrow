import { createSignal, createUniqueId, For, Show } from "solid-js";
import styles from "./SelectDropdown.module.css";
import { faCaretDown } from "@fortawesome/free-solid-svg-icons/faCaretDown";
import Fa from "solid-fa";
import Dropdown, { DropdownOptions } from "./Dropdown";
import { faCheck } from "@fortawesome/free-solid-svg-icons";

interface Option<T> {
  name: string;
  value: T,
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
  options: Option<T>[];
  selected: (value: T) => boolean,
  onChanged: (value: T, selected: boolean) => void,
  /**
   * In some cases, it would be better to set this to false and handle manually in `onChanged`.
   */
  multiselect?: boolean;
}

export default function SelectDropdown<T>(options: SelectDropdownOptions<T>) {
  const id = createUniqueId();
  const [open, setOpen] = createSignal(false);
  const [labelValue, setLabelValue] = createSignal(
    options.label.labelText === "preset"
      ? options.label.preset
      : options.options.find((option) => options.selected(option.value))?.name ?? options.options[0].name ?? "Select..."
  );

  return (
    <div class={styles.container}>
      <label for={id} class={styles.label}>
        <Fa icon={faCaretDown} rotate={open() ? 180 : 0} />
        {labelValue()}
        <input type="checkbox" name="Toggle" id={id} class="phantom" onInput={(event) => setOpen(event.target.checked)} />
      </label>
      <Show when={open()}>
        <Dropdown align={options.align}>
          <ul>
            <For each={options.options}>
              {option => {
                let ref!: HTMLLIElement;

                function onSelect() {
                  // use the cached value here, so the action performed by the
                  // UI is **never** out of sync with the displayed value.
                  options.onChanged(option.value, ref.ariaChecked! !== "true");

                  if (!options.multiselect) {
                    for (const other of options.options) {
                      if (other.value !== option.value) {
                        options.onChanged(other.value, false);
                      }
                    }
                  }
                  if (options.label.labelText === "value") setLabelValue(option.name);
                }

                return (
                  <li
                    tabIndex={0}
                    role="checkbox"
                    class={styles.option}
                    aria-checked={options.selected(option.value)}
                    on:click={onSelect}
                    on:keydown={(event) => {
                      if (event.key === "Enter" || event.key === " ") onSelect();
                    }}
                    ref={ref}
                  >
                    <Fa icon={faCheck} class={styles.option__check} />
                    {option.name}
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
