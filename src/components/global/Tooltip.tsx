import { JSX, Show } from "solid-js";
import { arrow } from "@floating-ui/dom";

import { FloatingElement } from "./FloatingElement";

import styles from "./Tooltip.module.css";

interface TooltipProps {
  content: string | JSX.Element;
  showArrow?: boolean;
  children: JSX.Element;
}

export default function Tooltip({ content, showArrow, children }: TooltipProps) {
  return (
    <FloatingElement
      class={styles.tooltip}
      content={
        <div class={styles.tooltip}>
          {/* <Show when={showArrow || showArrow === undefined}>
            <div class={styles.showArrow} aria-hidden="true"></div>
          </Show> */}
          {content}
        </div>
      }
    >
      {children}
    </FloatingElement>
  );
}
