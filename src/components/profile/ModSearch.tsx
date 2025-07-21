import { faArrowDownShortWide, faArrowUpWideShort } from "@fortawesome/free-solid-svg-icons";
import { Fa } from "solid-fa";
import { Setter, Show } from "solid-js";

import { ModSortColumn, SortOption } from "../../api";
import { Progress } from "../../api/tasks";

import { SimpleProgressIndicator } from "../global/Progress";
import SelectDropdown from "../global/SelectDropdown";
import { SortableList } from "../global/SortableList";
import TogglableDropdown from "../global/TogglableDropdown";

import styles from "./ModSearch.module.css";

export interface InitialProgress {
  completed_steps: null;
  total_steps: null;
  progress: null;
}

interface ModSearchProps {
  game: string;
  query: string;
  setQuery: Setter<string>;
  sort: readonly SortOption<ModSortColumn>[];
  setSort: Setter<readonly SortOption<ModSortColumn>[]>;
  profileSortOrder: boolean;
  setProfileSortOrder: Setter<boolean>;
  isLoading: boolean;
  progress: Progress;
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

          <SelectDropdown
            label={{ labelText: "preset", preset: "Sort By" }}
            options={{
              [ModSortColumn.Relevance]: {
                value: "relevance",
                selected: true,
              },
              [ModSortColumn.Downloads]: {
                value: "downloads",
              },
              [ModSortColumn.Name]: {
                value: "name",
              },
              [ModSortColumn.Owner]: {
                value: "owner",
              },
              [ModSortColumn.Size]: {
                value: "size",
              },
            }}
            onChanged={() => {}}
          />

          <button
            type="button"
            // class={sidebarStyles.sidebar__profilesSearchSortByBtn}
            on:click={() => props.setProfileSortOrder((order) => !order)}
          >
            {props.profileSortOrder ? <Fa icon={faArrowUpWideShort} /> : <Fa icon={faArrowDownShortWide} />}
          </button>

          <TogglableDropdown label="Advanced" labelClass={styles.modSearch__dropdownBtn}>
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

      <Show when={props.isLoading}>
        <div class={styles.progressLine}>
          <p>Fetching mods</p>
          <SimpleProgressIndicator progress={props.progress} />
        </div>
      </Show>
    </div>
  );
}
