import { createResource, createSignal, Show } from "solid-js";
import { fetchModIndex, queryModIndex } from "../../api";
import ModList from "./ModList";
import styles from './ModSearch.module.css';

export default function ModSearch(props: { game: string }) {
  const [query, setQuery] = createSignal('');

  const [modIndex] = createResource(() => props.game, async game => {
    await fetchModIndex(game, { refresh: false });
    return true;
  });

  const [queriedMods] = createResource(() => [props.game, modIndex.loading, query()] as [string, true | undefined, string], ([game, _, query]) => {
    return queryModIndex(game, query);
  }, { initialValue: { mods: [], count: 0 } });

  return <>
    <div class={styles.searchBar}>
      <input type="search" placeholder="Search" value={query()} on:input={e => setQuery(e.target.value)} />
      <Show when={queriedMods.loading}>
        <span>Querying mods...</span>
      </Show>
    </div>

    <Show when={modIndex.loading}>
      <p>Fetching mods...</p>
    </Show>

    <Show when={queriedMods.latest}>
      <p>Discovered {queriedMods()!.count} mods</p>
      <ModList mods={queriedMods()!.mods} />
    </Show>
  </>;
}
