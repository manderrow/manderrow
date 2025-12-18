import { createMemo, createSignal, createUniqueId, For, JSX, Show, splitProps } from "solid-js";
import { createStore } from "solid-js/store";
import { faChevronLeft } from "@fortawesome/free-solid-svg-icons";
import { Fa } from "solid-fa";
import { OverlayScrollbarsComponent } from "overlayscrollbars-solid";
import { Channel } from "@tauri-apps/api/core";

import {
  importModpackFromThunderstoreCode,
  Modpack,
  ModProgressRegistration,
  ModSpec,
  previewImportModpackFromThunderstoreCode,
} from "../../api";
import { t } from "../../i18n/i18n.ts";
// @ts-ignore: typescript is unaware of solid's use: syntax
import { bindValue } from "../global/Directives";
import { refetchProfiles } from "../../globals";
import { initProgress, Listener, Id as TaskId, tasks } from "../../api/tasks";

import ErrorBoundary from "../global/ErrorBoundary";
import { SimpleAsyncButton } from "../global/AsyncButton";
import { SimpleProgressIndicator } from "../global/Progress";
import MultistageModel, { BaseStageProps } from "../global/MultistageModel";
import { type DialogExternalProps, DialogClose } from "../global/Dialog";

import styles from "./ImportDialog.module.css";

interface ChoosePageProps {
  gameId: string;
  profile?: string;
}

export default function ImportDialog(props: ChoosePageProps & DialogExternalProps) {
  const [local, rest] = splitProps(props, ["gameId", "profile"]);

  return (
    <MultistageModel
      initialStage={(actions) => ({
        title: t("profile.import_model.import_title"),
        element: () => <ChoosePage gameId={local.gameId} profile={local.profile} actions={actions} />,
      })}
      onDismiss={rest.onDismiss}
      estimatedStages={2}
      {...rest}
    />
  );
}

function ChoosePage(props: ChoosePageProps & BaseStageProps) {
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
      element: PreviewPage,
      args: {
        thunderstoreCode: thunderstoreCode(),
        gameId: props.gameId,
        profile: props.profile,
        modpack: modpack,
        actions: props.actions,
      },
    });
  }

  return (
    <form on:submit={(e) => e.preventDefault()}>
      <div class={styles.importInputGroup}>
        <label for={thunderstoreCodeFieldId}>{t("profile.import_model.thunderstore_code_label")}</label>
        <input id={thunderstoreCodeFieldId} use:bindValue={[thunderstoreCode, setThunderstoreCode]} required></input>
      </div>

      <div class={styles.buttonRow}>
        <DialogClose onClick={props.actions.dismiss}>{t("global.phrases.cancel")}</DialogClose>
        <SimpleAsyncButton progress type="submit" onClick={onSubmitThunderstoreCode}>
          {t("global.phrases.next")}
        </SimpleAsyncButton>
      </div>
    </form>
  );
}

interface PreviewPageProps extends BaseStageProps {
  thunderstoreCode: string;
  gameId: string;
  profile?: string;
  modpack: Modpack;
}

function PreviewPage(props: PreviewPageProps) {
  return <ErrorBoundary>{PreviewPageInner(props)}</ErrorBoundary>;
}

function PreviewPageInner(props: PreviewPageProps) {
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
    props.actions.dismiss?.();
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
          <DialogClose onClick={props.actions.dismiss}>{t("global.phrases.cancel")}</DialogClose>
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
