import { faArrowDownShortWide, faArrowUpWideShort, faRefresh } from "@fortawesome/free-solid-svg-icons";
import { Fa } from "solid-fa";
import { createResource, createSignal, type ResourceFetcherInfo, Show, useContext } from "solid-js";

import { countModIndex, fetchModIndex, ModSortColumn, queryModIndex, type SortOption } from "../../api";
import { createProgressProxyStore } from "../../api/tasks";
import type { Mod } from "../../types.d.ts";
import { numberFormatter } from "../../utils";
import { ErrorContext } from "../global/ErrorBoundary";
import { SimpleProgressIndicator } from "../global/Progress";
import SelectDropdown from "../global/SelectDropdown";
import { SortableList } from "../global/SortableList";
import TogglableDropdown from "../global/TogglableDropdown";
import ModList, { Fetcher } from "./ModList";
import styles from "./ModSearch.module.css";

export interface InitialProgress {
  completed_steps: null;
  total_steps: null;
  progress: null;
}

const MODS_PER_PAGE = 50;

function createModFetcher(
  game: string,
  query: string,
  sort: readonly SortOption<ModSortColumn>[],
): (index: number) => Promise<Mod> {
  const CACHE_CAPACITY = 6;
  let cacheBase = 0;
  const cache = new Array<readonly Mod[] | Promise<readonly Mod[]> | undefined>(CACHE_CAPACITY);

  async function fetchNewPage(cacheBase: number, page: number, line: number) {
    console.log(`fetchNewPage(${cacheBase}, ${page}, ${line})`);
    const promise = (async () => {
      return (
        await queryModIndex(game, query, sort, {
          skip: page * MODS_PER_PAGE,
          limit: MODS_PER_PAGE,
        })
      ).mods;
    })();
    const cacheIndex = page - cacheBase;
    cache[cacheIndex] = promise;
    return (await promise)[line];
  }

  return async (index: number) => {
    // figure out which page the requested mod is in
    const page = Math.floor(index / MODS_PER_PAGE);
    const line = index % MODS_PER_PAGE;

    if (page >= cacheBase) {
      if (page - cacheBase < CACHE_CAPACITY) {
        let cachedPage = cache[page - cacheBase];
        if (cachedPage != null) {
          if (!Array.isArray(cachedPage)) cachedPage = await cachedPage;
          return cachedPage[line];
        }

        // need to fetch a new page

        return await fetchNewPage(page - cacheBase, page, line);
      } else {
        // TODO: rotate the cache to avoid flushing the entire thing
      }
    } else {
      // TODO: rotate the cache to avoid flushing the entire thing
    }

    cacheBase = page;
    for (let i = 1; i < CACHE_CAPACITY; i++) {
      cache[i] = undefined;
    }
    return await fetchNewPage(0, page, line);
  };
}

export default function ModSearch(props: { game: string }) {
  const [query, setQuery] = createSignal("");

  const [sort, setSort] = createSignal<SortOption<ModSortColumn>[]>([
    { column: ModSortColumn.Relevance, descending: true },
    { column: ModSortColumn.Downloads, descending: true },
    { column: ModSortColumn.Name, descending: false },
    { column: ModSortColumn.Owner, descending: false },
  ]);

  const [progress, setProgress] = createProgressProxyStore();

  const reportErr = useContext(ErrorContext)!;

  const [loadStatus, { refetch: refetchModIndex }] = createResource(
    () => props.game,
    async (game, info: ResourceFetcherInfo<boolean, never>) => {
      try {
        await fetchModIndex(game, { refresh: info.refetching }, (event) => {
          if (event.event === "created") {
            setProgress(event.progress);
          }
        });
      } catch (e) {
        reportErr(e);
        throw e;
      }
      return true;
    },
  );

  const [queriedMods] = createResource<{ count: number; mods: Fetcher }>(
    () =>
      [props.game, query(), sort(), loadStatus.loading] as [
        string,
        string,
        readonly SortOption<ModSortColumn>[],
        true | undefined,
      ],
    async ([game, query, sort]) => {
      const count = await countModIndex(game, query);
      return {
        count,
        mods: createModFetcher(game, query, sort),
      };
    },
    { initialValue: { count: 0, mods: async (_: number) => [] } },
  );

  const [profileSortOrder, setProfileSortOrder] = createSignal(false);

  return (
    <div class={styles.modSearch}>
      <form on:submit={(e) => e.preventDefault()} class={styles.modSearch__form}>
        <div class={styles.modSearch__searchBar}>
          <input
            type="mod-search"
            placeholder="Search for mods"
            value={query()}
            on:input={(e) => setQuery(e.target.value)}
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
            }}
            onChanged={() => {}}
          />

          <button
            type="button"
            // class={sidebarStyles.sidebar__profilesSearchSortByBtn}
            on:click={() => setProfileSortOrder((order) => !order)}
          >
            {profileSortOrder() ? <Fa icon={faArrowUpWideShort} /> : <Fa icon={faArrowDownShortWide} />}
          </button>

          <TogglableDropdown label="Advanced" labelClass={styles.modSearch__dropdownBtn}>
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
                              on:change={(e) =>
                                setSort([
                                  ...sort().slice(0, i),
                                  { column: option.column, descending: e.target.checked },
                                  ...sort().slice(i + 1),
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

      <Show when={loadStatus.loading}>
        <div class={styles.progressLine}>
          <p>Fetching mods</p>
          <SimpleProgressIndicator progress={progress} />
        </div>
      </Show>

      <Show when={queriedMods.latest} fallback={<p>Querying mods...</p>}>
        <div class={styles.discoveredLine}>
          Discovered {numberFormatter.format(queriedMods()!.count)} mods
          <button class={styles.refreshButton} on:click={() => refetchModIndex()}>
            <Fa icon={faRefresh} />
          </button>
        </div>
        <ModList count={queriedMods()!.count} mods={queriedMods()!.mods} />
      </Show>
    </div>
  );
}
