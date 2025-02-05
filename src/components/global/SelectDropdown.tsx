import { createEffect, createSignal, createUniqueId, For, onMount, Show } from "solid-js";
import styles from "./SelectDropdown.module.css";
import { faCaretDown } from "@fortawesome/free-solid-svg-icons/faCaretDown";
import Fa from "solid-fa";
import Dropdown, { DropdownOptions } from "./Dropdown";
import { createStore, produce } from "solid-js/store";

interface Option {
  name: string;
  value: any;
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

interface SelectDropdownOptions extends Omit<DropdownOptions, "children"> {
  label: LabelText;
  options: Option[];
  onChange: (selectedValues: any[]) => void;
  multiselect?: boolean;
}

export default function SelectDropdown(options: SelectDropdownOptions) {
  const id = createUniqueId();
  const [open, setOpen] = createSignal(false);
  const [labelValue, setLabelValue] = createSignal(
    options.label.labelText === "preset"
      ? options.label.preset
      : options.options.find((options) => options.selected)?.name ?? options.options[0].name ?? "Select..."
  );
  const [selectedValues, setSelectedValues] = createStore<boolean[]>(options.options.map((option) => option?.selected ?? false));

  createEffect(() => {
    const selected: any[] = [];

    for (let i = 0; i < selectedValues.length; i++) {
      const value = selectedValues[i];
      if (value) selected.push(options.options[i].value);
    }

    options.onChange(selected);
  });

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
              {(option, i) => {
                function onSelect() {
                  setSelectedValues(i(), !selectedValues[i()]);
                  if (!options.multiselect) {
                    setSelectedValues(
                      produce((state) => {
                        for (let index = 0; index < state.length; index++) {
                          if (index !== i()) state[index] = false;
                        }
                      })
                    );
                  }
                  if (options.label.labelText === "value") setLabelValue(option.value);
                }

                return (
                  <li
                    tabIndex={0}
                    role="button"
                    aria-checked={selectedValues[i()]}
                    on:click={onSelect}
                    on:keydown={(event) => {
                      if (event.key === "Enter") onSelect();
                    }}
                  >
                    <Show when={selectedValues[i()]}>✅</Show>
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
