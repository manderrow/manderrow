import { faHardDrive, faHeart, faThumbsUp } from "@fortawesome/free-regular-svg-icons";
import { faDownLong, faDownload, faExternalLink, faRefresh, faTrash } from "@fortawesome/free-solid-svg-icons";
import { createInfiniteScroll } from "@solid-primitives/pagination";
import { useParams } from "@solidjs/router";
import { Fa } from "solid-fa";
import {
  Accessor,
  For,
  InitializedResource,
  Match,
  ResourceFetcherInfo,
  Show,
  Signal,
  Switch,
  createContext,
  createMemo,
  createResource,
  createSelector,
  createSignal,
  useContext,
} from "solid-js";

import {
  ModSortColumn,
  SortOption,
  countModIndex,
  fetchModIndex,
  getFromModIndex,
  installProfileMod,
  queryModIndex,
  uninstallProfileMod,
} from "../../api";
import { Progress, createProgressProxyStore, initProgress, registerTaskListener, tasks } from "../../api/tasks";
import { Mod, ModListing, ModPackage, ModVersion } from "../../types";
import { humanizeFileSize, numberFormatter, removeProperty, roundedNumberFormatter } from "../../utils";

import { SimpleAsyncButton } from "../global/AsyncButton";
import ErrorBoundary, { ErrorContext } from "../global/ErrorBoundary";
import TabRenderer, { Tab, TabContent } from "../global/TabRenderer";
import ModMarkdown from "./ModMarkdown.tsx";
import ModSearch from "./ModSearch.tsx";

import styles from "./ModList.module.css";

type PageFetcher = (page: number) => Promise<readonly Mod[]>;
export type Fetcher = (
  game: string,
  query: string,
  sort: readonly SortOption<ModSortColumn>[],
) => Promise<{
  count: number;
  mods: PageFetcher;
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
  refresh: () => void;
  isLoading: boolean;
  progress: Progress;
}) {
  const [selectedMod, setSelectedMod] = createSignal<Mod>();

  // TODO: Type this properly with ProfileParams
  const params = useParams();

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
    () => [props.game, query(), sort(), props.mods] as [string, string, readonly SortOption<ModSortColumn>[], Fetcher],
    async ([game, query, sort, mods]) => mods(game, query, sort),
    { initialValue: { mods: async (_: number) => [], count: 0 } },
  );

  return (
    <div class={styles.modListAndView}>
      <div class={styles.scrollOuter}>
        <ModSearch
          game={params.gameId}
          query={query()}
          setQuery={setQuery}
          sort={sort()}
          setSort={setSort}
          profileSortOrder={profileSortOrder()}
          setProfileSortOrder={setProfileSortOrder}
          isLoading={props.isLoading}
          progress={props.progress}
        />

        <Show when={queriedMods.latest} fallback={<p>Querying mods...</p>}>
          <div class={styles.discoveredLine}>
            Discovered {numberFormatter.format(queriedMods()!.count)} mods
            <button class={styles.refreshButton} on:click={() => props.refresh()}>
              <Fa icon={faRefresh} />
            </button>
          </div>
        </Show>

        <Show when={queriedMods()} keyed>
          {(mods) => <ModListMods mods={mods.mods} selectedMod={[selectedMod, setSelectedMod]} />}
        </Show>
      </div>
      <ModView mod={selectedMod} gameId={params.gameId} />
    </div>
  );
}

export function OnlineModList(props: { game: string }) {
  const [progress, setProgress] = createProgressProxyStore();
  const reportErr = useContext(ErrorContext)!;

  const [loadStatus, { refetch: refetchModIndex }] = createResource(
    () => props.game,
    async (game, info: ResourceFetcherInfo<boolean, never>) => {
      try {
        await fetchModIndex(game, { refresh: info.refetching }, (event) => {
          if (event.event === "created") {
            setProgress(event.progress);
          }
        });
      } catch (e) {
        reportErr(e);
        throw e;
      }
      return true;
    },
  );

  return (
    <ModList
      game={props.game}
      isLoading={loadStatus.loading}
      progress={progress}
      refresh={refetchModIndex}
      mods={async (game, query, sort) => {
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
        };
      }}
    />
  );
}

export function InstalledModList(props: { game: string }) {
  const context = useContext(ModInstallContext)!;

  const [progress, setProgress] = createProgressProxyStore();

  return (
    <Show when={context.installed.latest.length !== 0} fallback={<p>Looks like you haven't installed any mods yet.</p>}>
      <ModList
        game={props.game}
        isLoading={false}
        progress={progress}
        // TODO: implement local refreshing
        refresh={() => {}} // looks like eyes
        // kinda gross
        mods={async (game, query, sort) => {
          // TODO: implement filter and sort
          const data = context.installed();

          return {
            count: data.length,
            mods: async (page) => (page === 0 ? data : []),
          };
        }}
      />
    </Show>
  );
}

function ModView({ mod, gameId }: { mod: Accessor<Mod | undefined>; gameId: string }) {
  const [progress, setProgress] = createProgressProxyStore();

  function getInitialModListing(mod: Mod | undefined) {
    if (mod === undefined) return undefined;
    if ("version" in mod) {
      const obj: ModListing & { game?: string; version?: ModVersion } = { ...mod, versions: [mod.version] };
      delete obj.version;
      delete obj.game;
      return obj;
    } else {
      return mod;
    }
  }

  const [modListing, { refetch: refetchModListing }] = createResource<
    ModListing | undefined,
    Mod | Record<never, never>,
    never
  >(
    // we need the "nullish" value passed through, so disguise it as non-nullish
    () => mod() ?? {},
    async (mod, info: ResourceFetcherInfo<ModListing | undefined, never>) => {
      if ("game" in mod) {
        await fetchModIndex(mod.game, { refresh: info.refetching }, (event) => {
          if (event.event === "created") {
            setProgress(event.progress);
          }
        });
        return (await getFromModIndex(mod.game, [{ owner: mod.owner, name: mod.name }]))[0];
      } else if ("versions" in mod) {
        setProgress(initProgress());
        return mod;
      } else {
        return undefined;
      }
    },
    { initialValue: getInitialModListing(mod()) },
  );

  const [selectedVersion, setSelectedVersion] = createSignal<[string, number]>();

  const modVersionData = (mod: Mod) => {
    return "versions" in mod ? mod.versions[selectedVersion()?.[1] ?? 0] : mod.version;
  };

  const tabs: Tab<"overview" | "dependencies" | "changelog">[] = [
    {
      id: "overview",
      name: "Overview",
      component: () => <ModMarkdown mod={mod()} selectedVersion={selectedVersion()?.[0]} endpoint="readme" />,
    },
    {
      id: "dependencies",
      name: "Dependencies",
      component: () => (
        <Show when={mod()}>
          {(mod) => <For each={modVersionData(mod()).dependencies}>{(dependency) => <p>{dependency}</p>}</For>}
        </Show>
      ),
    },
    {
      id: "changelog",
      name: "Changelog",
      component: () => <ModMarkdown mod={mod()} selectedVersion={selectedVersion()?.[0]} endpoint="changelog" />,
    },
  ];

  const [currentTab, setCurrentTab] = createSignal(tabs[0].id);
  const isCurrentTab = createSelector(currentTab);

  return (
    <div class={styles.scrollOuter}>
      <div class={`${styles.scrollInner} ${styles.modView}`}>
        <Show
          when={mod()}
          fallback={
            <div class={styles.nothingMsg}>
              <h2>No mod selected</h2>
              <p>Select a mod to it view here.</p>
            </div>
          }
        >
          {(mod) => {
            const modVersionData = () => {
              const modConstant = mod();

              return "versions" in modConstant
                ? modConstant.versions[selectedVersion()?.[1] ?? 0]
                : modConstant.version;
            };

            return (
              <>
                <div class={styles.modSticky}>
                  <div class={styles.modMeta}>
                    {/* TODO: For local mod with no package URL, remove link */}
                    <div style={{ "grid-area": "name" }}>
                      <a
                        href={`https://thunderstore.io/c/${gameId}/p/${mod().owner}/${mod().name}/`}
                        target="_blank"
                        rel="noopener noreferrer"
                        class={styles.modMetaLink}
                      >
                        <h2 class={styles.name}>{mod().name}</h2>
                        <Fa icon={faExternalLink} />
                      </a>
                    </div>
                    <div style={{ "grid-area": "owner" }}>
                      <a
                        href={`https://thunderstore.io/c/${gameId}/p/${mod().owner}/`}
                        target="_blank"
                        rel="noopener noreferrer"
                        class={styles.modMetaLink}
                      >
                        {mod().owner}
                        <Fa icon={faExternalLink} />
                      </a>
                    </div>
                    <ul class={styles.modMetadata}>
                      <li class={styles.metadata__field}>
                        <Fa icon={faThumbsUp} /> {roundedNumberFormatter.format(mod().rating_score)}
                      </li>
                      <li class={styles.metadata__field}>
                        <Fa icon={faDownload} /> {roundedNumberFormatter.format(modVersionData().downloads)}
                      </li>
                      <li class={styles.metadata__field}>
                        <Fa icon={faHardDrive} /> {humanizeFileSize(modVersionData().file_size)}
                      </li>
                    </ul>

                    <Show when={mod().donation_link != null}>
                      <a
                        class={styles.modMeta__donate}
                        href={mod().donation_link}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        <Fa icon={faHeart} class={styles.donate__icon} />
                        <br /> Donate
                      </a>
                    </Show>
                  </div>

                  <TabRenderer
                    id="mod-view"
                    tabs={tabs}
                    styles={{
                      tabs: {
                        container: styles.tabs,
                        list: styles.tabs__list,
                        list__item: styles.tabs__tab,
                        list__itemActive: styles.tab__active,
                      },
                    }}
                    setter={(tab) => setCurrentTab(tab.id)}
                  />
                </div>

                <div class={styles.modView__content}>
                  <TabContent isCurrentTab={isCurrentTab} tabs={tabs} />
                </div>

                <form class={styles.modView__form} action="#">
                  <div class={styles.modView__downloader}>
                    <select
                      class={styles.modView__versions}
                      onInput={(event) =>
                        setSelectedVersion([
                          event.target.value,
                          parseInt(event.target.selectedOptions[0].dataset.index!, 10),
                        ])
                      }
                    >
                      {/* This entire thing is temporary anyway, it will be removed in a later commit */}
                      <For each={modListing.latest?.versions}>
                        {(version, i) => {
                          return (
                            <option value={version.version_number} data-index={i()}>
                              v{version.version_number} {i() === 0 ? "(latest)" : ""}
                            </option>
                          );
                        }}
                      </For>
                    </select>
                    <button type="button" class={styles.modView__downloadBtn}>
                      Download
                    </button>
                  </div>
                </form>
              </>
            );
          }}
        </Show>
      </div>
    </div>
  );
}

function ModListMods(props: { mods: PageFetcher; selectedMod: Signal<Mod | undefined> }) {
  const infiniteScroll = createMemo(() => {
    // this should take readonly, which would make the cast unnecessary
    return createInfiniteScroll(props.mods as (page: number) => Promise<Mod[]>);
  });
  const paginatedMods = () => infiniteScroll()[0]();
  // idk why we're passing props here
  const infiniteScrollLoader = (el: Element) => infiniteScroll()[1](el);
  const end = () => infiniteScroll()[2].end();

  return (
    <ol class={`${styles.modList} ${styles.scrollInner}`}>
      <For each={paginatedMods()}>{(mod) => <ModListItem mod={mod} selectedMod={props.selectedMod} />}</For>
      <Show when={!end()}>
        <li use:infiniteScrollLoader>Loading...</li>
      </Show>
    </ol>
  );
}

function getIconUrl(owner: string, name: string, version: string) {
  return `https://gcdn.thunderstore.io/live/repository/icons/${owner}-${name}-${version}.png`;
}

function ModListItem(props: { mod: Mod; selectedMod: Signal<Mod | undefined> }) {
  const displayVersion = createMemo(() => {
    if ("version" in props.mod) return props.mod.version;
    return props.mod.versions[0];
  });

  const installContext = useContext(ModInstallContext);

  const installed = createMemo(() => {
    const mod = props.mod;
    if ("version" in mod) {
      return mod;
    } else {
      return installContext?.installed.latest.find((pkg) => pkg.owner === mod.owner && pkg.name === mod.name);
    }
  });

  function onSelect() {
    props.selectedMod[1](props.selectedMod[0]() === props.mod ? undefined : props.mod);
  }

  return (
    <li classList={{ [styles.mod]: true, [styles.selected]: props.selectedMod[0]() === props.mod }}>
      <div
        on:click={onSelect}
        onKeyDown={(key) => {
          if (key.key === "Enter") onSelect();
        }}
        class={styles.mod__btn}
        role="button"
        aria-pressed={props.selectedMod[0]() === props.mod}
        tabIndex={0}
      >
        <img
          class={styles.icon}
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
              <span class={styles.owner}>{props.mod.owner}</span>
              <Show when={"version" in props.mod}>
                <span class={styles.separator} aria-hidden>
                  &bull;
                </span>
                <span class={styles.version}>{(props.mod as ModPackage).version.version_number}</span>
              </Show>
            </p>
            <p class={styles.downloads}>
              <Show when={"versions" in props.mod}>
                <Fa icon={faDownload} />
                {roundedNumberFormatter.format(
                  (props.mod as ModListing).versions.map((v) => v.downloads).reduce((acc, x) => acc + x),
                )}
              </Show>
            </p>
            <p class={styles.description}>{displayVersion().description}</p>
          </div>
          <div class={styles.right}>
            <Show when={installContext !== undefined}>
              <Switch
                fallback={
                  <ErrorBoundary>
                    <InstallButton mod={props.mod as ModListing} installContext={installContext!} />
                  </ErrorBoundary>
                }
              >
                <Match when={installed()}>
                  {(installed) => (
                    <ErrorBoundary>
                      <UninstallButton mod={installed()} installContext={installContext!} />
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

function InstallButton(props: { mod: ModListing; installContext: NonNullable<typeof ModInstallContext.defaultValue> }) {
  return (
    <SimpleAsyncButton
      progress
      class={styles.downloadBtn}
      onClick={async (listener) => {
        let foundDownloadTask = false;
        await installProfileMod(
          props.installContext.profileId(),
          removeProperty(props.mod, "versions"),
          props.mod.versions[0],
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
      <Fa icon={faDownLong} />
    </SimpleAsyncButton>
  );
}

function UninstallButton(props: {
  mod: ModPackage;
  installContext: NonNullable<typeof ModInstallContext.defaultValue>;
}) {
  return (
    <SimpleAsyncButton
      progress
      class={styles.downloadBtn}
      onClick={async (_listener) => {
        await uninstallProfileMod(props.installContext.profileId(), props.mod.owner, props.mod.name);
        await props.installContext.refetchInstalled();
      }}
    >
      <Fa icon={faTrash} />
    </SimpleAsyncButton>
  );
}
