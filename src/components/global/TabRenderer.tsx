import { createEffect, createSelector, For, JSX, Match, Switch, untrack } from "solid-js";
import { useSearchParamsInPlace } from "../../utils/router";

export interface Tab<Id extends string> {
  id: Id;
  name: string;
  // fallback?: JSX.Element;
  selected?: boolean;
  component: () => JSX.Element;
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
export default function TabRenderer<Id extends string>(props: {
  id: string;
  tabs: readonly Tab<Id>[];
  rootUrl?: string;
  styles: TabStyles;
  setter?: (tab: Tab<Id>) => void;
}) {
  const [searchParams, setSearchParams] = useSearchParamsInPlace();

  const defaultTab = props.tabs.find((tab) => tab.selected)?.id ?? props.tabs[0].id;
  const tablistId = `${props.id}-tab`;
  const currentTab = () =>
    ((Array.isArray(searchParams[tablistId]) ? searchParams[tablistId]![0] : searchParams[tablistId]) as Id) ??
    defaultTab;

  const tabsMap = Object.fromEntries(props.tabs.map((tab) => [tab.id, tab])) as Record<Id, Tab<Id>>;

  createEffect(() => {
    const setter = props.setter;
    if (setter != null) {
      const tab = tabsMap[currentTab()];
      untrack(() => setter(tab));
    }
  });

  const isCurrentTab = createSelector(currentTab);

  return (
    <>
      <div class={props.styles.tabs.container ?? ""}>
        <ul class={props.styles.tabs.list}>
          <For each={props.tabs}>
            {(tab) => (
              <li
                classList={{
                  [props.styles.tabs.list__item]: true,
                  [props.styles.tabs.list__itemActive]: isCurrentTab(tab.id),
                }}
              >
                <button type="button" on:click={() => setSearchParams({ [tablistId]: tab.id })}>
                  {tab.name}
                </button>
              </li>
            )}
          </For>
        </ul>
      </div>

      {props.setter == null ? <TabContent isCurrentTab={isCurrentTab} tabs={props.tabs} /> : null}
    </>
  );
}

export function TabContent<Id extends string>(props: { tabs: readonly Tab<Id>[]; isCurrentTab: (id: Id) => boolean }) {
  return (
    <Switch>
      <For each={props.tabs}>{(tab) => <Match when={props.isCurrentTab(tab.id)}>{tab.component()}</Match>}</For>
    </Switch>
  );
}
