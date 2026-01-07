import { Accessor, createSignal, For, JSX, Show, splitProps } from "solid-js";
import { createStore } from "solid-js/store";

import Dialog, { DismissCallback, DialogClose, DialogExternalProps } from "./Dialog.tsx";

import styles from "./MultistageModel.module.css";
import { t } from "../i18n/i18n.ts";

interface Stage<TArgs = void> {
  title: string;
  element: TArgs extends void ? () => JSX.Element : (args: TArgs) => JSX.Element;
  args?: TArgs extends void ? never : TArgs;
  buttons?: CallbackButtons;
}
interface CallbackButton {
  text?: string;
  callback: () => void;
}
export interface CallbackButtons {
  next: CallbackButton;
  previous: CallbackButton;
}
export interface BaseStageProps {
  actions: Actions;
}
export interface Actions {
  pushStage: <TArgs = void>(stage: Stage<TArgs>) => void;
  popStage: () => void;
  dismiss?: DismissCallback;
}

interface Props {
  initialStage: (actions: Actions) => Stage<any>;
  estimatedStages: number;
  onDismiss?: DismissCallback;
}
function ModelStepsDisplay(props: { estimated: Accessor<number>; stages: Stage<any>[] }) {
  const stageIndex = () => props.stages.length - 1;

  return (
    <ol class={styles.stages}>
      <For each={new Array(Math.max(props.estimated(), stageIndex() + 1))}>
        {(_, i) => (
          <li class={styles.stage} aria-selected={i() == stageIndex()}>
            <div aria-hidden class={styles.indicator}></div>
            <span class={styles.title}>
              Step {i() + 1}
              <Show when={i() < props.stages.length}> - {props.stages[i()].title}</Show>
            </span>
          </li>
        )}
      </For>
    </ol>
  );
}

export default function MultistageModel(props: Props & DialogExternalProps) {
  const [local, rest] = splitProps(props, ["initialStage", "estimatedStages"]);

  let modelElement!: HTMLDivElement;
  let childElement!: HTMLDivElement;

  const actions: Actions = {
    pushStage: (stage: Stage<any>) => setStack(stack.length, stage),
    popStage: () => setStack((stages) => stages.slice(0, -1)),
    dismiss: rest.onDismiss,
  };

  const [stack, setStack] = createStore<Stage<any>[]>([local.initialStage(actions)]);
  const [modelHeight, setModelHeight] = createSignal(0);

  const currentStage = () => stack[stack.length - 1];

  function updateModelHeight() {
    setModelHeight(Math.min(innerHeight, childElement.clientHeight));
  }

  const [resizeObserver, setResizeObserver] = createSignal<ResizeObserver>();

  return (
    <Dialog
      class={styles.model}
      style={{ "--computed-height": `${modelHeight()}px` }}
      ref={modelElement}
      hideCloseBtn
      onContentPresentChange={(present) => {
        if (present) {
          const resizeObserver = new ResizeObserver(updateModelHeight);
          resizeObserver.observe(childElement);
          setResizeObserver(resizeObserver);
        } else {
          resizeObserver()?.disconnect();
          setResizeObserver(undefined);
          setModelHeight(0);
        }
      }}
      {...rest}
    >
      <div class={styles.container} ref={childElement}>
        <ModelStepsDisplay estimated={() => local.estimatedStages} stages={stack} />
        <h2 class={styles.stageTitle}>{currentStage().title}</h2>
        <div class={styles.content}>
          {currentStage().args !== undefined
            ? (currentStage().element as (args: any) => JSX.Element)(currentStage().args)
            : (currentStage().element as () => JSX.Element)()}
        </div>
        <Show when={currentStage().buttons}>
          {(buttons) => (
            <div class={styles.navBtns}>
              <Show when={stack.length > 1}>
                <button onClick={buttons().previous.callback}>
                  {buttons().previous.text || t("global.phrases.previous")}
                </button>
              </Show>

              <DialogClose onClick={rest.onDismiss}>{t("global.phrases.cancel")}</DialogClose>
              <button onClick={buttons().next.callback}>{buttons().next.text || t("global.phrases.next")}</button>
            </div>
          )}
        </Show>
      </div>
    </Dialog>
  );
}
