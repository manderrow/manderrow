import { faStar } from "@fortawesome/free-regular-svg-icons";
import { A } from "@solidjs/router";
import Fa from "solid-fa";
import { createSignal, For, onCleanup, onMount } from "solid-js";

import { games } from "../../globals";
import { Game } from "../../types";

import blobStyles from "./GameBlobs.module.css";
import gameListStyles from "./GameList.module.css";
import styles from "./GameSelect.module.css";

export default function GameSelect() {
  const [displayType, setDisplayType] = createSignal<"card" | "list">("card");
  const [search, setSearch] = createSignal("");

  onMount(() => {
    document.body.classList.add(styles.body);
  });

  onCleanup(() => {
    document.body.classList.remove(styles.body);
  });

  return (
    <>
      <div class={blobStyles.gradientBlobs} aria-hidden="true">
        <div class={blobStyles.gradientBlob} data-blob-1></div>
        <div class={blobStyles.gradientBlob} data-blob-2></div>
        <div class={blobStyles.gradientBlob} data-blob-3></div>
        <div class={blobStyles.gradientBlob} data-blob-4></div>
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
            maxlength="100"
            on:input={(e) => setSearch(e.target.value)}
          />
        </form>
        <ol
          classList={{
            [gameListStyles.gameList]: true,
            [gameListStyles.searching]: search().length > 0,
            [gameListStyles.gameList__gameCard]: displayType() === "card",
            [gameListStyles.gameList__gameItem]: displayType() === "list",
          }}
        >
          <For
            each={games().filter((game) => game.name.toLowerCase().includes(search().toLowerCase()))}
            fallback={
              <li class={gameListStyles.gameList__empty}>
                <p>No games matched the search</p>
              </li>
            }
          >
            {(game) => <GameComponent game={game} />}
          </For>
        </ol>
      </main>
    </>
  );
}

function GameComponent(props: { game: Game }) {
  const url = `/img/game_covers/${props.game.game_image}`;

  return (
    <li class={gameListStyles.gameList__game} style={`--img-src: url(${url})`}>
      <img src={url} alt={`Background image of ${props.game.name}`} />
      <div class={gameListStyles.game__content}>
        <p class={gameListStyles.game__title}>{props.game.name}</p>
        <div class={gameListStyles.game__actions}>
          <A href={`/profile/${props.game.id}/`} tabIndex="-1">
            <button data-select>Select</button>
          </A>
          <A href={`/profile/${props.game.id}/`} tabIndex="-1">
            <button data-default>Set Default</button>
          </A>
        </div>
        <button class={gameListStyles.game__favoriteBtn} title="Favorite this game">
          <Fa icon={faStar} />
        </button>
      </div>
    </li>
  );
}
