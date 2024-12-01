import { createResource, createSignal, createUniqueId, Show } from "solid-js";
import { fetchModIndex, queryModIndex, SortColumn, SortOption } from "../../api";
import { SortableList } from "../SortableList";
import ModList from "./ModList";
import styles from './ModSearch.module.css';

export default function ModSearch(props: { game: string }) {
  const [query, setQuery] = createSignal('');

  const [sort, setSort] = createSignal<SortOption[]>([
    { column: SortColumn.Relevance, descending: true },
    { column: SortColumn.Downloads, descending: true },
    { column: SortColumn.Name, descending: false },
    { column: SortColumn.Owner, descending: false },
  ]);

  const [modIndex] = createResource(() => props.game, async game => {
    await fetchModIndex(game, { refresh: false });
    return true;
  });

  const [queriedMods] = createResource(() => [props.game, modIndex.loading, query(), sort()] as [string, true | undefined, string, SortOption[]], ([game, _, query, sort]) => {
    console.log(`Querying ${JSON.stringify(sort)}`);
    return queryModIndex(game, query, sort);
  }, { initialValue: { mods: [], count: 0 } });

  const searchOptionsId = createUniqueId();

  return <div class={styles.modSearch}>
    <div class={styles.searchForm}>
      <div class={styles.searchBar}>
        <input type="search" placeholder="Search" value={query()} on:input={e => setQuery(e.target.value)} />
        <label for={searchOptionsId}>Options</label>
      </div>
      <input type="checkbox" class={styles.searchOptionsToggle} id={searchOptionsId} />
      <div class={styles.searchOptions}>
        <div class={styles.sortOptions}>
          <div class={styles.inner}>
            <SortableList items={[sort, setSort]} id={option => option.column}>
              {(option, i) => {
                const id = `sort-descending-${option.column}`;
                return <div class={styles.sortOption}>
                  {option.column}
                  <div class={styles.descendingToggle}>
                    <input type="checkbox" id={id} checked={option.descending} on:change={e => setSort([...sort().slice(0, i), { column: option.column, descending: e.target.checked }, ...sort().slice(i + 1)])} />
                    <label for={id}>{option.descending ? 'Descending' : 'Ascending'}</label>
                  </div>
                </div>;
              }}
            </SortableList>
          </div>
        </div>
      </div>
    </div>

    <Show when={modIndex.loading}>
      <p>Fetching mods...</p>
    </Show>

    <Show when={queriedMods.latest} fallback={<p>Querying mods...</p>}>
      <p>Discovered {queriedMods()!.count} mods</p>
      <ModList mods={queriedMods()!.mods} />
    </Show>
  </div>;
}
