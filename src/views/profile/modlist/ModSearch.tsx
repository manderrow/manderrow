import { faArrowDownShortWide, faArrowUpWideShort } from "@fortawesome/free-solid-svg-icons";
import { Fa } from "solid-fa";
import { Setter } from "solid-js";

import { ModSortColumn, SortOption } from "../../../api/api";

import SelectDropdown from "../../../widgets/SelectDropdown";
import { SortableList } from "../../../widgets/SortableList";
import TogglableDropdown from "../../../widgets/TogglableDropdown";

import styles from "./ModSearch.module.css";
import { t } from "../../../i18n/i18n";

interface ModSearchProps {
  game: string;
  query: string;
  setQuery: Setter<string>;
  sort: readonly SortOption<ModSortColumn>[];
  setSort: Setter<readonly SortOption<ModSortColumn>[]>;
  profileSortOrder: boolean;
  setProfileSortOrder: Setter<boolean>;
}
export default function ModSearch(props: ModSearchProps) {
  return (
    <form on:submit={(e) => e.preventDefault()} class={styles.modSearch}>
      <input
        type="mod-search"
        placeholder="Search for mods"
        value={props.query}
        on:input={(e) => props.setQuery(e.target.value)}
      />
      <label for="mod-search" class="phantom">
        Mod search
      </label>
      <div class={`${styles.modSearch__group} ${styles.groupSort}`}>
        <SelectDropdown
          label={{ labelText: "preset", preset: "Sort By" }}
          labelClass={styles.modSearch__dropdownBtn}
          multiselect={false}
          options={[
            {
              value: ModSortColumn.Relevance,
              label: t("global.mod_sort_column.relevance"),
              selected: () => true,
            },
            {
              value: ModSortColumn.Downloads,
              label: t("global.mod_sort_column.downloads"),
              selected: () => false,
            },
            {
              value: ModSortColumn.Name,
              label: t("global.mod_sort_column.name"),
              selected: () => false,
            },
            {
              value: ModSortColumn.Owner,
              label: t("global.mod_sort_column.owner"),
              selected: () => false,
            },
            {
              value: ModSortColumn.Size,
              label: t("global.mod_sort_column.size"),
              selected: () => false,
            },
          ]}
          onChanged={() => {}}
          offset={4}
        />
        <button
          type="button"
          // class={sidebarStyles.sidebar__profilesSearchSortByBtn}
          on:click={() => props.setProfileSortOrder((order) => !order)}
          class={styles.modSearch__dropdownBtn}
        >
          {props.profileSortOrder ? <Fa icon={faArrowUpWideShort} /> : <Fa icon={faArrowDownShortWide} />}
        </button>
      </div>
      <div class={styles.modSearch__group}>
        <TogglableDropdown label="Advanced" labelClass={styles.modSearch__dropdownBtn}>
          <div class={styles.searchOptions}>
            <div class={styles.sortOptions}>
              <div class={styles.inner}>
                <SortableList items={[() => props.sort, props.setSort]} id={(option) => option.column}>
                  {(option, i) => {
                    const id = `sort-descending-${option.column}`;
                    return (
                      <div class={styles.sortOption}>
                        {t(`global.mod_sort_column.${option.column}`)}
                        <div class={styles.descendingToggle}>
                          <input
                            type="checkbox"
                            id={id}
                            checked={option.descending}
                            on:change={(e) =>
                              props.setSort([
                                ...props.sort.slice(0, i),
                                { column: option.column, descending: e.target.checked },
                                ...props.sort.slice(i + 1),
                              ])
                            }
                          />
                          <label for={id}>
                            {option.descending ? t("global.sort_order.descending") : t("global.sort_order.ascending")}
                          </label>
                        </div>
                      </div>
                    );
                  }}
                </SortableList>
              </div>
            </div>
          </div>
        </TogglableDropdown>
      </div>
    </form>
  );
}
