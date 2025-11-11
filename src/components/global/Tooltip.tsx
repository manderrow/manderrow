import { JSX, Show } from "solid-js";
import { offset, shift } from "@floating-ui/dom";

import { FloatingElement } from "./FloatingElement";

import styles from "./Tooltip.module.css";

interface TooltipProps {
  content: string | JSX.Element;
  showArrow?: boolean;
  children: JSX.Element;
  showDelay?: string;
  hideDelay?: string;
  anchorId?: string; // Explicit ID of element to anchor to (needed for multiple floating elements on the same anchor)
}

export default function Tooltip(props: TooltipProps) {
  return (
    <FloatingElement
      anchorId={props.anchorId}
      class={styles.tooltip}
      style={{
        "--tooltip-delay-start": props.showDelay ?? "0.1s",
        "--tooltip-delay-end": props.hideDelay ?? "0s",
      }}
      content={
        <>
          <p class={styles.tooltipText}>
            {/* <Show when={showArrow || showArrow === undefined}>
            <div class={styles.showArrow} aria-hidden="true"></div>
          </Show> */}
            {props.content}
          </p>

          <Show when={props.anchorId}>
            {/* Kinda chopped solution here to handle multiple/nested floating elements,
            hopefully can revise in the future. Styles copied from Tooltip.module.css:17 */}
            <style>
              {`
                #${props.anchorId}:hover ~ .${styles.tooltip}[data-anchor-id="${props.anchorId}"],
                #${props.anchorId}:focus-visible ~ .${styles.tooltip}[data-anchor-id="${props.anchorId}"]
                {
                  visibility: visible;
                  opacity: 1;
                  transition-delay: var(--tooltip-delay-start, 0.1s);
                }
              `}
            </style>
          </Show>
        </>
      }
      options={{
        middleware: [offset(6), shift({ padding: 4 })],
        strategy: "fixed",
      }}
    >
      {props.children}
    </FloatingElement>
  );
}
