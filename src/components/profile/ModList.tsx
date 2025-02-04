import { createInfiniteScroll } from "@solid-primitives/pagination";
import { Accessor, createSignal, For, Show, Signal } from "solid-js";

import { ModAndVersion } from "../../types";
import { numberFormatter, roundedNumberFormatter } from "../../utils";

import styles from "./ModList.module.css";
import Fa from "solid-fa";
import { faDownload, faDownLong, faExternalLink } from "@fortawesome/free-solid-svg-icons";
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
      <ModView selectedMod={selectedMod} />
    </div>
  );
}

function ModView({ selectedMod }: { selectedMod: Accessor<ModAndVersion | undefined> }) {
  return (
    <div class={styles.scrollOuter}>
      <div class={`${styles.modView} ${styles.scrollInner}`}>
        <Show
          when={selectedMod()}
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
                <h2 class={styles.name}>{mod().mod.name}</h2>
                <p class={styles.description}>{mod()!.mod.owner}</p>
                <p class={styles.description}>{mod()!.mod.versions[0].description}</p>
              </div>

              <form class={styles.modView__downloader} action="#">
                <select class={styles.versions}>
                  <For each={mod()!.mod.versions}>
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
                    <Show when={mod()!.mod.package_url != null}>
                      <a href={mod()!.mod.package_url} target="_blank" rel="noopener noreferrer">
                        <Fa icon={faExternalLink} /> Website
                      </a>
                    </Show>
                  </li>
                  <li>
                    <Show when={mod()!.mod.donation_link != null}>
                      <a href={mod()!.mod.donation_link} target="_blank" rel="noopener noreferrer">
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
            </>
          )}
        </Show>
      </div>
    </div>
  );
}

function ModListMods({ mods, selectedMod: [selectedMod, setSelectedMod] }: { mods: Fetcher; selectedMod: Signal<ModAndVersion | undefined> }) {
  const [paginatedMods, infiniteScrollLoader, { end }] = createInfiniteScroll(mods);

  function selectMod(mod: ModAndVersion) {
    setSelectedMod(selectedMod() === mod ? undefined : mod);
  }

  return (
    <div class={styles.scrollOuter}>
      <ol class={`${styles.modList} ${styles.scrollInner}`}>
        <For each={paginatedMods()}>
          {(mod) => (
            <li classList={{ [styles.mod]: true, [styles.selected]: selectedMod() === mod }}>
              <div
                on:click={() => selectMod(mod)}
                onKeyDown={(key) => {
                  if (key.key === "Enter") selectMod(mod);
                }}
                class={styles.mod__btn}
                role="button"
                aria-pressed={selectedMod() === mod}
                tabIndex={0}
              >
                <img class={styles.icon} src={mod.mod.versions[0].icon} />
                <div class={styles.mod__content}>
                  <div class={styles.left}>
                    <p class={styles.info}>
                      <span class={styles.name}>{mod.mod.name}</span>
                      <span class={styles.separator} aria-hidden>
                        &bull;
                      </span>
                      <span class={styles.owner}>{mod.mod.owner}</span>
                    </p>
                    <p class={styles.downloads}>
                      <Fa icon={faDownload} /> {roundedNumberFormatter.format(mod.mod.versions.map((v) => v.downloads).reduce((acc, x) => acc + x))}
                    </p>
                    <p class={styles.description}>{mod.mod.versions[0].description}</p>
                  </div>
                  <div class={styles.right}>
                    <button>
                      <Fa icon={faDownLong} />
                    </button>
                  </div>
                </div>
              </div>
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
