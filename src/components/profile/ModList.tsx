import { createSignal, For, Setter, Show, Signal } from 'solid-js';
import { ModAndVersion } from '../../types';
import styles from './ModList.module.css';
import { Temporal } from '@js-temporal/polyfill';
import { createInfiniteScroll } from '@solid-primitives/pagination';

const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

export type Fetcher = (page: number) => Promise<ModAndVersion[]>;

function ModListLeft({ mods, selectedMod: [selectedMod, setSelectedMod] }: { mods: Fetcher, selectedMod: Signal<ModAndVersion | undefined> }) {
  const [paginatedMods, infiniteScrollLoader, { end }] = createInfiniteScroll(mods);

  return <div class={styles.scrollOuter}>
    <ol class={`${styles.modList} ${styles.scrollInner}`}>
      <For each={paginatedMods()}>
        {mod => <li classList={{ [styles.selected]: selectedMod() === mod }}>
          <button on:click={() => setSelectedMod(selectedMod() === mod ? undefined : mod)}>
            <img class={styles.icon} src={mod.mod.versions[0].icon} />
            <div class={styles.split}>
              <div class={styles.left}>
                <div>
                  <span class={styles.name}>{mod.mod.name}</span> <span class={styles.version}>v
                    <Show when={mod.version} fallback={mod.mod.versions[0].version_number}>
                      {version => version()}
                    </Show>
                  </span>
                </div>
                <div class={styles.owner}><span class={styles.label}>@</span><span class={styles.value}>{mod.mod.owner}</span></div>
                <div class={styles.categories}>
                  <For each={mod.mod.categories}>
                    {category => <div>{category}</div>}
                  </For>
                </div>
              </div>
              <div class={styles.right}>
                <p class={styles.downloads}><span class={styles.label}>Downloads: </span><span class={styles.value}>{mod.mod.versions[0].downloads ?? '0'}</span></p>
              </div>
            </div>
          </button>
        </li>}
      </For>
      <Show when={!end()}>
        <li use:infiniteScrollLoader>Loading...</li>
      </Show>
    </ol>
  </div>;
}

export default function ModList(props: { mods: Fetcher }) {
  const [selectedMod, setSelectedMod] = createSignal<ModAndVersion>();

  return <div class={styles.modListAndView}>
    <Show when={props.mods} keyed>
      {mods => <ModListLeft mods={mods} selectedMod={[selectedMod, setSelectedMod]} />}
    </Show>
    <Show when={selectedMod()}>
      {mod => <div class={styles.scrollOuter}>
        <div class={`${styles.modView} ${styles.scrollInner}`}>
          <h2 class={styles.name}>{mod().mod.name}</h2>
          <p class={styles.description}>{mod().mod.versions[0].description}</p>

          <h3>Versions</h3>
          <ol class={styles.versions}>
            <For each={mod().mod.versions}>
              {version => {
                const timestamp = Temporal.Instant.from(version.date_created).toZonedDateTime({ timeZone: Temporal.Now.timeZoneId(), calendar: 'gregory' });
                return <li>
                  <div>
                    <span class={styles.version}>{version.version_number}</span>
                    <span> - </span>
                    <span class={styles.timestamp} title={timestamp.toLocaleString()}>{MONTHS[timestamp.month - 1]} {timestamp.day}, {timestamp.year} at {timestamp.hour > 12 ? timestamp.hour - 12 : timestamp.hour === 0 ? 12 : timestamp.hour}:{timestamp.minute} {timestamp.hour >= 12 ? "PM" : "AM"}</span>
                  </div>
                  <div>
                    <p class={styles.downloads}><span class={styles.label}>Downloads: </span><span class={styles.value}>{version.downloads ?? '0'}</span></p>
                  </div>
                </li>;
              }}
            </For>
          </ol>
        </div>
      </div>}
    </Show>
  </div>;
}
