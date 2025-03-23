import { batch, createEffect, For, untrack } from "solid-js";
import { faCheck } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { DropdownOptions } from "./Dropdown";
import { createStore } from "solid-js/store";
import { t } from "../../i18n/i18n";

import styles from "./SelectDropdown.module.css";
import TogglableDropdown, { TogglableDropdownOptions } from "./TogglableDropdown";

interface Option<T> {
  value: T;
  selected?: boolean;
}

type LabelTextValue = {
  labelText: "value";
  fallback?: string;
};
type LabelTextPreset = {
  labelText: "preset";
  preset: string;
};
type LabelText = LabelTextValue | LabelTextPreset;

interface SelectDropdownOptions<T>
  extends Omit<TogglableDropdownOptions, "children" | "label" | "labelClass" | "dropdownClass"> {
  label: LabelText;
  options: Record<string, Option<T>>;
  onChanged: (value: T, selected: boolean) => void;
  nullable?: boolean;
  multiselect?: boolean;
}

export default function SelectDropdown<T>(options: SelectDropdownOptions<T>) {
  const [selected, setSelected] = createStore(options.options);

  createEffect(() => {
    batch(() => {
      // don't want to depend on selected, only options.options
      untrack(() => {
        for (const [key, _] of Object.entries(selected)) {
          if (!options.options.hasOwnProperty(key)) {
            setSelected(({ [key]: _, ...selected }) => selected);
          }
        }
      });
      // which is tracked here.
      for (const [key, value] of Object.entries(options.options)) {
        setSelected(key, value);
      }
    });
  });

  // TODO: use createEffect to support dynamically adding/removing options
  const labelValue = () =>
    options.label.labelText === "preset"
      ? options.label.preset
      : // FIXME: correct label for multiselect
        Object.entries(options.options).find(([key, value]) => value.selected)?.[0] ??
        options.label.fallback ??
        t("global.select_dropdown.default_fallback");

  return (
    <TogglableDropdown
      align={options.align}
      maxWidth={options.maxWidth}
      class={options.class}
      dropdownClass={styles.dropdown}
      buttonId={options.buttonId}
      label={labelValue()}
    >
      <ul class={styles.options}>
        <For each={Object.entries(options.options)}>
          {([key, option]) => {
            let ref!: HTMLLIElement;

            function onSelect() {
              // use the cached value here, so the action performed by the
              // UI is **never** out of sync with the displayed value.
              const isSelected = ref.ariaChecked! !== "true";

              if (!isSelected && !options.nullable) {
                if (
                  !options.multiselect ||
                  Object.entries(options.options).find(([otherKey, value]) => value.selected && key !== otherKey) ===
                    undefined
                ) {
                  return;
                }
              }

              setSelected(key, "selected", isSelected);
              options.onChanged(option.value, isSelected);

              if (!options.multiselect && isSelected) {
                for (const other in options.options) {
                  if (options.options[other].value !== option.value) {
                    setSelected(other, "selected", false);
                  }
                }
              }
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
    </TogglableDropdown>
  );
}
