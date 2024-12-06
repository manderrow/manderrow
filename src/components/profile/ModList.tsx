import { createInfiniteScroll } from "@solid-primitives/pagination";
import { createSignal, For, Show, Signal } from "solid-js";

import { ModAndVersion } from "../../types";
import { numberFormatter } from "../../utils";

import styles from "./ModList.module.css";

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
  hour: "numeric",
  minute: "numeric",
});

export type Fetcher = (page: number) => Promise<ModAndVersion[]>;

export default function ModList(props: { mods: Fetcher }) {
  const [selectedMod, setSelectedMod] = createSignal<ModAndVersion>();

  return (
    <div class={styles.modListAndView}>
      <Show when={props.mods} keyed>
        {(mods) => <ModListLeft mods={mods} selectedMod={[selectedMod, setSelectedMod]} />}
      </Show>
      <Show when={selectedMod()}>
        {(mod) => (
          <div class={styles.scrollOuter}>
            <div class={`${styles.modView} ${styles.scrollInner}`}>
              <h2 class={styles.name}>{mod().mod.name}</h2>
              <p class={styles.description}>{mod().mod.versions[0].description}</p>

              <h3>Versions</h3>
              <ol class={styles.versions}>
                <For each={mod().mod.versions}>
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
                            <span class={styles.value}>{numberFormatter.format(version.downloads)}</span>
                          </p>
                        </div>
                      </li>
                    );
                  }}
                </For>
              </ol>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
}

function ModListLeft({ mods, selectedMod: [selectedMod, setSelectedMod] }: { mods: Fetcher; selectedMod: Signal<ModAndVersion | undefined> }) {
  const [paginatedMods, infiniteScrollLoader, { end }] = createInfiniteScroll(mods);

  return (
    <div class={styles.scrollOuter}>
      <ol class={`${styles.modList} ${styles.scrollInner}`}>
        <For each={paginatedMods()}>
          {(mod) => (
            <li classList={{ [styles.selected]: selectedMod() === mod }}>
              <button on:click={() => setSelectedMod(selectedMod() === mod ? undefined : mod)}>
                <img class={styles.icon} src={mod.mod.versions[0].icon} />
                <div class={styles.split}>
                  <div class={styles.left}>
                    <div>
                      <span class={styles.name}>{mod.mod.name}</span>{" "}
                      <span class={styles.version}>
                        v
                        <Show when={mod.version} fallback={mod.mod.versions[0].version_number}>
                          {(version) => version()}
                        </Show>
                      </span>
                    </div>
                    <div class={styles.owner}>
                      <span class={styles.label}>@</span>
                      <span class={styles.value}>{mod.mod.owner}</span>
                    </div>
                    <ul class={styles.categories}>
                      <For each={mod.mod.categories}>{(category) => <li>{category}</li>}</For>
                    </ul>
                  </div>
                  <div class={styles.right}>
                    <p class={styles.downloads}>
                      <span class={styles.label}>Downloads: </span>
                      <span class={styles.value}>{numberFormatter.format(mod.mod.versions[0].downloads)}</span>
                    </p>
                  </div>
                </div>
              </button>
            </li>
          )}
        </For>
        <Show when={!end()}>
          <li use:infiniteScrollLoader>Loading...</li>
        </Show>
      </ol>
    </div>
  );
}
