import { batch, createEffect, For, untrack } from "solid-js";
import { faCheck } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { createStore } from "solid-js/store";
import { t } from "../../i18n/i18n";

import styles from "./SelectDropdown.module.css";
import TogglableDropdown, { TogglableDropdownOptions } from "./TogglableDropdown";

interface Option<T> {
  text: string;
  value: T;
  selected: () => boolean;
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

interface SelectDropdownOptions<T> extends Omit<TogglableDropdownOptions, "children" | "label" | "dropdownClass"> {
  label: LabelText;
  options: Option<T>[];
  onChanged: (value: T, selected: boolean) => void;
  multiselect?: boolean;
}

export default function SelectDropdown<T>(options: SelectDropdownOptions<T>) {
  // TODO: use createEffect to support dynamically adding/removing options
  const labelValue = () =>
    options.label.labelText === "preset"
      ? options.label.preset
      : // FIXME: correct label for multiselect
        options.options.find((value) => value.selected())?.text ??
        options.label.fallback ??
        t("global.select_dropdown.default_fallback");

  return (
    <TogglableDropdown
      dropdownClass={styles.dropdown}
      label={labelValue()}
      labelClass={options.labelClass}
      offset={options.offset}
    >
      <ul class={styles.options} role={options.multiselect === false ? "radiogroup" : "listbox"}>
        <For each={options.options}>
          {(option) => {
            let ref!: HTMLLIElement;

            function onSelect() {
              // use the cached value here, so the action performed by the
              // UI is **never** out of sync with the displayed value.
              const isSelected = ref.ariaChecked! !== "true";
              options.onChanged(option.value, isSelected);
            }

            return (
              <li
                tabIndex={0}
                role={options.multiselect === false ? "radio" : "option"}
                class={styles.option}
                aria-checked={option.selected()}
                on:click={onSelect}
                on:keydown={(event) => {
                  if (event.key === "Enter" || event.key === " ") onSelect();
                }}
                ref={ref}
              >
                <Fa icon={faCheck} class={styles.option__check} />
                {option.text}
              </li>
            );
          }}
        </For>
      </ul>
    </TogglableDropdown>
  );
}
