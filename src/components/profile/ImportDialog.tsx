import { createMemo, createSignal, createUniqueId, For, JSX, Show } from "solid-js";

import {
  importModpackFromThunderstoreCode,
  Modpack,
  ModProgressRegistration,
  ModSpec,
  previewImportModpackFromThunderstoreCode,
} from "../../api";
import { DefaultDialog, DismissCallback } from "../global/Dialog";
import { bindValue } from "../global/Directives";

import { faChevronLeft } from "@fortawesome/free-solid-svg-icons";
import { OverlayScrollbarsComponent } from "overlayscrollbars-solid";
import { Fa } from "solid-fa";
import { createStore } from "solid-js/store";
import { refetchProfiles } from "../../globals";
import ErrorBoundary from "../global/ErrorBoundary";
import { SimpleAsyncButton } from "../global/AsyncButton";
import { initProgress, Listener, Id as TaskId, tasks } from "../../api/tasks";
import { Channel } from "@tauri-apps/api/core";
import { SimpleProgressIndicator } from "../global/Progress";

import styles from "./ImportDialog.module.css";
import MultistageModel, { Actions, InitialSetupProps, InitialStageProps } from "../global/MultistageModel.tsx";
import { t } from "../../i18n/i18n.ts";

interface ChoosePageProps {
  gameId: string;
  profile?: string;
}

export default function ImportDialog(props: ChoosePageProps & InitialSetupProps) {
  return (
    <MultistageModel
      initialStage={(actions) => ({
        title: t("profile.import_model.import_title"),
        element: <ChoosePage gameId={props.gameId} profile={props.profile} actions={actions} />,
      })}
      onDismiss={props.dismiss}
      estimatedStages={2}
    />
  );
}

function ChoosePage(props: ChoosePageProps & InitialStageProps) {
  const thunderstoreCodeFieldId = createUniqueId();

  const [thunderstoreCode, setThunderstoreCode] = createSignal("");

  async function onSubmitThunderstoreCode(listener: Listener) {
    const modpack = await previewImportModpackFromThunderstoreCode(
      thunderstoreCode(),
      props.gameId,
      props.profile,
      listener,
    );
    props.actions.pushStage({
      title: t("profile.import_model.preview_title"),
      element: (
        <PreviewPage
          thunderstoreCode={thunderstoreCode()}
          gameId={props.gameId}
          profile={props.profile}
          modpack={modpack}
          actions={props.actions}
        />
      ),
    });
  }

  return (
    <form on:submit={(e) => e.preventDefault()}>
      <div class={styles.importInputGroup}>
        <label for={thunderstoreCodeFieldId}>Thunderstore Code</label>
        <input id={thunderstoreCodeFieldId} use:bindValue={[thunderstoreCode, setThunderstoreCode]} required></input>
      </div>

      <div class={styles.buttonRow}>
        <button on:click={props.actions.dismiss}>Cancel</button>
        <SimpleAsyncButton progress type="submit" onClick={onSubmitThunderstoreCode}>
          Next
        </SimpleAsyncButton>
      </div>
    </form>
  );
}

function PreviewPage(props: {
  thunderstoreCode: string;
  gameId: string;
  profile?: string;
  modpack: Modpack;
  actions: Actions;
}) {
  return <ErrorBoundary>{PreviewPageInner(props)}</ErrorBoundary>;
}

function PreviewPageInner(props: {
  thunderstoreCode: string;
  gameId: string;
  profile?: string;
  modpack: Modpack;
  actions: Actions;
}) {
  let [modProgress, setModProgress] = createStore<Record<string, TaskId>>({});

  async function onImport(listener: Listener) {
    const modProgressChannel = new Channel<ModProgressRegistration>();
    modProgressChannel.onmessage = (info) => {
      setModProgress(info.url, info.task);
    };
    const id = await importModpackFromThunderstoreCode(
      props.thunderstoreCode,
      props.gameId,
      props.profile,
      modProgressChannel,
      listener,
    );
    console.log(`Imported to profile ${id}`);
    props.actions.dismiss();
    await refetchProfiles();
  }

  return (
    <>
      <ErrorBoundary>
        <h2>
          <button style={{ display: "inline-block" }} on:click={props.actions.popStage}>
            <Fa icon={faChevronLeft} />{" "}
          </button>{" "}
          {props.modpack.name}
        </h2>
        <div class={styles.preview}>
          <OverlayScrollbarsComponent defer options={{ scrollbars: { autoHide: "leave" } }}>
            <h3>Mods</h3>
            <ul>
              <For each={props.modpack.mods}>{(mod) => <ModEntry mod={mod} modProgress={modProgress} />}</For>
            </ul>

            <h3>Files</h3>
            <ul>
              <For each={props.modpack.diff}>
                {(diff) => (
                  <li>
                    <strong>[{diff.diff}]</strong> {diff.path}
                  </li>
                )}
              </For>
            </ul>
          </OverlayScrollbarsComponent>
        </div>
        <div class={styles.buttonRow}>
          <button on:click={props.actions.dismiss}>Cancel</button>
          <SimpleAsyncButton progress onClick={onImport}>
            Import
          </SimpleAsyncButton>
        </div>
      </ErrorBoundary>
    </>
  );
}

const THUNDERSTORE_URL_PATTERN =
  /https:\/\/thunderstore\.io\/package\/download\/([a-zA-Z0-9_]+)\/([a-zA-Z0-9_]+)\/(\d+\.\d+\.\d+)\//;
const THUNDERSTORE_CDN_URL_PATTERN =
  /https:\/\/gcdn\.thunderstore\.io\/live\/repository\/packages\/([a-zA-Z0-9_]+)-([a-zA-Z0-9_]+)-(\d+\.\d+\.\d+).zip/;

function detectModSource(mod: ModSpec): { name: string; version: string; author?: string; source: string } | undefined {
  switch (mod.type) {
    case "Online": {
      const match = mod.url.match(THUNDERSTORE_URL_PATTERN) ?? mod.url.match(THUNDERSTORE_CDN_URL_PATTERN);
      if (match !== null) {
        const [_, namespace, name, version] = match;
        return {
          name,
          version,
          author: namespace,
          source: "Thunderstore",
        };
      }
      break;
    }
    default:
      throw new Error();
  }
}

function ModEntry(props: { mod: ModSpec; modProgress: Record<string, TaskId> }) {
  const metadata = createMemo(() => detectModSource(props.mod));
  return (
    <li class={styles.modEntry}>
      <Show when={metadata()} fallback={<ModSource mod={props.mod} />}>
        {(metadata) => (
          <>
            <span>{metadata().name}</span> v<span>{metadata().version}</span>
            <Show when={metadata().author}>
              {(author) => (
                <>
                  {" "}
                  by <span>{author()}</span>
                </>
              )}
            </Show>
            <Show when={props.modProgress[props.mod.url]}>
              {(taskId) => (
                <div class={styles.right}>
                  <SimpleProgressIndicator progress={tasks().get(taskId())?.progress ?? initProgress()} />
                </div>
              )}
            </Show>
          </>
        )}
      </Show>
    </li>
  );
}

function ModSource(props: { mod: ModSpec }) {
  return createMemo(() => {
    const mod = props.mod;
    switch (mod.type) {
      case "Online":
        return <>{mod.url}</>;
      default:
        throw new Error();
    }
  }) as unknown as JSX.Element;
}
