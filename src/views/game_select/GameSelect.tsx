import { faStar } from "@fortawesome/free-regular-svg-icons";
import { faGlobe, faList, faTableCellsLarge } from "@fortawesome/free-solid-svg-icons";
import { useNavigate } from "@solidjs/router";
import Fa from "solid-fa";
import { createEffect, createResource, createSelector, createSignal, For, onMount } from "solid-js";

import { games, initialSortedGames } from "../../globals";
import { Locale, localeNamesMap, setLocale, locale, t, RAW_LOCALES } from "../../i18n/i18n";
import { Game } from "../../types";
// @ts-ignore: TS is unaware of `use:` directives despite using them for type definitions
import { autofocus } from "../../components/Directives";

import blobStyles from "./GameBlobs.module.css";
import gameListStyles from "./GameList.module.css";
import styles from "./GameSelect.module.css";
import { GameSortColumn, searchGames } from "../../api/api";
import { updateSettings } from "../../api/settings";
import { SimpleAsyncButton } from "../../widgets/AsyncButton";
import SelectDropdown from "../../widgets/SelectDropdown";
import Tooltip, { TooltipTrigger } from "../../widgets/Tooltip";

enum DisplayType {
  Card = -1,
  List = 1,
}

const ZOOM_ANIMATION_TIME_MS = 250;

export default function GameSelect(props: {
  replace: boolean;
  shouldShow: boolean;
  beginDismiss: () => void;
  finishDismiss: () => void;
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

  const sortMethodSelected = createSelector(sort);

  onMount(() => {
    if (!props.shouldShow) {
      // Close immediately, likely opened app into a game profile
      props.finishDismiss();
    }
  });

  createEffect(() => {
    if (!props.shouldShow) {
      // Dismiss after animation
      setTimeout(() => {
        props.finishDismiss();
      }, ZOOM_ANIMATION_TIME_MS);
    }
  });

  return (
    <div
      class={styles.page}
      classList={{ [styles.zoomOut]: props.shouldShow, [styles.zoomIn]: !props.shouldShow }}
      data-showing={props.shouldShow}
      style={{ "--duration": `${ZOOM_ANIMATION_TIME_MS}ms` }}
    >
      <div class={styles.pageInner}>
        <div class={blobStyles.gradientBlobs} aria-hidden="true">
          <div class={blobStyles.gradientBlob} data-blob-1></div>
          <div class={blobStyles.gradientBlob} data-blob-2></div>
          <div class={blobStyles.gradientBlob} data-blob-3></div>
          <div class={blobStyles.gradientBlob} data-blob-4></div>
        </div>
        <div class={styles.language}>
          <Fa icon={faGlobe} />
          <SelectDropdown
            label={{ labelText: "value" }}
            options={RAW_LOCALES.map((loc) => ({
              value: loc,
              label: localeNamesMap[loc],
              selected: () => locale() === loc,
            }))}
            onChanged={(value) => setLocale(value as Locale)}
          />
        </div>
        <header class={styles.header}>
          <h1>{t("game_select.title")}</h1>
          <p>{t("game_select.subtitle")}</p>
        </header>
        <main class={styles.main}>
          <div class={styles.gameSearch}>
            <form on:submit={(e) => e.preventDefault()} class={styles.gameSearch__content}>
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
              <SelectDropdown<GameSortColumn>
                label={{ labelText: "preset", preset: t("global.select_dropdown.sort_by") }}
                options={[
                  {
                    value: GameSortColumn.ModDownloads,
                    label: t("global.game_sort_column.mod_downloads"),
                    selected: () => sortMethodSelected(GameSortColumn.ModDownloads),
                  },
                  {
                    value: GameSortColumn.Popularity,
                    label: t("global.game_sort_column.popularity"),
                    selected: () => sortMethodSelected(GameSortColumn.Popularity),
                  },
                  {
                    value: GameSortColumn.Name,
                    label: t("global.game_sort_column.name"),
                    selected: () => sortMethodSelected(GameSortColumn.Name),
                  },
                ]}
                onChanged={setSort}
              />
              <Tooltip
                content={t("game_select.search.display_type_btn", {
                  type:
                    displayType() === DisplayType.Card
                      ? t("game_select.search.card_display_type")
                      : t("game_select.search.list_display_type"),
                })}
              >
                <TooltipTrigger onClick={() => setDisplayType((prev) => (prev * -1) as DisplayType)}>
                  {displayType() === DisplayType.Card ? <Fa icon={faList} /> : <Fa icon={faTableCellsLarge} />}
                </TooltipTrigger>
              </Tooltip>
            </form>
          </div>
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
              {(game) => <GameComponent game={games()[game]} replace={props.replace} dismiss={props.beginDismiss} />}
            </For>
          </ol>
        </main>
      </div>
    </div>
  );
}

function GameComponent(props: { game: Game; replace: boolean; dismiss: () => void }) {
  const url = `/img/game_covers/${props.game.thunderstoreId}.webp`;

  const navigate = useNavigate();

  function navigateToGame() {
    navigate(`/profile/${props.game.id}/`, { replace: props.replace });
    props.dismiss();
  }

  return (
    <li class={gameListStyles.gameList__game} style={`--img-src: url("${url}");`}>
      <img src={url} alt={t("game_select.bg_img_alt", { gameName: props.game.name })} />
      <div class={gameListStyles.game__content}>
        <p class={gameListStyles.game__title}>
          <Tooltip content={t("game_select.fav_btn")}>
            <TooltipTrigger class={gameListStyles.game__favoriteBtn}>
              <Fa icon={faStar} />
            </TooltipTrigger>
          </Tooltip>

          {props.game.name}
        </p>
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
      </div>
    </li>
  );
}
