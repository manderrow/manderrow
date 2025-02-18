import { createInfiniteScroll } from "@solid-primitives/pagination";
import {
  Accessor,
  createContext,
  createMemo,
  createResource,
  createSignal,
  For,
  InitializedResource,
  Match,
  ResourceFetcherInfo,
  Show,
  Signal,
  Switch,
  useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import Fa from "solid-fa";
import { faDownload, faDownLong, faExternalLink, faTrash } from "@fortawesome/free-solid-svg-icons";
import { faHeart } from "@fortawesome/free-regular-svg-icons";
import { fetch } from "@tauri-apps/plugin-http";

import { Mod, ModListing, ModPackage } from "../../types";
import { numberFormatter, roundedNumberFormatter } from "../../utils";
import ErrorBoundary, { ErrorContext } from "../global/ErrorBoundary";
import { InitialProgress, ProgressData } from "./ModSearch";
import { fetchModIndex, installProfileMod, queryModIndex, uninstallProfileMod } from "../../api";

import styles from "./ModList.module.css";
import Markdown from "../global/Markdown";

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
  hour: "numeric",
  minute: "numeric",
});

export type Fetcher = (page: number) => Promise<readonly Mod[]>;

export const ModInstallContext = createContext<{
  profile: string;
  installed: InitializedResource<readonly ModPackage[]>;
  refetchInstalled: () => Promise<void>;
}>();

export default function ModList(props: { mods: Fetcher }) {
  const [selectedMod, setSelectedMod] = createSignal<Mod>();

  return (
    <div class={styles.modListAndView}>
      <Show when={props.mods} keyed>
        {(mods) => <ModListMods mods={mods} selectedMod={[selectedMod, setSelectedMod]} />}
      </Show>
      <ModView mod={selectedMod} />
    </div>
  );
}

function ModView({ mod }: { mod: Accessor<Mod | undefined> }) {
  const [progress, setProgress] = createStore<InitialProgress | ProgressData>({
    completed: null,
    total: null,
  });

  function getInitialValue(mod: Mod | undefined) {
    if (mod === undefined) return undefined;
    if ("version" in mod) {
      const obj = { ...mod, versions: [mod.version] };
      // @ts-expect-error
      delete obj.version;
      // @ts-expect-error
      delete obj.game;
      return obj;
    } else {
      return mod;
    }
  }

  const [modListing, { refetch: refetchModListing }] = createResource<ModListing | undefined, Mod | {}, never>(
    // we need the "nullish" value passed through, so disguise it as non-nullish
    () => (mod() === undefined ? {} : mod()!),
    async (mod, info: ResourceFetcherInfo<ModListing | undefined, never>) => {
      if ("game" in mod) {
        await fetchModIndex(mod.game, { refresh: info.refetching }, setProgress);
        return (await queryModIndex(mod.game, "", [], { exact: [{owner: mod.owner, name: mod.name}] })).mods[0];
      } else if ("versions" in mod) {
        setProgress({ completed: null, total: null });
        return mod;
      } else {
        return undefined;
      }
    },
    { initialValue: getInitialValue(mod()) },
  );

  const [selectedVersion, setSelectedVersion] = createSignal<string>();

  const [modReadme] = createResource(
    () => ({ mod: mod(), selectedVersion: selectedVersion() }),
    async ({ mod, selectedVersion }) => {
      if (mod == null) return undefined;

      // mod is a ModPackage if it has the version field, otherwise it is a ModListing
      const version =
        "version" in mod ? mod.version.version_number : (selectedVersion ?? mod.versions[0].version_number);

      try {
        const request = await fetch(
          `https://thunderstore.io/api/experimental/package/${mod.owner}/${mod.name}/${version}/readme/`,
        );
        return (await request.json()) as { markdown: string };
      } catch (error) {
        // TODO: error handling
        console.error(error);
        return undefined;
      }
    },
  );

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
          {(mod) => (
            <>
              <div>
                <h2 class={styles.name}>
                  {mod().name}
                  <a href={`https://thunderstore.io/package/${mod().owner}/${mod().name}/`} target="_blank" rel="noopener noreferrer">
                    <Fa icon={faExternalLink} /> Website
                  </a>
                </h2>
                <p class={styles.description}>
                  {mod().owner}
                  <Show when={mod().donation_link != null}>
                    <a href={mod().donation_link} target="_blank" rel="noopener noreferrer">
                      <Fa icon={faHeart} /> Donate
                    </a>
                  </Show>
                </p>
                <Show when={modListing.latest}>
                  {(modListing) => <p class={styles.description}>{modListing().versions[0].description}</p>}
                </Show>
              </div>

              <Show when={modReadme()} fallback={<p>Fallback</p>}>
                {(modReadme) => <Markdown source={modReadme().markdown} div={{ class: styles.modView__content }} />}
              </Show>

              <form class={styles.modView__downloader} action="#">
                <select class={styles.versions} onInput={(event) => setSelectedVersion(event.target.value)}>
                  {/* This entire thing is temporary anyway, it will be removed in a later commit */}
                  <For each={modListing.latest?.versions}>
                    {(version, i) => {
                      return (
                        <option value={version.uuid4}>
                          v{version.version_number} {i() === 0 ? "(latest)" : ""}
                        </option>
                      );
                    }}
                  </For>
                </select>
                <button>Download</button>
              </form>
            </>
          )}
        </Show>
      </div>
    </div>
  );
}

function ModListMods(props: { mods: Fetcher; selectedMod: Signal<Mod | undefined> }) {
  const infiniteScroll = createMemo(() => {
    // this should take readonly, which would make the cast unnecessary
    return createInfiniteScroll(props.mods as (page: number) => Promise<Mod[]>);
  });
  const paginatedMods = () => infiniteScroll()[0]();
  // idk why we're passing props here
  const infiniteScrollLoader = (el: Element) => infiniteScroll()[1](el);
  const end = () => infiniteScroll()[2].end();

  return (
    <div class={styles.scrollOuter}>
      <ol class={`${styles.modList} ${styles.scrollInner}`}>
        <For each={paginatedMods()}>{(mod) => <ModListItem mod={mod} selectedMod={props.selectedMod} />}</For>
        <Show when={!end()}>
          <li use:infiniteScrollLoader>Loading...</li>
        </Show>
      </ol>
    </div>
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
      return installContext?.installed.latest.find((pkg) => pkg.uuid4 === mod.uuid4);
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
        <img class={styles.icon} src={getIconUrl(props.mod.owner, props.mod.name, displayVersion().version_number)} />
        <div class={styles.mod__content}>
          <div class={styles.left}>
            <p class={styles.info}>
              <span class={styles.name}>{props.mod.name}</span>
              <span class={styles.separator} aria-hidden>
                &bull;
              </span>
              <span class={styles.owner}>{props.mod.owner}</span>
            </p>
            <p class={styles.downloads}>
              <Show when={"versions" in props.mod}>
                <Fa icon={faDownload} />{" "}
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
  const reportErr = useContext(ErrorContext);
  const [busy, setBusy] = createSignal(false);
  return (
    <button
      class={styles.downloadBtn}
      disabled={busy()}
      on:click={async (e) => {
        e.stopPropagation();
        setBusy(true);
        try {
          await installProfileMod(props.installContext.profile, props.mod, 0);
          await props.installContext.refetchInstalled();
        } catch (e) {
          reportErr(e);
        } finally {
          setBusy(false);
        }
      }}
    >
      <Show when={busy()} fallback={<Fa icon={faDownLong} />}>
        <progress />
      </Show>
    </button>
  );
}

function UninstallButton(props: {
  mod: ModPackage;
  installContext: NonNullable<typeof ModInstallContext.defaultValue>;
}) {
  const reportErr = useContext(ErrorContext);
  const [busy, setBusy] = createSignal(false);
  return (
    <button
      class={styles.downloadBtn}
      disabled={busy()}
      on:click={async (e) => {
        e.stopPropagation();
        setBusy(true);
        try {
          await uninstallProfileMod(props.installContext.profile, props.mod.owner, props.mod.name);
          await props.installContext.refetchInstalled();
        } catch (e) {
          reportErr(e);
        } finally {
          setBusy(false);
        }
      }}
    >
      <Show when={busy()} fallback={<Fa icon={faTrash} />}>
        <progress />
      </Show>
    </button>
  );
}
