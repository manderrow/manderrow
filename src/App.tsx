import { createEffect, createResource, createSignal, For, Show, Suspense } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";
import "./App.css";

function App() {
  async function queryModIndex(query: string): Promise<Object[]> {
    return await invoke('query_mod_index', { query });
  }

  const [query, setQuery] = createSignal('');

  const [queriedMods, { refetch: refetchQueriedMods }] = createResource(() => {
    return queryModIndex(query());
  });

  createEffect(() => {
    query();
    refetchQueriedMods();
  });

  const [modIndex, { refetch: refetchModIndex }] = createResource(async () => {
    await invoke('fetch_mod_index', {})
    refetchQueriedMods();
    return true;
  });

  return (
    <main class="container">
      <h1>Thunderstore</h1>

      <input type="search" placeholder="Search" value={query()} on:input={e => setQuery(e.target.value)} />

      <Show when={modIndex() && queriedMods() != null}>
          <p>Discovered {queriedMods()!.length} mods</p>
          <For each={queriedMods()!}>
            {mod => <p>{JSON.stringify(mod)}</p>}
          </For>
      </Show>
      <Show when={!modIndex() || queriedMods() == null}>
        <p>Loading...</p>
      </Show>
    </main>
  );
}

export default App;
