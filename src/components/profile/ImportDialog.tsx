import { createMemo, createSignal, createUniqueId, For, JSX, Show } from "solid-js";

import { Modpack, ModSpec, previewImportModpackFromThunderstoreCode } from "../../api";
import { DefaultDialog, DismissCallback } from "../global/Dialog";
import { bindValue } from "../global/Directives";

import { faChevronLeft } from "@fortawesome/free-solid-svg-icons";
import { OverlayScrollbarsComponent } from "overlayscrollbars-solid";
import Fa from "solid-fa";
import { createStore } from "solid-js/store";
import styles from "./ImportDialog.module.css";

interface Actions {
  dismiss?: () => void;
  pushPage: (page: JSX.Element) => void;
  popPage: () => void;
}

export default function ImportDialog(props: { profile?: string; onDismiss?: DismissCallback }) {
  const [stack, setStack] = createStore<JSX.Element[]>([]);
  const actions: Actions = {
    dismiss: props.onDismiss,
    pushPage: (page) => setStack(stack.length, page),
    popPage: () => setStack((pages) => pages.slice(0, -1)),
  };
  setStack(0, <ChoosePage profile={props.profile} actions={actions} />);

  return <DefaultDialog onDismiss={props.onDismiss}>{stack[stack.length - 1]}</DefaultDialog>;
}

function ChoosePage(props: { profile?: string; actions: Actions }) {
  const thunderstoreCodeFieldId = createUniqueId();

  const [thunderstoreCode, setThunderstoreCode] = createSignal("");

  async function onSubmitThunderstoreCode(e: SubmitEvent) {
    e.preventDefault();

    const modpack = await previewImportModpackFromThunderstoreCode(thunderstoreCode(), props.profile);
    props.actions.pushPage(<PreviewPage modpack={modpack} actions={props.actions} />);
  }

  return (
    <>
      <h2>Import</h2>
      <form on:submit={onSubmitThunderstoreCode}>
        <label for={thunderstoreCodeFieldId}>Thunderstore Code:</label>
        <input id={thunderstoreCodeFieldId} use:bindValue={[thunderstoreCode, setThunderstoreCode]}></input>
        <div class={styles.buttonRow}>
          <button on:click={props.actions.dismiss}>Cancel</button>
          <button type="submit">Import</button>
        </div>
      </form>
    </>
  );
}

function PreviewPage(props: { modpack: Modpack; actions: Actions }) {
  return (
    <>
      <h2>
        <button style={{ display: "inline-block" }} on:click={props.actions.popPage}>
          <Fa icon={faChevronLeft} />{" "}
        </button>{" "}
        {props.modpack.name}
      </h2>
      <div class={styles.preview}>
        <OverlayScrollbarsComponent defer options={{ scrollbars: { autoHide: "leave" } }}>
          <h3>Mods</h3>
          <ul>
            <For each={props.modpack.mods}>{(mod) => <ModEntry mod={mod} />}</For>
          </ul>

          <h3>Files</h3>
          <ul>
            <For each={props.modpack.diff}>{(diff) => <li><strong>[{diff.diff}]</strong> {diff.path}</li>}</For>
          </ul>
        </OverlayScrollbarsComponent>
      </div>
      <div class={styles.buttonRow}>
        <button on:click={props.actions.dismiss}>Cancel</button>
        <button>Import</button>
      </div>
    </>
  );
}

const THUNDERSTORE_URL_PATTERN =
  /https:\/\/thunderstore.io\/package\/download\/([a-zA-Z0-9_]+)\/([a-zA-Z0-9_]+)\/(\d+\.\d+\.\d+)\//;

function detectModSource(mod: ModSpec): { name: string; version: string; author?: string; source: string } | undefined {
  switch (mod.type) {
    case "Online":
      const match = mod.url.match(THUNDERSTORE_URL_PATTERN);
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
    default:
      throw new Error();
  }
}

function ModEntry(props: { mod: ModSpec }) {
  const metadata = createMemo(() => detectModSource(props.mod));
  return (
    <li>
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
