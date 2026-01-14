import { faXmark } from "@fortawesome/free-solid-svg-icons";
import type { RootProps } from "corvu/dialog";
import CorvuDialog from "corvu/dialog";
import Fa from "solid-fa";
import { JSX, Show, splitProps } from "solid-js";

import styles from "./Dialog.module.css";
import { t } from "../i18n/i18n";

export const dialogStyles = styles;

type DialogProps = Omit<RootProps, "contextId"> &
  JSX.IntrinsicElements["div"] & {
    onDismiss?: DismissCallback;
    anchorId?: string;
    hideCloseBtn?: boolean;
    trigger?: JSX.Element;
  };
export type DialogExternalProps = Omit<DialogProps, "children">;

type BtnType = "danger" | "ok" | "attention" | "info";
interface PromptDialogButton {
  type?: BtnType;
  text?: string;
  callback?: () => void;
}
type PromptDialogOptions = Omit<DialogExternalProps, "title"> & {
  title?: string | null;
  question: string;
  btns?: {
    ok?: PromptDialogButton;
    cancel?: PromptDialogButton;
  };
};

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

/*
  USAGE NOTE

  If wrapping a dialog in a <Show when> that displays after the press of a button,
  for some reason, because Corvu listens for the close event using the pointerup
  event, the dialog will immediately close after like 1 frame after opening. Not
  sure what to do about this yet.
*/

export function PromptDialog(options: PromptDialogOptions) {
  const [props, rest] = splitProps(options, ["title", "question", "btns"]);

  return (
    <Dialog class={styles.dialogDefault} {...rest}>
      <Show when={props.title !== undefined}>
        <h2 class={styles.dialog__title}>{props.title ?? t("global.dialogue_default_title")}</h2>
      </Show>
      <p class={styles.dialog__message}>{props.question}</p>
      <div class={styles.dialog__btns}>
        <button
          onClick={props.btns?.ok?.callback}
          classList={{ [styles.dialog__btnsBtn]: true, [getBtnTypeClass(props.btns?.ok?.type)]: true }}
        >
          {props.btns?.ok?.text ?? t("global.phrases.confirm")}
        </button>
        <DialogClose
          onClick={props.btns?.cancel?.callback}
          classList={{ [styles.dialog__btnsBtn]: true, [getBtnTypeClass(props.btns?.cancel?.type)]: true }}
        >
          {props.btns?.cancel?.text ?? t("global.phrases.cancel")}
        </DialogClose>
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

export function InfoDialog({ title, message }: { title?: string | null; message: string }) {
  return (
    <PromptDialog
      title={title}
      question={message}
      btns={{
        cancel: { text: t("global.phrases.ok") },
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
