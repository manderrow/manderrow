import { createSignal, For, Suspense } from "solid-js";
import { games } from "../../globals";

import { A } from "@solidjs/router";
import { Game } from "../../types";
import styles from "./GameSelect.module.css";

export default function GameSelect() {
  const [displayType, setDisplayType] = createSignal<"card" | "list">("card");
  const [search, setSearch] = createSignal("");

  return (
    <>
      <style>body {"{ position: relative; z-index: 1; }"}</style>

      <div class={styles.gradientBlobs} aria-hidden="true">
        <div class={styles.gradientBlob} data-blob-1></div>
        <div class={styles.gradientBlob} data-blob-2></div>
        <div class={styles.gradientBlob} data-blob-3></div>
      </div>
      <header class={styles.header}>
        <h1>Game Selection</h1>
        <p>Select the game you are managing mods for</p>
      </header>
      <main class={styles.main}>
        <form action="#" class={styles.gameSearch}>
          <input
            type="search"
            name="search-game"
            id="search-game"
            placeholder="Search for a game"
            value={search()}
            on:input={(e) => setSearch(e.target.value)}
          />
        </form>
        <ol
          classList={{ [styles.gameList]: true, [styles.gameList__gameCard]: displayType() === "card", [styles.gameList__gameItem]: displayType() === "list" }}
        >
          <Suspense>
            <For each={games()}>{(game) => <GameComponent game={game} />}</For>
          </Suspense>
        </ol>
      </main>
    </>
  );
}

function GameComponent(props: { game: Game }) {
  const url = `/img/game_covers/${props.game.game_image}`;

  return (
    <li class={styles.gameList__game} style={`--img-src: url(${url})`}>
      <img src={url} alt={`Background image of ${props.game.name}`} />
      <div class={styles.game__content}>
        <p class={styles.game__title}>{props.game.name}</p>
        <div class={styles.game__actions}>
          <A href={`/profile/${props.game.id}/`} tabIndex="-1">
            <button>Select</button>
          </A>
        </div>
      </div>
    </li>
  );
}
