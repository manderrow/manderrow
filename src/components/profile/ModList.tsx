import { createInfiniteScroll } from "@solid-primitives/pagination";
import {
  createContext,
  createMemo,
  createResource,
  createSignal,
  For,
  ResourceFetcherInfo,
  Show,
  Signal,
  useContext,
} from "solid-js";

import { Mod, ModListing } from "../../types";
import { numberFormatter } from "../../utils";

import styles from "./ModList.module.css";
import ErrorBoundary, { ErrorContext } from "../global/ErrorBoundary";
import { InitialProgress, ProgressData } from "./ModSearch";
import { createStore } from "solid-js/store";
import { fetchModIndex, installProfileMod, queryModIndex } from "../../api";

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
}>();

export default function ModList(props: { mods: Fetcher }) {
  const [selectedMod, setSelectedMod] = createSignal<Mod>();

  return (
    <div class={styles.modListAndView}>
      <Show when={props.mods} keyed>
        {(mods) => (
          <ModListLeft
            mods={mods}
            selectedMod={[selectedMod, setSelectedMod]}
          />
        )}
      </Show>
      <Show when={selectedMod()}>{(mod) => <SelectedMod mod={mod()} />}</Show>
    </div>
  );
}

function ModListLeft({
  mods,
  selectedMod,
}: {
  mods: Fetcher;
  selectedMod: Signal<Mod | undefined>;
}) {
  const [paginatedMods, infiniteScrollLoader, { end }] =
    // cast away the readonly
    // TODO: when we fork this, make it take readonly instead
    createInfiniteScroll(mods as (page: number) => Promise<Mod[]>);

  return (
    <div class={styles.scrollOuter}>
      <ol class={`${styles.modList} ${styles.scrollInner}`}>
        <For each={paginatedMods()}>
          {(mod) => <ModListItem mod={mod} selectedMod={selectedMod} />}
        </For>
        <Show when={!end()}>
          <li use:infiniteScrollLoader>Loading...</li>
        </Show>
      </ol>
    </div>
  );
}

function ModListItem(props: {
  mod: Mod;
  selectedMod: Signal<Mod | undefined>;
}) {
  const displayVersion = createMemo(() => {
    if ("version" in props.mod) return props.mod.version;
    return props.mod.versions[0];
  });

  const installContext = useContext(ModInstallContext);

  return (
    <li
      classList={{ [styles.selected]: props.selectedMod[0]() === props.mod }}
      on:click={() =>
        props.selectedMod[1](
          props.selectedMod[0]() === props.mod ? undefined : props.mod
        )
      }
    >
      <img class={styles.icon} src={displayVersion().icon} />
      <div class={styles.split}>
        <div class={styles.left}>
          <div>
            <span class={styles.name}>{props.mod.name}</span>{" "}
            <span class={styles.version}>
              v{displayVersion().version_number}
            </span>
          </div>
          <div class={styles.owner}>
            <span class={styles.label}>@</span>
            <span class={styles.value}>{props.mod.owner}</span>
          </div>
          <ul class={styles.categories}>
            <For each={props.mod.categories}>
              {(category) => <li>{category}</li>}
            </For>
          </ul>
        </div>
        <div class={styles.right}>
          <Show when={"versions" in props.mod}>
            <p class={styles.downloads}>
              <span class={styles.label}>Downloads: </span>
              <span class={styles.value}>
                {numberFormatter.format(
                  (props.mod as ModListing).versions
                    .map((v) => v.downloads)
                    .reduce((acc, x) => acc + x)
                )}
              </span>
            </p>
          </Show>
          <Show when={installContext !== undefined && "versions" in props.mod}>
            <ErrorBoundary>
              <InstallButton
                mod={props.mod as ModListing}
                installContext={installContext!}
              />
            </ErrorBoundary>
          </Show>
        </div>
      </div>
    </li>
  );
}

function InstallButton(props: {
  mod: ModListing;
  installContext: NonNullable<typeof ModInstallContext.defaultValue>;
}) {
  const reportErr = useContext(ErrorContext);
  const [installing, setInstalling] = createSignal(false);
  return (
    <button
      disabled={installing()}
      on:click={async (e) => {
        e.stopPropagation();
        setInstalling(true);
        try {
          await installProfileMod(props.installContext.profile, props.mod, 0);
        } catch (e) {
          reportErr(e);
        } finally {
          setInstalling(false);
        }
      }}
    >
      <Show when={installing()} fallback="Install">
        <progress />
      </Show>
    </button>
  );
}

function SelectedMod(props: { mod: Mod }) {
  const [progress, setProgress] = createStore<InitialProgress | ProgressData>({
    completed: null,
    total: null,
  });

  function getInitialValue(mod: Mod) {
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

  const [modListing, { refetch: refetchModListing }] = createResource(
    () => props.mod,
    async (mod, info: ResourceFetcherInfo<ModListing, never>) => {
      if ("game" in mod) {
        await fetchModIndex(
          mod.game,
          { refresh: info.refetching },
          setProgress
        );
        return (
          await queryModIndex(mod.game, "", [], { exact: [mod.full_name] })
        ).mods[0];
      } else {
        setProgress({ completed: null, total: null });
        return mod;
      }
    },
    { initialValue: getInitialValue(props.mod) }
  );

  return (
    <div class={styles.scrollOuter}>
      <div class={`${styles.modView} ${styles.scrollInner}`}>
        <h2 class={styles.name}>{props.mod.name}</h2>
        <p class={styles.description}>
          {modListing.latest.versions[0].description}
        </p>

        <h3>Versions</h3>
        <ol class={styles.versions}>
          <For each={modListing.latest.versions}>
            {(version) => {
              return (
                <li>
                  <div>
                    <span class={styles.version}>{version.version_number}</span>
                    <span> - </span>
                    <span class={styles.timestamp} title={version.date_created}>
                      {dateFormatter.format(new Date(version.date_created))}
                    </span>
                  </div>
                  <div>
                    <p class={styles.downloads}>
                      <span class={styles.label}>Downloads: </span>
                      <span class={styles.value}>
                        {numberFormatter.format(version.downloads)}
                      </span>
                    </p>
                  </div>
                </li>
              );
            }}
          </For>
        </ol>
      </div>
    </div>
  );
}
