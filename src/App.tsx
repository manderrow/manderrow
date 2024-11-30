import { createEffect, createResource, createSignal, For, Show, Suspense } from "solid-js";
import { fetchModIndex, getGames, queryModIndex } from "./api";
import "./App.css";
import modListStyles from './ModList.module.css';

const [games] = createResource(getGames);

function ModSearch(props: { game: string }) {
  const [query, setQuery] = createSignal('');

  const [modIndex] = createResource(() => props.game, async game => {
    await fetchModIndex(game, { refresh: false });
    return true;
  });

  const [queriedMods] = createResource(() => [props.game, modIndex.loading, query()] as [string, true | undefined, string], ([game, _, query]) => {
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
            <img class={modListStyles.icon} src={mod.versions[0].icon} />
            <div class={modListStyles.split}>
              <div class={modListStyles.left}>
                <p class={modListStyles.name}>{mod.full_name}</p>
                <div class={modListStyles.categories}>
                  <For each={mod.categories}>
                    {category => <div>{category}</div>}
                  </For>
                </div>
              </div>
              <div class={modListStyles.right}>
                <p class={modListStyles.downloads}>{mod.versions[0].downloads ?? '0'}</p>
              </div>
            </div>
          </div>}
        </For>
      </div>
    </Show>
  </>;
}

function App() {
  const [selectedGame, setSelectedGame] = createSignal<string | null>(null);

  createEffect(() => {
    if (games.latest !== undefined) {
      if (selectedGame() === null) {
        setSelectedGame(games.latest[0].id);
      }
    }
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
