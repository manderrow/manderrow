import { faStar } from "@fortawesome/free-regular-svg-icons";
import { faGlobe, faList, faTableCellsLarge } from "@fortawesome/free-solid-svg-icons";
import { useLocation, useNavigate } from "@solidjs/router";
import Fa from "solid-fa";
import { createEffect, createResource, createSignal, For, onCleanup, onMount } from "solid-js";

import { games, initialGame, initialSortedGames } from "../../globals";
import { Locale, localeNamesMap, setLocale, locale, t, RAW_LOCALES } from "../../i18n/i18n";
import { Game } from "../../types";
import { autofocus } from "../../components/global/Directives";

import blobStyles from "./GameBlobs.module.css";
import gameListStyles from "./GameList.module.css";
import styles from "./GameSelect.module.css";
import { GameSortColumn, searchGames } from "../../api";
import { updateSettings } from "../../api/settings";
import { SimpleAsyncButton } from "../../components/global/AsyncButton";
import { replaceRouteState } from "../../utils/router";

enum DisplayType {
  Card = -1,
  List = 1,
}

interface GameSelectState {
  explicit?: true;
}

export default function GameSelect(props: {
  replace: boolean;
  dismiss?: () => void;
}) {
  const [displayType, setDisplayType] = createSignal<DisplayType>(DisplayType.Card);
  const [search, setSearch] = createSignal("");
  const [sort, setSort] = createSignal<GameSortColumn>(GameSortColumn.ModDownloads);

  const [filteredGames] = createResource<readonly number[], readonly [string, GameSortColumn], readonly number[]>(
    () => [search(), sort()],
    async ([query, sort]) => {
      return await searchGames(query, [
        { column: GameSortColumn.Relevance, descending: true },
        { column: sort, descending: sort !== GameSortColumn.Name },
      ]);
    },
    { initialValue: initialSortedGames() },
  );

  return (
    <div class={styles.page}>
      <div class={styles.pageInner}>
      <div class={blobStyles.gradientBlobs} aria-hidden="true">
        <div class={blobStyles.gradientBlob} data-blob-1></div>
        <div class={blobStyles.gradientBlob} data-blob-2></div>
        <div class={blobStyles.gradientBlob} data-blob-3></div>
        <div class={blobStyles.gradientBlob} data-blob-4></div>
      </div>
      <div class={styles.language}>
        <form on:submit={(e) => e.preventDefault()}>
          <label for="language" aria-label="Change language">
            <Fa icon={faGlobe} />
          </label>
          <select name="language" id="language" on:change={(e) => setLocale(e.target.value as Locale)}>
            <For each={RAW_LOCALES}>
              {(loc) => (
                <option value={loc} selected={locale() === loc}>
                  {localeNamesMap[loc]}
                </option>
              )}
            </For>
          </select>
        </form>
      </div>
      <header class={styles.header}>
        <h1>{t("game_select.title")}</h1>
        <p>{t("game_select.subtitle")}</p>
      </header>
      <main class={styles.main}>
        <form on:submit={(e) => e.preventDefault()} class={styles.gameSearch}>
          <input
            type="search"
            name="search-game"
            id="search-game"
            placeholder={t("game_select.search.input_placeholder")}
            value={search()}
            maxlength="100"
            use:autofocus
            on:input={(e) => setSearch(e.target.value)}
          />
          <select name="sort-type" id="sort-type" on:input={(e) => setSort(e.target.value as GameSortColumn)}>
            <option value={GameSortColumn.ModDownloads} selected>
              {t("global.game_sort_column.mod_downloads")}
            </option>
            <option value={GameSortColumn.Popularity}>{t("global.game_sort_column.popularity")}</option>
            <option value={GameSortColumn.Name}>{t("global.game_sort_column.name")}</option>
          </select>
          <button
            type="button"
            on:click={() => setDisplayType((prev) => (prev * -1) as DisplayType)}
            title={t("game_select.search.display_type_btn", {
              type:
                displayType() === DisplayType.Card
                  ? t("game_select.search.card_display_type")
                  : t("game_select.search.list_display_type"),
            })}
          >
            {displayType() === DisplayType.Card ? <Fa icon={faList} /> : <Fa icon={faTableCellsLarge} />}
          </button>
        </form>
        <ol
          classList={{
            [gameListStyles.gameList]: true,
            [gameListStyles.searching]: search().length > 0,
            [gameListStyles.gameList__gameCard]: displayType() === DisplayType.Card,
            [gameListStyles.gameList__gameItem]: displayType() === DisplayType.List,
          }}
        >
          <For
            each={filteredGames.latest}
            fallback={
              <li class={gameListStyles.gameList__empty}>
                <p>{t("game_select.no_games_msg")}</p>
              </li>
            }
          >
            {(game) => <GameComponent game={games()[game]} replace={props.replace} dismiss={props.dismiss} />}
          </For>
        </ol>
      </main>
      </div>
    </div>
  );
}

function GameComponent(props: {
  game: Game;
  replace: boolean;
  dismiss?: () => void;
}) {
  const url = `/img/game_covers/${props.game.thunderstoreId}.webp`;

  const navigate = useNavigate();

  function navigateToGame() {
    navigate(`/profile/${props.game.id}/`, { replace: props.replace });
    props.dismiss?.();
  }

  return (
    <li class={gameListStyles.gameList__game} style={`--img-src: url("${url}");`}>
      <img src={url} alt={t("game_select.bg_img_alt", { gameName: props.game.name })} />
      <div class={gameListStyles.game__content}>
        <p class={gameListStyles.game__title}>{props.game.name}</p>
        <div class={gameListStyles.game__actions}>
          <button data-select on:click={navigateToGame}>
            {t("game_select.select_btn")}
          </button>
          <SimpleAsyncButton
            data-default
            onClick={async () => {
              await updateSettings({ defaultGame: { override: props.game.id } });
              navigateToGame();
            }}
          >
            {t("game_select.set_default_btn")}
          </SimpleAsyncButton>
        </div>
        <button class={gameListStyles.game__favoriteBtn} title={t("game_select.fav_btn")}>
          <Fa icon={faStar} />
        </button>
      </div>
    </li>
  );
}
