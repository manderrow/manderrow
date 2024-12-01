import { For } from 'solid-js';
import { Mod } from '../../types';
import styles from './ModList.module.css';

export default function ModList(props: { mods: Mod[] }) {
  return <div class={styles.modList}>
    <For each={props.mods}>
      {mod => <div>
        <img class={styles.icon} src={mod.versions[0].icon} />
        <div class={styles.split}>
          <div class={styles.left}>
            <p class={styles.name}>{mod.full_name}</p>
            <div class={styles.categories}>
              <For each={mod.categories}>
                {category => <div>{category}</div>}
              </For>
            </div>
          </div>
          <div class={styles.right}>
            <p class={styles.downloads}>Downloads: {mod.versions[0].downloads ?? '0'}</p>
          </div>
        </div>
      </div>}
    </For>
  </div>;
}
