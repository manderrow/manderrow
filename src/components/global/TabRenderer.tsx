import { Accessor, createEffect, For, JSX, Match, Switch } from "solid-js";
import { useSearchParamsInPlace } from "../../utils/router";

export interface Tab<Id extends string> {
  id: Id;
  name: string;
  // fallback?: JSX.Element;
  selected?: boolean;
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

/**
 * The first tab will be the default.
 */
export default function TabRenderer<Id extends string>({
  id,
  tabs,
  styles,
  setter,
}: {
  id: string;
  tabs: Tab<Id>[];
  rootUrl?: string;
  styles: TabStyles;
  setter?: (tab: Tab<Id>) => void;
}) {
  const [searchParams, setSearchParams] = useSearchParamsInPlace();

  const defaultTab = tabs.find((tab) => tab.selected)?.id ?? tabs[0].id;
  const tablistId = `${id}-tab`;
  const currentTab = () =>
    ((Array.isArray(searchParams[tablistId]) ? searchParams[tablistId][0] : searchParams[tablistId]) as Id) ??
    defaultTab;

  const tabsMap = Object.fromEntries(tabs.map((tab) => [tab.id, tab])) as Record<Id, Tab<Id>>;

  if (setter != null) {
    createEffect(() => {
      setter(tabsMap[currentTab()]);
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
                <button type="button" on:click={() => setSearchParams({ [tablistId]: tab.id })}>{tab.name}</button>
              </li>
            )}
          </For>
        </ul>
      </div>

      {setter == null ? <TabContent currentTab={() => tabsMap[currentTab()]} tabs={tabs} /> : null}
    </>
  );
}

export function TabContent<Id extends string>({
  tabs,
  currentTab,
}: {
  tabs: Tab<Id>[];
  currentTab: Accessor<Tab<Id>>;
}) {
  return (
    <Switch>
      <For each={tabs}>{(tab) => <Match when={currentTab().id === tab.id}>{tab.component}</Match>}</For>
    </Switch>
  );
}
