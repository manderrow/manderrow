import { createResource, createSignal, createUniqueId, ResourceFetcherInfo, Show } from "solid-js";
import { fetchModIndex, queryModIndex, SortColumn, SortOption } from "../../api";
import { SortableList } from "../global/SortableList";
import ModList from "./ModList";
import styles from "./ModSearch.module.css";
import { faRefresh } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { numberFormatter } from "../../utils";
import { createStore } from "solid-js/store";

interface ProgressData {
  completed: number;
  total: number;
}
interface InitialProgress {
  completed: null;
  total: null;
}

const MODS_PER_PAGE = 50;

export default function ModSearch(props: { game: string }) {
  const [query, setQuery] = createSignal("");

  const [sort, setSort] = createSignal<SortOption[]>([
    { column: SortColumn.Relevance, descending: true },
    { column: SortColumn.Downloads, descending: true },
    { column: SortColumn.Name, descending: false },
    { column: SortColumn.Owner, descending: false },
  ]);

  const [progress, setProgress] = createStore<InitialProgress | ProgressData>({ completed: null, total: null });

  const [loadStatus, { refetch: refetchModIndex }] = createResource(
    () => props.game,
    async (game, info: ResourceFetcherInfo<boolean, never>) => {
      await fetchModIndex(game, { refresh: info.refetching }, setProgress);
      return true;
    }
  );

  const [queriedMods] = createResource(
    () => [props.game, query(), sort(), loadStatus.loading] as [string, string, SortOption[], true | undefined],
    async ([game, query, sort]) => {
      const { count } = await queryModIndex(game, query, sort, { limit: 0 });
      return { count, mods: async (page: number) => (await queryModIndex(game, query, sort, { skip: page * MODS_PER_PAGE, limit: MODS_PER_PAGE })).mods };
    },
    { initialValue: { mods: async (_: number) => [], count: 0 } }
  );

  const searchOptionsId = createUniqueId();

  return (
    <div class={styles.modSearch}>
      <div class={styles.searchForm}>
        <div class={styles.searchBar}>
          <input type="search" placeholder="Search" value={query()} on:input={(e) => setQuery(e.target.value)} />
          <label for={searchOptionsId}>Options</label>
        </div>
        <input type="checkbox" class={styles.searchOptionsToggle} id={searchOptionsId} />
        <div class={styles.searchOptions}>
          <div class={styles.sortOptions}>
            <div class={styles.inner}>
              <SortableList items={[sort, setSort]} id={(option) => option.column}>
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
                          on:change={(e) => setSort([...sort().slice(0, i), { column: option.column, descending: e.target.checked }, ...sort().slice(i + 1)])}
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
      </div>

      <Show when={loadStatus.loading}>
        <div class={styles.progressLine}>
          <p>Fetching mods</p>
          <progress value={progress.total == null ? 0 : progress.completed / progress.total} />
        </div>
      </Show>

      <Show when={queriedMods.latest} fallback={<p>Querying mods...</p>}>
        <div class={styles.discoveredLine}>
          Discovered {numberFormatter.format(queriedMods()!.count)} mods
          <button class={styles.refreshButton} on:click={() => refetchModIndex()}>
            <Fa icon={faRefresh} />
          </button>
        </div>
        <ModList mods={queriedMods()!.mods} />
      </Show>
    </div>
  );
}
