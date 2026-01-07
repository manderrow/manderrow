import { JSX, Show, splitProps } from "solid-js";
import CorvuDialog from "corvu/dialog";
import type { RootProps } from "corvu/dialog";

import styles from "./Dialog.module.css";
import Fa from "solid-fa";
import { faXmark } from "@fortawesome/free-solid-svg-icons";

export const dialogStyles = styles;

type BtnType = "danger" | "ok" | "attention" | "info";
interface PromptDialogButton {
  type?: BtnType;
  text?: string;
  callback?: () => void;
}
interface PromptDialogOptions {
  title?: string | null;
  question: string;
  btns?: {
    ok?: PromptDialogButton;
    cancel?: PromptDialogButton;
  };
}

function getBtnTypeClass(type?: BtnType) {
  switch (type) {
    case "attention":
      return styles.dialog__btnsBtnAttention;
    case "danger":
      return styles.dialog__btnsBtnDanger;
    case "ok":
      return styles.dialog__btnsBtnOk;
    case "info":
      return styles.dialog__btnsBtnInfo;
    default:
      return "";
  }
}

export type DismissCallback = () => void;

type DialogProps = Omit<RootProps, "contextId"> &
  JSX.IntrinsicElements["div"] & {
    onDismiss?: DismissCallback;
    anchorId?: string;
    hideCloseBtn?: boolean;
    trigger?: JSX.Element;
  };
export type DialogExternalProps = Omit<DialogProps, "children">;

export function PromptDialog({ options }: { options: PromptDialogOptions }) {
  return (
    <Dialog class={styles.dialogDefault}>
      <Show when={options.title !== undefined}>
        <h2 class={styles.dialog__title}>{options.title ?? "Confirm Decision"}</h2>
      </Show>
      <p class={styles.dialog__message}>{options.question}</p>
      <div class={styles.dialog__btns}>
        <button
          on:click={options?.btns?.ok?.callback}
          classList={{ [styles.dialog__btnsBtn]: true, [getBtnTypeClass(options?.btns?.ok?.type)]: true }}
        >
          {options?.btns?.ok?.text ?? "Confirm"}
        </button>
        <button
          on:click={options?.btns?.cancel?.callback}
          classList={{ [styles.dialog__btnsBtn]: true, [getBtnTypeClass(options?.btns?.cancel?.type)]: true }}
        >
          {options.btns?.cancel?.text ?? "Cancel"}
        </button>
      </div>
    </Dialog>
  );
}

export function DefaultDialog(props: DialogProps) {
  return (
    <Dialog {...props} class={`${styles.dialogDefault} ${props.class ?? ""}`}>
      {props.children}
    </Dialog>
  );
}

export function InfoDialog({ title, message }: { title: string | null; message: string }) {
  return (
    <PromptDialog
      options={{
        title,
        question: message,
        btns: {
          cancel: { text: "Ok" },
        },
      }}
    />
  );
}

export default function Dialog(props: DialogProps) {
  const [rootProps, contentProps] = splitProps(props, ["anchorId", "onDismiss", "trigger", "hideCloseBtn", "class"]);

  return (
    <CorvuDialog
      {...props}
      contextId={rootProps.anchorId}
      onOpenChange={(open) => {
        if (!open) {
          rootProps.onDismiss?.();
        }
      }}
    >
      {rootProps.trigger}

      <CorvuDialog.Portal contextId={rootProps.anchorId}>
        <CorvuDialog.Overlay class={styles.overlay} contextId={rootProps.anchorId} />
        <CorvuDialog.Content
          class={`${styles.dialog} ${rootProps.class ?? ""}`}
          contextId={rootProps.anchorId}
          {...contentProps}
        >
          <Show when={!rootProps.hideCloseBtn}>
            <CorvuDialog.Close class={styles.dialog__closeBtn}>
              <Fa icon={faXmark} />
            </CorvuDialog.Close>
          </Show>

          {contentProps.children}
        </CorvuDialog.Content>
      </CorvuDialog.Portal>
    </CorvuDialog>
  );
}

export const DialogTrigger = CorvuDialog.Trigger;
export const DialogClose = CorvuDialog.Close;
