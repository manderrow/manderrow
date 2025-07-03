import { faHardDrive, faHeart, faThumbsUp } from "@fortawesome/free-regular-svg-icons";
import { faDownload, faDownLong, faExternalLink, faTrash } from "@fortawesome/free-solid-svg-icons";
import { createSkeletonScrollCell, createSkeletonScrollObserver } from "skeleton-scroll";
import { Fa } from "solid-fa";
import {
  Accessor,
  createContext,
  createEffect,
  createMemo,
  createResource,
  createSelector,
  createSignal,
  For,
  InitializedResource,
  Match,
  ResourceFetcherInfo,
  Show,
  Switch,
  useContext,
} from "solid-js";

import { fetchModIndex, getFromModIndex, installProfileMod, uninstallProfileMod } from "../../api";
import { createProgressProxyStore, initProgress } from "../../api/tasks";
import { Mod, ModListing, ModPackage, ModVersion } from "../../types";
import { humanizeFileSize, removeProperty, roundedNumberFormatter } from "../../utils";

import { SimpleAsyncButton } from "../global/AsyncButton";
import ErrorBoundary from "../global/ErrorBoundary";
import TabRenderer, { Tab, TabContent } from "../global/TabRenderer";
import ModMarkdown from "./ModMarkdown.tsx";

import styles from "./ModList.module.css";
// @ts-types="solid-js"

export type Fetcher = (index: number) => Promise<Mod> | Mod;

export const ModInstallContext = createContext<{
  profileId: Accessor<string>;
  installed: InitializedResource<readonly ModPackage[]>;
  refetchInstalled: () => Promise<void>;
}>();

export default function ModList(props: { count: number; mods: Fetcher }) {
  const [selectedMod, setSelectedMod] = createSignal<Mod>();
  const isSelectedMod = createSelector<Mod | undefined, Mod>(
    selectedMod,
    (mod, selectedMod) => selectedMod !== undefined && mod.owner === selectedMod.owner && mod.name === selectedMod.name,
  );

  return (
    <div class={styles.modListAndView}>
      <ModListMods
        count={props.count}
        mods={props.mods}
        isSelectedMod={isSelectedMod}
        setSelectedMod={setSelectedMod}
      />
      <ModView mod={selectedMod} />
    </div>
  );
}

function ModView({ mod }: { mod: Accessor<Mod | undefined> }) {
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
      component: <ModMarkdown mod={mod()} selectedVersion={selectedVersion()?.[0]} endpoint="readme" />,
    },
    {
      id: "dependencies",
      name: "Dependencies",
      component: (
        <Show when={mod()}>
          {(mod) => <For each={modVersionData(mod()).dependencies}>{(dependency) => <p>{dependency}</p>}</For>}
        </Show>
      ),
    },
    {
      id: "changelog",
      name: "Changelog",
      component: <ModMarkdown mod={mod()} selectedVersion={selectedVersion()?.[0]} endpoint="changelog" />,
    },
  ];

  const [currentTab, setCurrentTab] = createSignal(tabs[0]);

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
                        href={`https://thunderstore.io/package/${mod().owner}/${mod().name}/`}
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
                        href={`https://thunderstore.io/package/${mod().owner}/`}
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
                    setter={setCurrentTab}
                  />
                </div>

                <div class={styles.modView__content}>
                  <TabContent currentTab={currentTab} tabs={tabs} />
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

function generateArray<T>(length: number, element: (index: number) => T): T[] {
  const array = new Array(length);
  for (let i = 0; i < length; i++) {
    array[i] = element(i);
  }
  return array;
}

function ModListMods(props: {
  count: number;
  mods: Fetcher;
  isSelectedMod: (mod: Mod) => boolean;
  setSelectedMod: (mod: Mod | null) => void;
}) {
  // TODO: fetch mod ids eagerly so we can use them as keys in the For
  return (
    <div class={styles.scrollOuter}>
      <ol class={`${styles.modList} ${styles.scrollInner}`}>
        <For each={generateArray(props.count, (i) => i)}>
          {(i) => (
            <ModListItem
              mods={props.mods}
              modIndex={i}
              isSelectedMod={props.isSelectedMod}
              setSelectedMod={props.setSelectedMod}
            />
          )}
        </For>
      </ol>
    </div>
  );
}

function getIconUrl(owner: string, name: string, version: string) {
  return `https://gcdn.thunderstore.io/live/repository/icons/${owner}-${name}-${version}.png`;
}

const observer = createSkeletonScrollObserver({ rootMargin: "1600px 0px" });

function ModListItem(props: {
  mods: Fetcher;
  modIndex: number;
  isSelectedMod: (mod: Mod) => boolean;
  setSelectedMod: (mod: Mod | null) => void;
}) {
  const [visible, setVisible] = createSignal(false);

  let ref!: HTMLLIElement;

  createSkeletonScrollCell(observer, () => ref, setVisible);

  const [mod, setMod] = createSignal<Mod>();
  let fetching = false;

  createEffect(() => {
    // once `fetching` is set, this effect will stop tracking `visible` and will start tracking `props.mods` instead.
    if (!fetching && visible()) {
      fetching = true;
      (async () => setMod(await props.mods(props.modIndex)))();
    }
  });

  const isSelectedMod = (mod: Mod | undefined) => {
    return mod != null && props.isSelectedMod(mod);
  };

  return (
    <li classList={{ [styles.mod]: true, [styles.selected]: isSelectedMod(mod()) }} ref={ref}>
      <Show when={visible()}>
        <Show when={mod()} fallback="...">
          {(mod) => (
            <ModListItemContent
              mod={mod()}
              isSelectedMod={props.isSelectedMod}
              setSelectedMod={props.setSelectedMod}
            ></ModListItemContent>
          )}
        </Show>
      </Show>
    </li>
  );
}

function ModListItemContent(props: {
  mod: Mod;
  isSelectedMod: (mod: Mod) => boolean;
  setSelectedMod: (mod: Mod | null) => void;
}) {
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
    const modConstant = props.mod;
    if (modConstant == null) return;
    props.setSelectedMod(props.isSelectedMod(modConstant) ? null : modConstant);
  }

  return (
    <div
      on:click={onSelect}
      onKeyDown={(key) => {
        if (key.key === "Enter") onSelect();
      }}
      class={styles.mod__btn}
      role="button"
      aria-pressed={props.isSelectedMod(props.mod)}
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
          <Show when={installContext}>
            {(installContext) => (
              <Switch
                fallback={
                  <ErrorBoundary>
                    <InstallButton mod={props.mod as ModListing} installContext={installContext()} />
                  </ErrorBoundary>
                }
              >
                <Match when={installed()}>
                  {(installed) => (
                    <ErrorBoundary>
                      <UninstallButton mod={installed()} installContext={installContext()} />
                    </ErrorBoundary>
                  )}
                </Match>
              </Switch>
            )}
          </Show>
        </div>
      </div>
    </div>
  );
}

function InstallButton(props: { mod: ModListing; installContext: NonNullable<typeof ModInstallContext.defaultValue> }) {
  return (
    <SimpleAsyncButton
      progress
      class={styles.downloadBtn}
      onClick={async (listener) => {
        await installProfileMod(
          props.installContext.profileId(),
          removeProperty(props.mod, "versions"),
          props.mod.versions[0],
          listener,
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
