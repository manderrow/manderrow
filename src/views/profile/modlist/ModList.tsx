import { faCircleUp, faRefresh } from "@fortawesome/free-solid-svg-icons";
import { createInfiniteScroll } from "@solid-primitives/pagination";
import { Fa } from "solid-fa";
import {
  Accessor,
  For,
  InitializedResource,
  JSX,
  Match,
  ResourceFetcherInfo,
  Show,
  Signal,
  Switch,
  createContext,
  createEffect,
  createMemo,
  createResource,
  createSelector,
  createSignal,
  untrack,
  useContext,
} from "solid-js";

import {
  ModId,
  ModSortColumn,
  SortOption,
  countModIndex,
  fetchModIndex,
  getFromModIndex,
  modIdEquals,
  queryModIndex,
} from "../../../api/api";
import { Progress, createProgressProxyStore } from "../../../api/tasks";
import { Mod, ModPackage } from "../../../types";
import { createMultiselectableList, numberFormatter } from "../../../utils/utils";

import { ActionContext, SimpleAsyncButton } from "../../../widgets/AsyncButton";
import ModSearch from "./ModSearch.tsx";

import styles from "./ModList.module.css";
import { t } from "../../../i18n/i18n.ts";
import { DialogTrigger } from "../../../widgets/Dialog.tsx";
import { ErrorIndicator } from "../../../components/ErrorDialog.tsx";
import { SimpleProgressIndicator } from "../../../widgets/Progress.tsx";

import { useSearchParamsInPlace } from "../../../utils/router.ts";
import ModListItem from "./ModListItem.tsx";
import ModUpdateDialogue, { ModUpdate } from "./Updater.tsx";
import ModView from "./ModView.tsx";
import BulkActions from "./BulkActions.tsx";

type PageFetcher = (page: number) => Promise<readonly Mod[]>;
type ModFetcherResult = {
  count: number;
  mods: PageFetcher;
  get: (id: ModId) => Promise<Mod | undefined> | Mod | undefined;
};

export type Fetcher = (
  game: string,
  query: string,
  sort: readonly SortOption<ModSortColumn>[],
) => Promise<ModFetcherResult>;

export const ModInstallContext = createContext<{
  profileId: Accessor<string>;
  installed: InitializedResource<readonly ModPackage[]>;
  refetchInstalled: () => Promise<void>;
}>();

export const enum ModSelectorTutorialState {
  INIT,
  HOVERED,
  LEFT,
}

const MODS_PER_PAGE = 50;

export function OnlineModList(props: { game: string }) {
  const [progress, setProgress] = createProgressProxyStore();

  const [loadStatus, { refetch: refetchModIndex }] = createResource(
    () => props.game,
    async (game, info: ResourceFetcherInfo<boolean, never>) => {
      await fetchModIndex(game, { refresh: info.refetching }, (event) => {
        if (event.event === "created") {
          setProgress(event.progress);
        }
      });
      return true;
    },
  );

  const getFetcher: () => Fetcher = () => {
    // track load status
    loadStatus.latest;

    return async (game, query, sort) => {
      const count = await countModIndex(game, query);

      return {
        count,
        mods: async (page: number) =>
          (
            await queryModIndex(game, query, sort, {
              skip: page * MODS_PER_PAGE,
              limit: MODS_PER_PAGE,
            })
          ).mods,
        get: async (id) => (await getFromModIndex(game, [id]))[0],
      };
    };
  };

  return (
    <ModList
      game={props.game}
      isLoading={loadStatus.loading}
      progress={progress}
      refresh={async () => {
        await refetchModIndex();
      }}
      mods={getFetcher()}
      multiselect={false}
    />
  );
}

const CHECK_UPDATES_REFETCH: unique symbol = Symbol();

export function InstalledModList(props: { game: string }) {
  const [_, setSearchParams] = useSearchParamsInPlace();

  const context = useContext(ModInstallContext)!;

  const [checkUpdatesProgress, setCheckUpdatesProgress] = createProgressProxyStore();

  const [updates, { refetch: refreshUpdates }] = createResource<ModUpdate[], true, typeof CHECK_UPDATES_REFETCH>(
    () => {
      try {
        // track
        context.installed.latest;
      } catch {}
      return true;
    },
    async (_, info) => {
      await fetchModIndex(props.game, { refresh: info.refetching === CHECK_UPDATES_REFETCH }, (event) => {
        if (event.event === "progress") {
          setCheckUpdatesProgress(event.progress);
        }
      });

      const installedMods = untrack(() => context.installed.latest);

      const latestMods = await getFromModIndex(
        props.game,
        installedMods.map((mod) => ({ owner: mod.owner, name: mod.name })),
      );

      return latestMods
        .map((mod, i) => ({ newMod: mod, oldVersionNumber: installedMods[i].version.version_number }))
        .filter((update) => update.newMod.versions[0].version_number !== update.oldVersionNumber);
    },
    { initialValue: [] },
  );

  const [progress, _setProgress] = createProgressProxyStore();

  const getFetcher: () => Fetcher = () => {
    const data = context.installed.latest;

    return async (_game, _query, _sort) => {
      // TODO: implement filter and sort

      return {
        count: data.length,
        mods: async (page) => (page === 0 ? data : []),
        get: (id) => data.find((mod) => modIdEquals(mod, id)),
      };
    };
  };

  async function checkUpdates() {
    await refreshUpdates(CHECK_UPDATES_REFETCH);
  }

  return (
    <Show
      when={context.installed.latest.length !== 0}
      fallback={
        <div class={styles.noModsMessage}>
          <h2>{t("modlist.installed.no_mods_title")}</h2>
          <p>{t("modlist.installed.no_mods_msg")}</p>

          <div class={styles.noModsMessage__btns}>
            <button data-btn="primary" onClick={() => setSearchParams({ "profile-tab": "mod-search" })}>
              {t("modlist.installed.browse_btn")}
            </button>
          </div>
        </div>
      }
    >
      <ModList
        game={props.game}
        isLoading={false}
        progress={progress}
        refresh={context.refetchInstalled}
        mods={getFetcher()}
        multiselect={true}
        trailingControls={
          <Show
            when={updates().length > 0}
            fallback={
              <SimpleAsyncButton
                btnStyle="ghost"
                busy={updates.loading}
                progress={checkUpdatesProgress}
                onClick={checkUpdates}
              >
                <Fa icon={faRefresh} /> {t("modlist.installed.check_updates_btn")}
              </SimpleAsyncButton>
            }
          >
            <ModUpdateDialogue updates={updates()}>
              <DialogTrigger data-btn="primary">
                <Fa icon={faCircleUp} /> {t("modlist.installed.updates_available_btn")}
              </DialogTrigger>
            </ModUpdateDialogue>
          </Show>
        }
      />
    </Show>
  );
}

function ModList(props: {
  game: string;
  mods: Fetcher;
  refresh: () => Promise<void> | void;
  isLoading: boolean;
  progress: Progress;
  trailingControls?: JSX.Element;
  multiselect: boolean;
}) {
  const [focusedModId, setFocusedModId] = createSignal<ModId>();

  const [profileSortOrder, setProfileSortOrder] = createSignal(false);
  const [query, setQuery] = createSignal("");

  const [sort, setSort] = createSignal<readonly SortOption<ModSortColumn>[]>([
    { column: ModSortColumn.Relevance, descending: true },
    { column: ModSortColumn.Downloads, descending: true },
    { column: ModSortColumn.Name, descending: false },
    { column: ModSortColumn.Owner, descending: false },
    { column: ModSortColumn.Size, descending: true },
  ]);

  const [queriedMods] = createResource(
    () => {
      try {
        props.mods;
      } catch {}
      return [props.game, query(), sort()] as [string, string, readonly SortOption<ModSortColumn>[]];
    },
    ([game, query, sort]) => untrack(() => props.mods)(game, query, sort),
    { initialValue: { mods: async (_: number) => [], count: 0, get: (_: ModId) => undefined } },
  );

  const [focusedMod] = createResource(
    () => {
      try {
        queriedMods.latest;
      } catch {}
      return focusedModId() ?? {};
    },
    (id) => ("owner" in id ? untrack(() => queriedMods.latest.get(id as ModId)) : undefined),
  );

  createEffect(() => {
    const mod = focusedModId();

    if (mod) {
      let mods;
      try {
        mods = queriedMods.latest;
      } catch {}

      (async () => {
        if (mods) {
          if (await mods.get(mod)) {
            return;
          }
        }

        setFocusedModId(undefined);
      })();
    }
  });

  const installContext = useContext(ModInstallContext)!;

  const {
    onCtrlClickItem,
    onShiftClickItem,
    clearSelection,
    isPivot,
    isSelected,
    data,
    delete: deleteSelectedMod,
  } = createMultiselectableList<ModPackage, string, ModId & { version: string }>(
    () => installContext.installed.latest,
    (mod) => `${mod.owner}-${mod.name}`,
    (mod) => ({ owner: mod.owner, name: mod.name, version: mod.version.version_number }),
    () => undefined,
  );

  return (
    <div class={styles.modListAndView}>
      <div class={styles.modListContainer}>
        <ModSearch
          game={props.game}
          query={query()}
          setQuery={setQuery}
          sort={sort()}
          setSort={setSort}
          profileSortOrder={profileSortOrder()}
          setProfileSortOrder={setProfileSortOrder}
        />

        <div class={styles.discoveredLine}>
          <Switch>
            <Match when={props.isLoading || queriedMods.loading}>
              <span>{t("modlist.fetching_msg")}</span>
              <SimpleProgressIndicator progress={props.progress} />
            </Match>
            <Match when={queriedMods.error}>
              {(err) => <ErrorIndicator icon={true} message="Query failed" err={err()} reset={props.refresh} />}
            </Match>
            <Match when={queriedMods.latest}>
              <span>{t("modlist.discovered_msg", { count: numberFormatter.format(queriedMods()!.count) })}</span>
              <ActionContext>
                {(busy, wrapOnClick) => (
                  <button class={styles.refreshButton} disabled={busy()} on:click={() => wrapOnClick(props.refresh)}>
                    <Fa icon={faRefresh} />
                  </button>
                )}
              </ActionContext>
            </Match>
          </Switch>

          <Show when={props.trailingControls}>
            <div class={styles.trailingControls}>{props.trailingControls}</div>
          </Show>
        </div>

        <Show when={queriedMods.error === undefined && queriedMods()} keyed>
          {(mods) => (
            <ModListMods
              mods={mods.mods}
              focusedMod={[focusedModId, setFocusedModId]}
              isSelected={props.multiselect ? isSelected : undefined}
              select={props.multiselect ? onCtrlClickItem : undefined}
              shiftClick={props.multiselect ? onShiftClickItem : undefined}
              deleteSelectedMod={props.multiselect ? deleteSelectedMod : undefined}
              isPivot={props.multiselect ? isPivot : undefined}
              forceSelectorVisibility={data().length !== 0}
            />
          )}
        </Show>
      </div>

      <div class={styles.modViewContainer}>
        <Switch
          fallback={
            <div class={styles.nothingMsg}>
              <h2>{t("modlist.modview.no_mod_selected_title")}</h2>
              <p>{t("modlist.modview.no_mod_selected_subtitle")}</p>
            </div>
          }
        >
          <Match when={focusedMod.error === undefined && focusedMod()} keyed>
            {(mod) => <ModView mod={mod} gameId={props.game} closeModView={() => setFocusedModId(undefined)} />}
          </Match>
          <Match when={props.multiselect && data().length !== 0}>
            <BulkActions mods={data} clearSelection={clearSelection} deleteSelectedMod={deleteSelectedMod} />
          </Match>
        </Switch>
      </div>
    </div>
  );
}

export type SelectableModListProps = {
  /// Whether the mod is selected in the ModList (for bulk actions)
  isSelected: (mod: ModPackage) => boolean;
  select: (item: ModPackage, index: number) => void;
  shiftClick: (item: ModPackage, index: number) => void;
  isPivot: (mod: ModPackage | undefined) => boolean;
  deleteSelectedMod: (mod: ModPackage) => void;
  forceSelectorVisibility: boolean;
};

function ModListMods(
  props: { mods: PageFetcher; focusedMod: Signal<ModId | undefined> } & Partial<SelectableModListProps>,
) {
  const infiniteScroll = createMemo(() => {
    // this should take readonly, which would make the cast unnecessary
    return createInfiniteScroll(props.mods as (page: number) => Promise<Mod[]>);
  });
  const paginatedMods = () => infiniteScroll()[0]();
  // idk why we're passing props here
  // @ts-ignore: static analysis doesn't understand `use:` directives
  const infiniteScrollLoader = (el: Element) => infiniteScroll()[1](el);
  const end = () => infiniteScroll()[2].end();

  const isFocusedMod = createSelector<ModId | undefined, ModId>(
    props.focusedMod[0],
    (a, b) => b !== undefined && modIdEquals(a, b),
  );

  const [modSelectorTutorialState, setModSelectorTutorialState] = createSignal(ModSelectorTutorialState.INIT);

  function onMouseEnter() {
    if (modSelectorTutorialState() == ModSelectorTutorialState.INIT) {
      setModSelectorTutorialState(ModSelectorTutorialState.HOVERED);
    }
  }

  function onMouseLeave() {
    if (modSelectorTutorialState() == ModSelectorTutorialState.HOVERED) {
      setModSelectorTutorialState(ModSelectorTutorialState.LEFT);
    }
  }

  return (
    <ol class={styles.modList} onMouseEnter={onMouseEnter} onMouseLeave={onMouseLeave}>
      <For each={paginatedMods()}>
        {(mod, i) => (
          <ModListItem
            mod={mod}
            isFocused={isFocusedMod}
            setFocused={props.focusedMod[1]}
            isSelected={props.isSelected ? () => props.isSelected!(mod as ModPackage) : undefined}
            select={() => props.select!(mod as ModPackage, i())}
            shiftClick={() => props.shiftClick!(mod as ModPackage, i())}
            deleteSelectedMod={() => props.deleteSelectedMod!(mod as ModPackage)}
            isPivot={props.isPivot?.(mod as ModPackage)}
            forceSelectorVisibility={props.forceSelectorVisibility || modSelectorTutorialState() < 2}
          />
        )}
      </For>
      <Show when={!end()}>
        <li use:infiniteScrollLoader>{t("global.phrases.loading")}...</li>
      </Show>
    </ol>
  );
}
