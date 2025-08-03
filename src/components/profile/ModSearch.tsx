import { faArrowDownShortWide, faArrowUpWideShort } from "@fortawesome/free-solid-svg-icons";
import { Fa } from "solid-fa";
import { Setter } from "solid-js";

import { ModSortColumn, SortOption } from "../../api";

import SelectDropdown from "../global/SelectDropdown";
import { SortableList } from "../global/SortableList";
import TogglableDropdown from "../global/TogglableDropdown";

import styles from "./ModSearch.module.css";
import { t } from "../../i18n/i18n";

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
    <div class={styles.modSearch}>
      <form on:submit={(e) => e.preventDefault()} class={styles.modSearch__form}>
        <div class={styles.modSearch__searchBar}>
          <input
            type="mod-search"
            placeholder="Search for mods"
            value={props.query}
            on:input={(e) => props.setQuery(e.target.value)}
          />
          <label for="mod-search" class="phantom">
            Mod search
          </label>
          {/* TODO: change select dropdown to support names for values so i18n works good */}
          <SelectDropdown
            label={{ labelText: "preset", preset: "Sort By" }}
            options={[
              {
                value: ModSortColumn.Relevance,
                text: t("global.mod_sort_column.relevance"),
                selected: true,
              },
              {
                value: ModSortColumn.Downloads,
                text: t("global.mod_sort_column.downloads"),
              },
              {
                value: ModSortColumn.Name,
                text: t("global.mod_sort_column.name"),
              },
              {
                value: ModSortColumn.Owner,
                text: t("global.mod_sort_column.owner"),
              },
              {
                value: ModSortColumn.Size,
                text: t("global.mod_sort_column.size"),
              },
            ]}
            onChanged={() => {}}
            offset={{ mainAxis: 4 }}
          />
          <button
            type="button"
            // class={sidebarStyles.sidebar__profilesSearchSortByBtn}
            on:click={() => props.setProfileSortOrder((order) => !order)}
          >
            {props.profileSortOrder ? <Fa icon={faArrowUpWideShort} /> : <Fa icon={faArrowDownShortWide} />}
          </button>
          <TogglableDropdown label="Advanced" labelClass={styles.modSearch__dropdownBtn} offset={{ mainAxis: 4 }}>
            <div class={styles.searchOptions}>
              <div class={styles.sortOptions}>
                <div class={styles.inner}>
                  <SortableList items={[() => props.sort, props.setSort]} id={(option) => option.column}>
                    {(option, i) => {
                      const id = `sort-descending-${option.column}`;
                      return (
                        <div class={styles.sortOption}>
                          {option.column}
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
                            <label for={id}>{option.descending ? "Descending" : "Ascending"}</label>
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
    </div>
  );
}
