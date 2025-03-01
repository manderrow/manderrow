import { useSearchParams } from "@solidjs/router";
import { Accessor, createEffect, For, JSX, Match, Setter, Switch } from "solid-js";

export interface PlainTab {
  id: string;
  name: string;
  // fallback?: JSX.Element;
  selected?: boolean;
  component: JSX.Element;
}

export interface DynamicTab extends Omit<PlainTab, "component"> {
  component: (data: any) => JSX.Element;
}

export type Tab = PlainTab | DynamicTab;

interface TabStyles {
  tabs: {
    container?: string;
    list: string;
    list__item: string;
    list__itemActive: string;
  };
}

export default function TabRenderer({
  id,
  tabs,
  styles,
  setter,
}: {
  id: string;
  tabs: Tab[];
  rootUrl?: string;
  styles: TabStyles;
  setter?: Setter<Tab>;
}) {
  const [searchParams, setSearchParams] = useSearchParams();
  const defaultTab = tabs.find((tab) => tab.selected)?.id ?? tabs[0].id;
  const tablistId = `${id}-tab`;
  const currentTab = () =>
    (Array.isArray(searchParams[tablistId]) ? searchParams[tablistId][0] : searchParams[tablistId]) ?? defaultTab; // First tab is the default

  const tabsMap = new Map<Tab[][number]["id"], Tab>(tabs.map((tab) => [tab.id, tab]));

  if (setter != null) {
    createEffect(() => {
      setter(tabsMap.get(currentTab())!);
    });
  }

  return (
    <>
      <div class={styles.tabs.container ?? ""}>
        <ul class={styles.tabs.list}>
          <For each={tabs}>
            {(tab) => (
              <li
                classList={{ [styles.tabs.list__item]: true, [styles.tabs.list__itemActive]: currentTab() === tab.id }}
              >
                <button on:click={() => setSearchParams({ [tablistId]: tab.id })}>{tab.name}</button>
              </li>
            )}
          </For>
        </ul>
      </div>

      {setter == null ? <TabContent currentTab={() => tabsMap.get(currentTab())!} tabs={tabs} /> : null}
    </>
  );
}

export function TabContent({ tabs, currentTab }: { tabs: Tab[]; currentTab: Accessor<Tab> }) {
  return (
    <Switch>
      <For each={tabs}>{(tab) => <Match when={currentTab().id === tab.id}>{tab.component}</Match>}</For>
    </Switch>
  );
}
