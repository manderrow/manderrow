import { IconDefinition } from "@fortawesome/free-regular-svg-icons";
import { faAdd, faRemove } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { createEffect, createResource, For, JSX, Match, onMount, Show, Switch } from "solid-js";

import { PathComponent, readModConfig, scanModConfigs, Value } from "../../api/configs";
import { t } from "../../i18n/i18n";
import { useSearchParam } from "../../utils/router";

import styles from "./ConfigEditor.module.css";

function sectionId(path: readonly PathComponent[]): string {
  return path.map((c) => (typeof c === "string" ? `s-${c}` : c)).join(".");
}

export default function ConfigEditor(props: { profile: string }) {
  const [currentFile, setCurrentFile] = useSearchParam("mod-config-path");

  const [configs, { refetch: _refreshConfigs }] = createResource(() => scanModConfigs(props.profile));

  const [currentConfig, { refetch: _refreshConfig }] = createResource(currentFile, (currentFile) =>
    readModConfig(props.profile, currentFile),
  );

  createEffect(() => {
    // Should run once on page load to select the first config file
    if (!currentFile() && configs.latest) setCurrentFile(configs.latest[0]);
  });

  return (
    <div class={styles.container}>
      <aside>
        <div class={styles.configs}>
          <h1>{t("config.title")}</h1>
          <Show when={configs.latest} fallback="Loading...">
            {(configs) => (
              <ul>
                <For each={configs()}>
                  {(path) => (
                    <li classList={{ [styles.active]: currentFile() === path }}>
                      <button onClick={() => setCurrentFile(path)}>{path}</button>
                    </li>
                  )}
                </For>
              </ul>
            )}
          </Show>
        </div>
      </aside>

      <Show when={currentConfig()} fallback="Loading...">
        {(currentConfig) => (
          <>
            <div class={styles.editor}>
              <For each={currentConfig().sections}>
                {(section) => (
                  <EntryEditor
                    key={section.path.join(".")}
                    value={section.value}
                    id={sectionId(section.path)}
                    onClick={() => {}}
                  />
                )}
              </For>
            </div>

            <aside>
              <div class={styles.sectionsOverviewContent}>
                <h2>{t("config.sections_title")}</h2>
                <ul>
                  <For each={currentConfig().sections}>
                    {(section) => (
                      <li>
                        <a href={`#${sectionId(section.path)}`}>{section.path.join(".")}</a>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            </aside>
          </>
        )}
      </Show>
    </div>
  );
}

function ValueEditor(props: { value: Value }) {
  return (
    <Switch>
      <Match when={Array.isArray(props.value)}>
        <div class={styles.valueEditor} data-type="array">
          <ol>
            <For each={props.value as Value[]}>
              {(value, i) => <EntryEditor key={i()} value={value} onClick={() => {}} />}
            </For>
          </ol>
          <div class={styles.entry}>
            <AddButton onClick={() => {}} />
          </div>
        </div>
      </Match>
      <Match when={props.value instanceof Object}>
        <div class={styles.valueEditor} data-type="object">
          <ul>
            <For each={Object.keys(props.value as Record<string, Value>)}>
              {(key) => (
                <EntryEditor key={key} value={(props.value as Record<string, Value>)[key]!} onClick={() => {}} />
              )}
            </For>
          </ul>
          <div class={styles.entry}>
            <AddButton onClick={() => {}} />
          </div>
        </div>
      </Match>
      <Match when={true}>
        <div class={styles.valueEditor}>{props.value?.toString()}</div>
      </Match>
    </Switch>
  );
}

function EntryEditor(props: { key: JSX.Element; value: Value; id?: string; onClick: () => void }) {
  return (
    <li class={styles.entry} id={props.id}>
      <span class={styles.entry__key}>{props.key}</span>
      <ValueEditor value={props.value} />
      <div class={styles.entry__actions}>
        <IconButton action="delete" icon={faRemove} onClick={() => props.onClick()} />
      </div>
    </li>
  );
}

function IconButton(props: { action: string; icon: IconDefinition; onClick: () => void }) {
  return (
    <button
      class={styles.editorIconBtn}
      title={props.action}
      data-action={props.action}
      onClick={() => props.onClick()}
    >
      <Fa icon={props.icon} scale="0.75em" />
    </button>
  );
}

function AddButton(props: { onClick: () => void }) {
  return (
    <button class={styles.addBtn} onClick={props.onClick}>
      <Fa icon={faAdd} scale="0.9em" />
      Add
    </button>
  );
}
