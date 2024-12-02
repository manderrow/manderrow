import { createSignal, For, Show, Suspense } from "solid-js";
import ModSearch from "../../components/profile/ModSearch";
import { games } from "../../globals";

export default function GameSelect() {
  const [selectedGame, setSelectedGame] = createSignal<string | undefined>(games()[0]?.id);

  return (
    <main>
      <h1>Game Select</h1>

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
