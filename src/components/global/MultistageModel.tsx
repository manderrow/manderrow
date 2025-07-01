import { Accessor, createEffect, createSignal, For, JSX, onCleanup, onMount, Show } from "solid-js";
import { createStore } from "solid-js/store";

import Dialog, { DismissCallback } from "./Dialog.tsx";

import styles from "./MultistageModel.module.css";

interface Stage {
  title: string;
  element: JSX.Element;
  buttons?: CallbackButtons;
}
interface CallbackButton {
  text?: string;
  callback: () => void;
}
export interface CallbackButtons {
  next: CallbackButton;
  previous: CallbackButton;
  dismiss?: DismissCallback;
}
export interface InitialStageProps {
  actions: Actions;
}
export interface InitialSetupProps {
  dismiss: DismissCallback;
}
export interface Actions {
  pushStage: (stage: Stage) => void;
  popStage: () => void;
  dismiss: DismissCallback;
}

interface Props {
  initialStage: (actions: Actions) => Stage;
  estimatedStages: number;
  onDismiss: DismissCallback;
}
function ModelStepsDisplay(props: { estimated: Accessor<number>; stages: Stage[] }) {
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

export default function MultistageModel(props: Props) {
  let modelElement!: HTMLDivElement;
  let childElem!: HTMLDivElement;

  const actions: Actions = {
    pushStage: (stage: Stage) => setStack(stack.length, stage),
    popStage: () => setStack((stages) => stages.slice(0, -1)),
    dismiss: props.onDismiss,
  };

  const [stack, setStack] = createStore<Stage[]>([props.initialStage(actions)]);
  const [modelHeight, setModelHeight] = createSignal(0);

  const currentStage = () => stack[stack.length - 1];

  function updateModelHeight() {
    setModelHeight(Math.min(innerHeight, childElem.clientHeight));
  }

  onMount(() => {
    addEventListener("resize", updateModelHeight);
  });

  createEffect(() => {
    currentStage();
    updateModelHeight();
  });

  onCleanup(() => {
    removeEventListener("resize", updateModelHeight);
  });

  return (
    <Dialog>
      <div class={styles.model} style={{ "--computed-height": `${modelHeight()}px` }} ref={modelElement}>
        <div class={styles.container} ref={childElem}>
          <ModelStepsDisplay estimated={() => props.estimatedStages} stages={stack} />
          <h2 class={styles.stageTitle}>{currentStage().title}</h2>
          <div class={styles.content}>{currentStage().element}</div>
          <Show when={currentStage().buttons}>
            {(buttons) => (
              <div class={styles.navBtns}>
                <Show when={stack.length > 1}>
                  <button onClick={buttons().previous.callback}>{buttons().previous.text || "Previous"}</button>
                </Show>

                <button onClick={buttons().dismiss}>Cancel</button>
                <button onClick={buttons().next.callback}>{buttons().next.text || "Next"}</button>
              </div>
            )}
          </Show>
        </div>
      </div>
    </Dialog>
  );
}
