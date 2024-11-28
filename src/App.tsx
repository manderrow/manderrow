import { createEffect, createResource, createSignal, For, Show, Suspense } from "solid-js";
import { fetchModIndex, getGames, queryModIndex } from "./api";
import "./App.css";
import modListStyles from './ModList.module.css';


function ModSearch(props: { game: string }) {
  const [query, setQuery] = createSignal('');

  console.log(`in func game=${props.game}`);

  const [modIndex, { refetch: refetchModIndex }] = createResource(() => props.game, async game => {
    console.log(`in resource game=${game}`);
    await fetchModIndex(game);
    console.log(`fetched mod index`);
    return true;
  });

  const [queriedMods, { refetch: refetchQueriedMods }] = createResource(() => [props.game, modIndex.loading, query()] as [string, true | undefined, string], ([game, _, query]) => {
    console.log(`Querying mods for ${game} by ${query}`);
    return queryModIndex(game, query);
  }, { initialValue: { mods: [], count: 0 } });

  return <>
    <div class={modListStyles.searchBar}>
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
      <div class={modListStyles.modList}>
        <For each={queriedMods()!.mods}>
          {mod => <div>
            <div>
              <p class={modListStyles.name}>{mod.full_name}</p>
            </div>
            <div>
              <p class={modListStyles.downloads}>{mod.versions[0].downloads ?? '0'}</p>
            </div>
          </div>}
        </For>
      </div>
    </Show>
  </>;
}

function App() {
  const [selectedGame, setSelectedGame] = createSignal<string | null>(null);

  const [games] = createResource(async () => {
    const games = await getGames();
    if (selectedGame() === null) {
      setSelectedGame(games[0].id);
    }
    return games;
  });

  return (
    <main class="container">
      <h1>Thunderstore</h1>

      <select on:change={e => setSelectedGame(e.target.value)}>
        <Suspense>
          <For each={games()}>
            {game => <option value={game.id} selected={selectedGame() === game.id}>{game.name}</option>}
          </For>
        </Suspense>
      </select>

      <Show when={selectedGame() !== null}>
        <ModSearch game={selectedGame()!} />
      </Show>
    </main>
  );
}

export default App;
