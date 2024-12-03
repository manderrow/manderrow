import { faStar } from "@fortawesome/free-regular-svg-icons";
import { A } from "@solidjs/router";
import Fa from "solid-fa";
import { createSignal, For, Suspense } from "solid-js";

import { games } from "../../globals";
import { Game } from "../../types";

import blobStyles from "./GameBlobs.module.css";
import gameListStyle from "./GameList.module.css";
import styles from "./GameSelect.module.css";

export default function GameSelect() {
  const [displayType, setDisplayType] = createSignal<"card" | "list">("card");
  const [search, setSearch] = createSignal("");

  return (
    <>
      <style>body {"{ position: relative; z-index: 1; }"}</style>

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
            on:input={(e) => setSearch(e.target.value)}
          />
        </form>
        <ol
          classList={{
            [gameListStyle.gameList]: true,
            [gameListStyle.gameList__gameCard]: displayType() === "card",
            [gameListStyle.gameList__gameItem]: displayType() === "list",
          }}
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
    <li class={gameListStyle.gameList__game} style={`--img-src: url(${url})`}>
      <img src={url} alt={`Background image of ${props.game.name}`} />
      <div class={gameListStyle.game__content}>
        <p class={gameListStyle.game__title}>{props.game.name}</p>
        <div class={gameListStyle.game__actions}>
          <A href={`/profile/${props.game.id}/`} tabIndex="-1">
            <button>Select</button>
          </A>
          <A href={`/profile/${props.game.id}/`} tabIndex="-1">
            <button>Set Default</button>
          </A>
        </div>
        <button class={gameListStyle.game__favoriteBtn}>
          <Fa icon={faStar} />
        </button>
      </div>
    </li>
  );
}
