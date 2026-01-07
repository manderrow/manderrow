import { createSignal, FlowProps, JSX, Ref, Show } from "solid-js";
import Fa from "solid-fa";
import { faCaretDown } from "@fortawesome/free-solid-svg-icons";
import Popover from "corvu/popover";
import type { RootProps as PopoverRootProps } from "corvu/popover";

import styles from "./TogglableDropdown.module.css";

export interface TogglableDropdownOptions {
  floatingContainerClass?: JSX.HTMLAttributes<HTMLElement>["class"];
  label: string | JSX.Element;
  labelClass?: JSX.HTMLAttributes<HTMLElement>["class"];
  dropdownClass?: JSX.HTMLAttributes<HTMLElement>["class"];
  hideCaret?: boolean; // caret in toggle button
  showArrow?: boolean; // arrow on dropdown
  anchorId?: string; // Explicit ID of element to anchor to (needed for multiple floating elements on the same anchor)
  popoverProps?: Omit<PopoverRootProps, "children" | "contextId">;
  offset?: number;
  fillToTriggerWidth?: boolean;
  ref?: Ref<HTMLDivElement>;
}

export default function TogglableDropdown(props: FlowProps & TogglableDropdownOptions) {
  let triggerRef!: HTMLButtonElement;
  let contentRef!: HTMLDivElement;
  const [resizeObserver, setResizeObserver] = createSignal<ResizeObserver | null>(null);

  return (
    <Popover
      floatingOptions={{
        offset: props.offset ?? (props.showArrow ? 10 : 6),
        flip: true,
        shift: true,
      }}
      contextId={props.anchorId}
      onContentPresentChange={(present) => {
        if (!present) {
          resizeObserver()?.disconnect();
          setResizeObserver(null);
          return;
        }

        if (props.fillToTriggerWidth) {
          function updateWidth() {
            contentRef.style.setProperty("--fill-width", `${triggerRef.offsetWidth}px`);
          }

          updateWidth();

          const resizeObserver = new ResizeObserver(updateWidth);
          resizeObserver.observe(triggerRef);
          setResizeObserver(resizeObserver);
        }
      }}
      {...props.popoverProps}
    >
      <Popover.Trigger
        as="button"
        classList={{ [styles.toggle]: true, [props.labelClass || styles.labelDefault]: true }}
        contextId={props.anchorId}
        ref={triggerRef}
      >
        <Show when={!props.hideCaret}>
          <Fa icon={faCaretDown} class={styles.toggle__icon} />
        </Show>
        {props.label}
      </Popover.Trigger>
      <Popover.Portal contextId={props.anchorId}>
        <Popover.Content
          class={`${styles.dropdownBase} ${props.floatingContainerClass || ""}`}
          contextId={props.anchorId}
          ref={contentRef}
        >
          <Popover.Label class="phantom" contextId={props.anchorId}>
            {props.label}
          </Popover.Label>
          <div class={`${styles.dropdownDefault} ${props.dropdownClass || ""}`} ref={props.ref}>
            {props.children}
          </div>

          <Show when={props.showArrow}>
            <Popover.Arrow contextId={props.anchorId} />
          </Show>
        </Popover.Content>
      </Popover.Portal>
    </Popover>
  );
}
