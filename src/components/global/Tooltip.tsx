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
}

export default function Tooltip({ content, children, showDelay, hideDelay }: TooltipProps) {
  return (
    <FloatingElement
      class={styles.tooltip}
      style={{
        "--tooltip-delay-start": showDelay ?? "0.1s",
        "--tooltip-delay-end": hideDelay ?? "0s",
      }}
      content={
        <p class={styles.tooltipText}>
          {/* <Show when={showArrow || showArrow === undefined}>
            <div class={styles.showArrow} aria-hidden="true"></div>
          </Show> */}
          {content}
        </p>
      }
      options={{
        middleware: [offset(6)],
      }}
    >
      {children}
    </FloatingElement>
  );
}
