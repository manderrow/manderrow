import { createInfiniteScroll } from "@solid-primitives/pagination";
import { Accessor, createSignal, For, Show, Signal } from "solid-js";

import { ModAndVersion } from "../../types";
import { numberFormatter } from "../../utils";

import styles from "./ModList.module.css";
import Fa from "solid-fa";
import { faDownload, faExternalLink } from "@fortawesome/free-solid-svg-icons";
import { faHeart } from "@fortawesome/free-regular-svg-icons";

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
        {(mods) => <ModListMods mods={mods} selectedMod={[selectedMod, setSelectedMod]} />}
      </Show>
      <Show when={selectedMod()}>{(mod) => <ModView mod={mod} />}</Show>
    </div>
  );
}

function ModView({ mod }: { mod: Accessor<ModAndVersion> }) {
  return (
    <div class={styles.scrollOuter}>
      <div class={`${styles.modView} ${styles.scrollInner}`}>
        <div>
          <h2 class={styles.name}>{mod().mod.name}</h2>
          <p class={styles.description}>{mod().mod.owner}</p>
          <p class={styles.description}>{mod().mod.versions[0].description}</p>
        </div>

        <form class={styles.modView__downloader} action="#">
          <select class={styles.versions}>
            <For each={mod().mod.versions}>
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

        <div>
          <h4>Links</h4>
          <ul>
            <li>
              <Show when={mod().mod.package_url != null}>
                <a href={mod().mod.package_url} target="_blank" rel="noopener noreferrer">
                  <Fa icon={faExternalLink} /> Website
                </a>
              </Show>
            </li>
            <li>
              <Show when={mod().mod.donation_link != null}>
                <a href={mod().mod.donation_link} target="_blank" rel="noopener noreferrer">
                  <Fa icon={faHeart} /> Donate
                </a>
              </Show>
            </li>
          </ul>
          {/* <span class={styles.version}>{version.version_number}</span>
                <span> - </span>
                <span class={styles.timestamp} title={version.date_created}>
                  {dateFormatter.format(new Date(version.date_created))}
                </span> */}
          {/* <p class={styles.downloads}>
                  <span class={styles.label}>Downloads: </span>
                  <span class={styles.value}>{numberFormatter.format(version.downloads)}</span>
                </p> */}
        </div>
      </div>
    </div>
  );
}

function ModListMods({ mods, selectedMod: [selectedMod, setSelectedMod] }: { mods: Fetcher; selectedMod: Signal<ModAndVersion | undefined> }) {
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
                    <p>
                      <span class={styles.name}>{mod.mod.name}</span>{" "}
                      <span class={styles.version}>
                        v
                        <Show when={mod.version} fallback={mod.mod.versions[0].version_number}>
                          {(version) => version()}
                        </Show>
                      </span>
                    </p>
                    <p class={styles.owner}>
                      <span class={styles.value}>{mod.mod.owner}</span>
                    </p>
                  </div>
                  <div class={styles.right}>
                    <p class={styles.downloads}>
                      <span class={styles.value}>
                        <Fa icon={faDownload} /> {numberFormatter.format(mod.mod.versions.map((v) => v.downloads).reduce((acc, x) => acc + x))}
                      </span>
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
