import { createEffect, createSignal, For, JSX } from "solid-js";
import { faCheck } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { t } from "../../i18n/i18n";

import styles from "./SelectDropdown.module.css";
import TogglableDropdown, { TogglableDropdownOptions } from "./TogglableDropdown";

type Option<T> = {
  label: string;
  value: T;
  selected: () => boolean;
  liContent?: JSX.Element,
};

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
  liClass?: string;
  multiselect?: boolean;
}

export default function SelectDropdown<T>(options: SelectDropdownOptions<T>) {
  // TODO: use createEffect to support dynamically adding/removing options
  const labelValue = () =>
    options.label.labelText === "preset"
      ? options.label.preset
      : // FIXME: correct label for multiselect
        options.options.find((value) => value.selected())?.label ??
        options.label.fallback ??
        t("global.select_dropdown.default_fallback");

  let listRef!: HTMLUListElement;

  const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement | undefined>(undefined);

  createEffect(() => {
    const dropdown = dropdownRef();
    if (!dropdown) return;

    if (listRef.scrollHeight > dropdown.clientHeight) {
      listRef.querySelector("li[aria-checked=true]")?.scrollIntoView({
        behavior: "instant",
        block: "center",
      });
    }
  });

  return (
    <TogglableDropdown
      ref={setDropdownRef}
      dropdownClass={styles.dropdown}
      label={labelValue()}
      labelClass={options.labelClass}
      offset={options.offset}
    >
      <ul class={styles.options} role={options.multiselect === false ? "radiogroup" : "listbox"} ref={listRef}>
        <For each={options.options}>
          {(option) => {
            let ref!: HTMLLIElement;

            // TODO: we could use a single function that checks the event target instead of using a ref
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
                class={`${styles.option} ${options.liClass || ""}`}
                aria-checked={option.selected()}
                on:click={onSelect}
                on:keydown={(event) => {
                  if (event.key === "Enter" || event.key === " ") onSelect();
                }}
                ref={ref}
              >
                <Fa icon={faCheck} class={styles.option__check} />

                {option.liContent ? option.liContent : option.label}
              </li>
            );
          }}
        </For>
      </ul>
    </TogglableDropdown>
  );
}
