import { faHardDrive, faHeart } from "@fortawesome/free-regular-svg-icons";
import {
  faArrowRightLong,
  faCircleUp,
  faDownLong,
  faDownload,
  faExternalLink,
  faRefresh,
  faTrash,
  faXmark,
} from "@fortawesome/free-solid-svg-icons";
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
  installProfileMod,
  modIdEquals,
  queryModIndex,
  uninstallProfileMod,
} from "../../api";
import { Progress, createProgressProxyStore, initProgress, registerTaskListener, tasks } from "../../api/tasks";
import { Mod, ModListing, ModPackage, ModVersion } from "../../types";
import {
  dateFormatterMed,
  humanizeFileSize,
  numberFormatter,
  removeProperty,
  roundedNumberFormatter,
} from "../../utils";

import { ActionContext, ProgressStyle, SimpleAsyncButton } from "../global/AsyncButton";
import ErrorBoundary from "../global/ErrorBoundary.tsx";
import TabRenderer, { Tab, TabContent } from "../global/TabRenderer";
import ModMarkdown from "./ModMarkdown.tsx";
import ModSearch from "./ModSearch.tsx";

import styles from "./ModList.module.css";
import { t } from "../../i18n/i18n.ts";
import { DefaultDialog } from "../global/Dialog.tsx";
import { ErrorIndicator } from "../global/ErrorDialog.tsx";
import { SimpleProgressIndicator } from "../global/Progress.tsx";
import SelectDropdown from "../global/SelectDropdown.tsx";
import TogglableDropdown from "../global/TogglableDropdown.tsx";
import Tooltip from "../global/Tooltip.tsx";
import { useSearchParamsInPlace } from "../../utils/router.ts";
import Checkbox from "../global/Checkbox.tsx";

type PageFetcher = (page: number) => Promise<readonly Mod[]>;
export type Fetcher = (
  game: string,
  query: string,
  sort: readonly SortOption<ModSortColumn>[],
) => Promise<{
  count: number;
  mods: PageFetcher;
  get: (id: ModId) => Promise<Mod | undefined> | Mod | undefined;
}>;

export const ModInstallContext = createContext<{
  profileId: Accessor<string>;
  installed: InitializedResource<readonly ModPackage[]>;
  refetchInstalled: () => Promise<void>;
}>();

const MODS_PER_PAGE = 50;

export default function ModList(props: {
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

  const [selectedMods, setSelectedMods] = createSignal<Map<ModId, string>>(new Map(), {
    equals: false,
  });

  const isSelectedMod = createSelector<Map<ModId, string>, ModId>(selectedMods, (id, selected) => selected.has(id));

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
              <span>Fetching mods</span>
              <SimpleProgressIndicator progress={props.progress} />
            </Match>
            <Match when={queriedMods.error}>
              {(err) => <ErrorIndicator icon={true} message="Query failed" err={err()} reset={props.refresh} />}
            </Match>
            <Match when={queriedMods.latest}>
              <span>Discovered {numberFormatter.format(queriedMods()!.count)} mods</span>
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
              isSelected={props.multiselect ? isSelectedMod : undefined}
              setSelected={
                props.multiselect
                  ? (mod, version, selected) => {
                      setSelectedMods((set) => {
                        if (selected) {
                          set.set(mod, version);
                        } else {
                          set.delete(mod);
                        }
                        return set;
                      });
                    }
                  : undefined
              }
            />
          )}
        </Show>
      </div>

      <div class={styles.modViewContainer}>
        <div class={styles.modView}>
          <Switch
            fallback={
              <div class={styles.nothingMsg}>
                <h2>No mod selected</h2>
                <p>Select a mod to it view here.</p>
              </div>
            }
          >
            <Match when={focusedMod.error === undefined && focusedMod()} keyed>
              {(mod) => <ModView mod={mod} gameId={props.game} closeModView={() => setFocusedModId(undefined)} />}
            </Match>
            <Match when={props.multiselect && selectedMods().size !== 0}>
              <SelectedModsList mods={selectedMods} />
            </Match>
          </Switch>
        </div>
      </div>
    </div>
  );
}

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

interface ModUpdate {
  newMod: ModListing;
  oldVersionNumber: string;
}

const CHECK_UPDATES_REFETCH: unique symbol = Symbol();

export function InstalledModList(props: { game: string }) {
  const [_, setSearchParams] = useSearchParamsInPlace();

  const context = useContext(ModInstallContext)!;
  const [updaterShown, setUpdaterShown] = createSignal(false);

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
                style="ghost"
                busy={updates.loading}
                progress={checkUpdatesProgress}
                onClick={checkUpdates}
              >
                <Fa icon={faRefresh} /> {t("modlist.installed.check_updates_btn")}
              </SimpleAsyncButton>
            }
          >
            <button data-btn="primary" onClick={() => setUpdaterShown(true)}>
              <Fa icon={faCircleUp} /> {t("modlist.installed.updates_available_btn")}
            </button>
          </Show>
        }
      />

      <Show when={updaterShown()}>
        <ModUpdateDialogue
          onDismiss={() => {
            setUpdaterShown(false);
          }}
          updates={updates()}
        />
      </Show>
    </Show>
  );
}

function ModUpdateDialogue(props: { onDismiss: () => void; updates: ModUpdate[] }) {
  const [_progress, _setProgress] = createProgressProxyStore();

  const [selectedMods, setSelectedMods] = createSignal<Set<ModUpdate>>(new Set(props.updates), {
    equals: false,
  });

  return (
    <DefaultDialog onDismiss={props.onDismiss} class={styles.updateDialog}>
      <h2>{t("modlist.installed.updater_title")}</h2>

      <div class={styles.listContainer}>
        <form action="#">
          <fieldset>
            <input
              type="checkbox"
              id="update-select-all-mods"
              checked={selectedMods().size === props.updates.length}
              onInput={(e) => {
                if (e.target.checked) {
                  setSelectedMods(new Set(props.updates));
                } else {
                  setSelectedMods(new Set<ModUpdate>());
                }
              }}
            />
            <label for="update-select-all-mods">Select All</label>
          </fieldset>
          <fieldset>
            <label for="update-search" class="phantom">
              Search
            </label>
            <input type="text" id="update-search" placeholder="Search mod..." />
          </fieldset>
        </form>
        <ul>
          {props.updates.map((update) => (
            <li>
              <label for={update.newMod.name}>
                <input
                  id={update.newMod.name}
                  type="checkbox"
                  checked={selectedMods().has(update)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      setSelectedMods((selectedMods) => selectedMods.add(update));
                    } else {
                      setSelectedMods((selectedMods) => {
                        selectedMods.delete(update);
                        return selectedMods;
                      });
                    }
                  }}
                />
                <img
                  width={48}
                  height={48}
                  alt="mod icon"
                  src={getIconUrl(update.newMod.owner, update.newMod.name, update.newMod.versions[0].version_number)}
                />
                <div class={styles.updateMetadata}>
                  <p data-name>{update.newMod.name}</p>
                  <p data-owner>{update.newMod.owner}</p>
                  <p data-version>
                    <span data-old-version>{update.oldVersionNumber}</span>
                    <span data-arrow>
                      <Fa icon={faArrowRightLong} />
                    </span>
                    <span data-new-version>{update.newMod.versions[0].version_number}</span>
                  </p>
                </div>
              </label>
            </li>
          ))}
        </ul>
      </div>

      <div class={styles.updateBtns}>
        <button data-btn="primary">
          {selectedMods().size === props.updates.length
            ? t("modlist.installed.update_all_btn")
            : t("modlist.installed.update_selected_btn")}
        </button>
        <button onClick={props.onDismiss} style={{ order: -1 }} data-btn="ghost">
          {t("global.phrases.cancel")}
        </button>
      </div>
    </DefaultDialog>
  );
}

const MAX_SELECTED_CARDS = 5;
function SelectedModsList(props: { mods: Accessor<Map<ModId, string>> }) {
  const selectedCount = () => props.mods().size;

  return (
    <>
      <div class={styles.selected__cards} aria-hidden>
        <ul
          class={styles.cards__list}
          style={{
            "--total-cards": Math.min(selectedCount(), MAX_SELECTED_CARDS),
          }}
        >
          <For each={Array.from(props.mods()).slice(0, MAX_SELECTED_CARDS)}>
            {([modId, version], i) => {
              return (
                <li
                  class={styles.selected__card}
                  style={{
                    "--card-index": i(),
                    "--bg-image": `url("${getIconUrl(modId.owner, modId.name, version)}")`,
                  }}
                >
                  {i() == MAX_SELECTED_CARDS - 1 ? `+${selectedCount() - MAX_SELECTED_CARDS + 1}` : undefined}
                </li>
              );
            }}
          </For>
        </ul>
      </div>

      <h2 class={styles.selected__title}>{t("modlist.installed.multiselect_title")}</h2>

      <ul>
        <li>
          {t(
            selectedCount() > 1 ? "modlist.installed.selected_count_plural" : "modlist.installed.selected_count_single",
            { count: selectedCount() },
          )}
        </li>
        <li>
          <Fa icon={faHardDrive} />{" "}
          {humanizeFileSize(
            Array.from(props.mods()).reduce((total, [_modId, _version]) => total /*+ getModById(modId).file_size*/, 0),
          )}
        </li>
      </ul>

      <div class={styles.selected__actions}>
        <button>{t("modlist.installed.enable_selected")}</button>
        <button>{t("modlist.installed.disable_selected")}</button>
        <button>{t("modlist.installed.delete_selected")}</button>
        <button>{t("modlist.installed.update_selected")}</button>
      </div>
    </>
  );
}

function ModView(props: { mod: Mod; gameId: string; closeModView: () => void }) {
  const [_progress, setProgress] = createProgressProxyStore();

  function getInitialModListing(mod: Mod) {
    if ("version" in mod) {
      const obj: ModListing & { game?: string; version?: ModVersion } = { ...mod, versions: [mod.version] };
      delete obj.version;
      delete obj.game;
      return obj;
    } else {
      return mod;
    }
  }

  const [modListing] = createResource<ModListing | undefined, Mod | Record<never, never>, never>(
    () => props.mod,
    async (mod) => {
      if ("version" in mod) {
        await fetchModIndex(props.gameId, { refresh: false }, (event) => {
          if (event.event === "created") {
            setProgress(event.progress);
          }
        });
        return (await getFromModIndex(props.gameId, [{ owner: mod.owner, name: mod.name }]))[0];
      } else if ("versions" in mod) {
        setProgress(initProgress());
        return mod;
      }
    },
    { initialValue: getInitialModListing(props.mod) },
  );

  const [selectedVersion, setSelectedVersion] = createSignal<string>();

  const modVersionData = () => {
    const selected = selectedVersion();

    const versions = "versions" in props.mod ? props.mod.versions : modListing?.latest?.versions;

    // If online, display the mod listing. Otherwise, in installed, display mod package initially, then
    // mod listing after it loads. Coincidentally, this undefined logic check works as the mod listing
    // loads after the initial mod package is always displayed by default first
    return (
      (selected === undefined ? undefined : versions?.find((v) => v.version_number === selected)) ??
      ("versions" in props.mod ? props.mod.versions[0] : props.mod.version)
    );
  };

  const tabs: Tab<"overview" | "dependencies" | "changelog">[] = [
    {
      id: "overview",
      name: "Overview",
      component: () => <ModMarkdown mod={props.mod} selectedVersion={selectedVersion()} endpoint="readme" />,
    },
    {
      id: "dependencies",
      name: "Dependencies",
      component: () => <ModViewDependencies dependencies={modVersionData().dependencies} />,
    },
    {
      id: "changelog",
      name: "Changelog",
      component: () => <ModMarkdown mod={props.mod} selectedVersion={selectedVersion()} endpoint="changelog" />,
    },
  ];

  const [currentTab, setCurrentTab] = createSignal(tabs[0].id);
  const isCurrentTab = createSelector(currentTab);

  const installContext = useContext(ModInstallContext);

  const isSelectedVersion = createSelector(selectedVersion);

  const installed = useInstalled(installContext, () => props.mod);

  return (
    <>
      <div class={styles.modSticky}>
        <div class={styles.modMeta}>
          {/* TODO: For local mod with no package URL, remove link */}
          <div style={{ "grid-area": "name" }}>
            <a
              href={`https://thunderstore.io/c/${props.gameId}/p/${props.mod.owner}/${props.mod.name}/`}
              target="_blank"
              rel="noopener noreferrer"
              class={styles.modMetaLink}
            >
              <h2 class={styles.name}>{props.mod.name}</h2>
              <Fa icon={faExternalLink} />
            </a>
          </div>
          <div style={{ "grid-area": "owner" }}>
            <a
              href={`https://thunderstore.io/c/${props.gameId}/p/${props.mod.owner}/`}
              target="_blank"
              rel="noopener noreferrer"
              class={styles.modMetaLink}
            >
              {props.mod.owner}
              <Fa icon={faExternalLink} />
            </a>
          </div>
          <ul class={styles.modMetadata}>
            <li class={styles.metadata__field}>v{modVersionData().version_number}</li>
            <li class={styles.metadata__field}>
              <Fa icon={faDownload} /> {roundedNumberFormatter.format(modVersionData().downloads)}
            </li>
            <li class={styles.metadata__field}>
              <Fa icon={faHardDrive} /> {humanizeFileSize(modVersionData().file_size)}
            </li>
          </ul>

          <Show when={props.mod.donation_link != null}>
            <a class={styles.modMeta__donate} href={props.mod.donation_link} target="_blank" rel="noopener noreferrer">
              <Fa icon={faHeart} class={styles.donate__icon} />
              <br /> Donate
            </a>
          </Show>

          <button style={{ "grid-area": "close" }} class={styles.modMeta__closeBtn} onClick={props.closeModView}>
            <Fa icon={faXmark} />
          </button>
        </div>

        <TabRenderer
          id="mod-view"
          tabs={tabs}
          styles={{
            preset: "base",
            classes: {
              container: styles.tabs,
              tab: styles.tabs__tab,
            },
          }}
          setter={(tab) => setCurrentTab(tab.id)}
        />
      </div>

      <div class={styles.modView__content}>
        <TabContent isCurrentTab={isCurrentTab} tabs={tabs} />
      </div>

      <form class={styles.modView__form} action="#">
        <Show
          when={installed()}
          fallback={
            <div class={styles.modView__onlineActions}>
              <SelectDropdown<string>
                options={
                  modListing.latest?.versions.map((version, i) => ({
                    label: version.version_number,
                    value: version.version_number,
                    selected: () =>
                      selectedVersion() == null && i === 0 ? true : isSelectedVersion(version.version_number),
                    liContent: (
                      <div>
                        <p data-version>{version.version_number}</p>
                        <p data-date>{dateFormatterMed.format(new Date(version.date_created))}</p>
                      </div>
                    ),
                  })) ?? []
                }
                label={{ labelText: "value" }}
                labelClass={styles.modView__versions}
                onChanged={(value) => setSelectedVersion(value)}
                liClass={styles.modView__versionsItem}
              />
              <InstallButton
                mod={props.mod as ModListing}
                installContext={installContext!}
                class={styles.modView__downloadBtn}
              >
                Download
              </InstallButton>
            </div>
          }
        >
          <div class={styles.modView__installedActions}>
            <TogglableDropdown
              label={t("modlist.installed.change_version_btn")}
              labelClass={styles.modView__versionLabel}
              floatingContainerClass={styles.modView__versionsDropdown}
              dropdownClass={styles.modView__versionsDropdownContent}
            >
              <input
                type="text"
                name="version-search"
                id="version-search"
                placeholder={t("modlist.installed.search_version_placeholder")}
              />
              <label for="version-search" class="phantom">
                {t("modlist.installed.search_version_placeholder")}
              </label>

              <Show when={modListing.latest}>
                {(listing) => (
                  <>
                    <SelectDropdown
                      label={{ labelText: "value" }}
                      onChanged={setSelectedVersion}
                      options={(listing().versions ?? []).map((version) => ({
                        label: version.version_number,
                        value: version.version_number,
                        selected: () => isSelectedVersion(version.version_number),
                      }))}
                    />

                    <InstallButton
                      mod={
                        {
                          ...removeProperty(listing(), "versions"),
                          version:
                            listing().versions.find((v) => v.version_number === selectedVersion()) ??
                            listing().versions[0],
                        } as ModPackage
                      }
                      installContext={installContext!}
                      class={styles.downloadBtn}
                    >
                      Apply
                    </InstallButton>
                  </>
                )}
              </Show>
            </TogglableDropdown>
            <UninstallButton mod={installed()!} installContext={installContext!} class={styles.modView__uninstallBtn}>
              {t("modlist.installed.uninstall_btn")}
            </UninstallButton>
          </div>
        </Show>
      </form>
    </>
  );
}

function ModViewDependencies(props: { dependencies: string[] }) {
  return (
    <ul class={styles.modDeps}>
      <For each={props.dependencies}>
        {(dependency) => (
          <li class={styles.dependency}>
            <img src="" alt="" class={styles.dependencyIcon} />
            {dependency}
          </li>
        )}
      </For>
    </ul>
  );
}

type SelectableModListProps = {
  /// Whether the mod is selected in the ModList (for bulk actions)
  isSelected: (mod: ModId) => boolean;
  setSelected: (mod: ModId, version: string, selected: boolean) => void;
};

function ModListMods(
  props: { mods: PageFetcher; focusedMod: Signal<ModId | undefined> } & (SelectableModListProps | {}),
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
        {(mod) => (
          <ModListItem
            mod={mod}
            isFocused={isFocusedMod}
            setFocused={props.focusedMod[1]}
            isSelected={(props as any).isSelected}
            setSelected={(props as any).setSelected}
            modSelectorTutorialState={modSelectorTutorialState()}
          />
        )}
      </For>
      <Show when={!end()}>
        <li use:infiniteScrollLoader>Loading...</li>
      </Show>
    </ol>
  );
}

function getIconUrl(owner: string, name: string, version: string) {
  return `https://gcdn.thunderstore.io/live/repository/icons/${owner}-${name}-${version}.png`;
}

function useInstalled(
  installContext: typeof ModInstallContext.defaultValue,
  modAccessor: Accessor<Mod>,
): Accessor<ModPackage | undefined> {
  return createMemo(() => {
    const mod = modAccessor();
    if ("version" in mod) {
      return mod;
    } else {
      return installContext?.installed.latest.find((pkg) => pkg.owner === mod.owner && pkg.name === mod.name);
    }
  });
}

const enum ModSelectorTutorialState {
  INIT,
  HOVERED,
  LEFT,
}

function ModListItem(
  props: {
    mod: Mod;
    /// Whether the mod is focused in the ModView
    isFocused: (mod: ModId) => boolean;
    setFocused: (mod: ModId | undefined) => void;
    modSelectorTutorialState: ModSelectorTutorialState;
    // setModSelectorTutorialState: (hovered: boolean) => void,
  } & (SelectableModListProps | {}),
) {
  const displayVersion = createMemo(() => {
    if ("version" in props.mod) return props.mod.version;
    return props.mod.versions[0];
  });

  const installContext = useContext(ModInstallContext);
  const installed = useInstalled(installContext, () => props.mod);

  function onFocus() {
    const isFocused = props.isFocused(props.mod);

    props.setFocused(
      isFocused
        ? undefined
        : {
            owner: props.mod.owner,
            name: props.mod.name,
          },
    );
  }

  return (
    <li
      classList={{
        [styles.mod]: true,
        [styles.selected]: props.isFocused(props.mod),
      }}
    >
      <div
        on:click={onFocus}
        onKeyDown={(key) => {
          if (key.key === "Enter") onFocus();
        }}
        class={styles.mod__btn}
        role="button"
        aria-pressed={props.isFocused(props.mod)}
        tabIndex={0}
      >
        <Show when={(props as any).isSelected !== undefined}>
          <div class={styles.mod__selector} data-always-show={props.modSelectorTutorialState < 2 ? "" : undefined}>
            <Checkbox
              checked={(props as SelectableModListProps).isSelected(props.mod)}
              onChange={(checked) => (props as SelectableModListProps).setSelected(props.mod, displayVersion().version_number, checked)}
              labelClass={styles.mod__selectorClickRegion}
              iconContainerClass={styles.mod__selectorIndicator}
            />
          </div>
        </Show>
        <div class={styles.mod__btnContent}>
          <img
            class={styles.icon}
            width={64}
            alt="mod icon"
            src={getIconUrl(props.mod.owner, props.mod.name, displayVersion().version_number)}
          />
          <div class={styles.mod__content}>
            <div class={styles.left}>
              <p class={styles.info}>
                <span class={styles.name}>{props.mod.name}</span>
                <span class={styles.separator} aria-hidden>
                  &bull;
                </span>
                <span class={styles.medHierarchy}>{props.mod.owner}</span>
                <Show when={"version" in props.mod}>
                  <span class={styles.separator} aria-hidden>
                    &bull;
                  </span>
                  <span class={styles.version}>{(props.mod as ModPackage).version.version_number}</span>
                </Show>
              </p>
              <p class={styles.info}>
                <Switch>
                  <Match when={"version" in props.mod}>
                    <span class={styles.lowHierarchy}>
                      <Fa icon={faHardDrive} /> {humanizeFileSize((props.mod as ModPackage).version.file_size)}
                    </span>
                  </Match>
                  <Match when={"versions" in props.mod}>
                    <span class={styles.lowHierarchy}>
                      <Fa icon={faDownload} />{" "}
                      {roundedNumberFormatter.format(
                        (props.mod as ModListing).versions.map((v) => v.downloads).reduce((acc, x) => acc + x),
                      )}
                    </span>
                  </Match>
                </Switch>
              </p>
              <p class={styles.description}>{displayVersion().description}</p>
            </div>
            <Show when={installContext !== undefined}>
              <Switch
                fallback={
                  <ErrorBoundary>
                    <Tooltip content={t("modlist.online.install_btn")}>
                      <InstallButton
                        mod={props.mod as ModListing}
                        installContext={installContext!}
                        class={styles.downloadBtn}
                        busyClass={styles.downloadBtnBusy}
                        progressStyle="circular"
                      >
                        <Fa icon={faDownLong} />
                      </InstallButton>
                    </Tooltip>
                  </ErrorBoundary>
                }
              >
                <Match when={installed()}>
                  {(installed) => (
                    <ErrorBoundary>
                      <Tooltip content={t("modlist.installed.uninstall_btn")}>
                        <UninstallButton
                          mod={installed()}
                          installContext={installContext!}
                          class={styles.downloadBtn}
                          busyClass={styles.downloadBtnBusy}
                          progressStyle="circular"
                        >
                          <Fa icon={faTrash} />
                        </UninstallButton>
                      </Tooltip>
                    </ErrorBoundary>
                  )}
                </Match>
              </Switch>
            </Show>
          </div>
        </div>
      </div>
    </li>
  );
}

function InstallButton(props: {
  mod: Mod;
  installContext: NonNullable<typeof ModInstallContext.defaultValue>;
  class: JSX.HTMLAttributes<Element>["class"];
  busyClass?: JSX.HTMLAttributes<Element>["class"];
  children: JSX.Element;
  progressStyle?: ProgressStyle;
}) {
  return (
    <SimpleAsyncButton
      progressStyle={props.progressStyle}
      progress
      class={props.class}
      busyClass={props.busyClass}
      dataset={{ "data-install": "" }}
      onClick={async (listener) => {
        let foundDownloadTask = false;
        await installProfileMod(
          props.installContext.profileId(),
          "versions" in props.mod ? removeProperty(props.mod, "versions") : removeProperty(props.mod, "version"),
          "versions" in props.mod ? props.mod.versions[0] : props.mod.version,
          (event) => {
            if (!foundDownloadTask && event.event === "dependency") {
              const dependency = tasks().get(event.dependency)!;
              if (dependency.metadata.kind === "Download") {
                foundDownloadTask = true;
                registerTaskListener(event.dependency, listener);
              } else if (dependency.status.status === "Unstarted") {
                // wait for metadata to be filled in and check again
                registerTaskListener(event.dependency, (depEvent) => {
                  if (!foundDownloadTask && depEvent.event === "created" && depEvent.metadata.kind === "Download") {
                    foundDownloadTask = true;
                    registerTaskListener(event.dependency, listener);
                  }
                });
              }
            }
          },
        );
        await props.installContext.refetchInstalled();
      }}
    >
      {props.children}
    </SimpleAsyncButton>
  );
}

function UninstallButton(props: {
  mod: ModPackage;
  installContext: NonNullable<typeof ModInstallContext.defaultValue>;
  class: JSX.HTMLAttributes<Element>["class"];
  busyClass?: JSX.HTMLAttributes<Element>["class"];
  children: JSX.Element;
  progressStyle?: ProgressStyle;
}) {
  return (
    <SimpleAsyncButton
      progressStyle={props.progressStyle}
      progress
      class={props.class}
      dataset={{ "data-uninstall": "" }}
      busyClass={props.busyClass}
      onClick={async (_listener) => {
        await uninstallProfileMod(props.installContext.profileId(), props.mod.owner, props.mod.name);
        await props.installContext.refetchInstalled();
      }}
    >
      {props.children}
    </SimpleAsyncButton>
  );
}
