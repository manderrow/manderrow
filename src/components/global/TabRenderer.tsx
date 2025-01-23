import { A, useSearchParams } from "@solidjs/router";
import { For, JSX, Match, Switch } from "solid-js";

interface Tabs {
  id: string;
  name: string;
  // fallback?: JSX.Element;
  component: JSX.Element;
}

interface TabStyles {
  tabs: {
    list: string;
    list__item: string;
    list__itemActive: string;
  };
}

export default function TabRenderer({ tabs, root, styles }: { tabs: Tabs[]; root?: string; styles: TabStyles }) {
  const [searchParams] = useSearchParams();
  const defaultTab = tabs[0].id;
  const currentTab = () => searchParams.tab ?? defaultTab; // First tab is the default

  return (
    <>
      <ul class={styles.tabs.list}>
        <For each={tabs}>
          {(tab) => (
            <li classList={{ [styles.tabs.list__item]: true, [styles.tabs.list__itemActive]: currentTab() === tab.id }}>
              <A href={`${root ?? ""}?tab=${tab.id}`}>{tab.name}</A>
            </li>
          )}
        </For>
      </ul>

      <Switch>
        <For each={tabs}>{(tab) => <Match when={currentTab() === tab.id}>{tab.component}</Match>}</For>
      </Switch>
    </>
  );
}
