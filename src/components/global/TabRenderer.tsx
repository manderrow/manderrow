import { createEffect, createSelector, For, JSX, Match, Switch, untrack } from "solid-js";

import { useSearchParam } from "../../utils/router";

import styles from "./TabRenderer.module.css";

export interface Tab<Id extends string> {
  id: Id;
  name: string;
  // fallback?: JSX.Element;
  selected?: boolean;
  component: () => JSX.Element;
}

type Preset = "none" | "base" | "basic" | "moving-bg";

interface TabStyles {
  container?: string;
  list: string;
  tab: string;
}

type StyleProps =
  | {
      classes: TabStyles;
      preset: "none";
    }
  | {
      classes?: Partial<TabStyles>;
      preset: Exclude<Preset, "none">;
    };

/**
 * The first tab will be the default.
 */
export default function TabRenderer<Id extends string>(props: {
  id: string;
  tabs: readonly Tab<Id>[];
  rootUrl?: string;
  styles: StyleProps;
  setter?: (tab: Tab<Id>) => void;
}) {
  const [currentTabNonDefaulted, setCurrentTab] = useSearchParam<Id>(`${props.id}-tab`);

  const defaultTab = props.tabs.find((tab) => tab.selected)?.id ?? props.tabs[0].id;
  const currentTab = () => (currentTabNonDefaulted() as Id | undefined) ?? defaultTab;

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
      <div
        class={
          props.styles.preset === "none"
            ? props.styles.classes.container
            : `${styles[props.styles.preset ?? "base"]} ${props.styles.classes?.container ?? ""}`
        }
      >
        <ul
          class={
            props.styles.preset === "none"
              ? props.styles.classes?.list
              : `${styles[props.styles.preset ?? "base"]} ${styles.list} ${props.styles.classes?.list ?? ""}`
          }
        >
          <For each={props.tabs}>
            {(tab) => (
              <li
                class={
                  props.styles.preset === "none"
                    ? props.styles.classes?.tab
                    : `${styles[props.styles.preset ?? "base"]} ${styles.tab} ${props.styles.classes?.tab ?? ""}`
                }
                data-active={isCurrentTab(tab.id)}
              >
                <button type="button" onClick={() => setCurrentTab(tab.id)}>
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
