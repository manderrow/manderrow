import { createEffect, createResource, createSignal, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

interface Mod {
  name: string;
  full_name: string;
  description?: string;
  icon?: string;
  version_number?: string;
  dependencies: string[];
  download_url?: string;
  downloads?: string;
  date_created: string;
  website_url?: string;
  is_active?: string;
  uuid4: string;
  file_size?: number;
}

interface QueryResult {
  mods: Mod[];
  count: number;
}

async function queryModIndex(query: string): Promise<QueryResult> {
  return await invoke("query_mod_index", { query });
}

export default function ModSearch() {
  const [query, setQuery] = createSignal("");

  const [queriedMods, { refetch: refetchQueriedMods }] = createResource(() => {
    return queryModIndex(query());
  });

  createEffect(() => {
    query();
    refetchQueriedMods();
  });

  const [modIndex, { refetch: refetchModIndex }] = createResource(async () => {
    await invoke("fetch_mod_index", {});
    refetchQueriedMods();
    return true;
  });

  return (
    <main class="container">
      <h1>Thunderstore</h1>

      <input type="search" placeholder="Search" value={query()} on:input={(e) => setQuery(e.target.value)} />

      <Show when={modIndex() && queriedMods() != null}>
        <p>Discovered {queriedMods()!.count} mods</p>
        <For each={queriedMods()!.mods}>{(mod) => <p>{mod.full_name}</p>}</For>
      </Show>
      <Show when={!modIndex() || queriedMods() == null}>
        <p>Loading...</p>
      </Show>
    </main>
  );
}
