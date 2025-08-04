import { JSX, onMount, Show } from "solid-js";
import { arrow, offset } from "@floating-ui/dom";

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

export default function Tooltip({ content, children, showDelay, hideDelay, anchorId }: TooltipProps) {
  return (
    <FloatingElement
      anchorId={anchorId}
      class={styles.tooltip}
      style={{
        "--tooltip-delay-start": showDelay ?? "0.1s",
        "--tooltip-delay-end": hideDelay ?? "0s",
      }}
      content={
        <>
          <p class={styles.tooltipText}>
            {/* <Show when={showArrow || showArrow === undefined}>
            <div class={styles.showArrow} aria-hidden="true"></div>
          </Show> */}
            {content}
          </p>

          <Show when={anchorId}>
            {/* Kinda chopped solution here to handle multiple/nested floating elements,
            hopefully can revise in the future. Styles copied from Tooltip.module.css:17 */}
            <style>
              {`
                #${anchorId}:hover ~ .${styles.tooltip}[data-anchor-id="${anchorId}"],
                #${anchorId}:focus-visible ~ .${styles.tooltip}[data-anchor-id="${anchorId}"]
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
        middleware: [offset(6)],
        strategy: "fixed",
      }}
    >
      {children}
    </FloatingElement>
  );
}
