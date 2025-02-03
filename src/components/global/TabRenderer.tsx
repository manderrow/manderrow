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
    container?: string;
    list: string;
    list__item: string;
    list__itemActive: string;
  };
}

interface CustomJSX {
  beforeTabs: JSX.Element;
  afterTabs: JSX.Element;
}

export default function TabRenderer({
  tabs,
  rootUrl,
  styles,
  customJsx,
}: {
  tabs: Tabs[];
  rootUrl?: string;
  styles: TabStyles;
  customJsx?: Partial<CustomJSX>;
}) {
  const [searchParams] = useSearchParams();
  const defaultTab = tabs[0].id;
  const currentTab = () => searchParams.tab ?? defaultTab; // First tab is the default

  return (
    <>
      <div class={styles.tabs.container ?? ""}>
        {customJsx?.beforeTabs}
        <ul class={styles.tabs.list}>
          <For each={tabs}>
            {(tab) => (
              <li classList={{ [styles.tabs.list__item]: true, [styles.tabs.list__itemActive]: currentTab() === tab.id }}>
                <A href={`${rootUrl ?? ""}?tab=${tab.id}`}>{tab.name}</A>
              </li>
            )}
          </For>
        </ul>
        {customJsx?.afterTabs}
      </div>

      <Switch>
        <For each={tabs}>{(tab) => <Match when={currentTab() === tab.id}>{tab.component}</Match>}</For>
      </Switch>
    </>
  );
}
