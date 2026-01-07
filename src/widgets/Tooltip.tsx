import { JSX, Show } from "solid-js";

import CorvuTooltip from "corvu/tooltip";

import styles from "./Tooltip.module.css";

interface TooltipProps {
  content: string | JSX.Element;
  hideArrow?: boolean;
  // trigger element, default a button. Not strictly enforced by TypeScript
  // to be a Corvu Trigger component, hopefully can be fixed in the future
  children: ReturnType<typeof CorvuTooltip.Trigger>;
  showDelay?: number; // milliseconds
  hideDelay?: number; // milliseconds
  anchorId?: string; // Explicit ID of element to anchor to (needed for multiple floating elements on the same anchor)
}

/**
 * A simple utility that warps Corvu's tooltip boilerplate along with default styles and options.
 * @param props Tooltip props
 * @returns A rendered tooltip component
 */
export default function Tooltip(props: TooltipProps) {
  return (
    <CorvuTooltip
      contextId={props.anchorId}
      openDelay={props.showDelay || 50}
      closeDelay={props.hideDelay}
      floatingOptions={{
        offset: 6,
      }}
    >
      {props.children}

      <CorvuTooltip.Portal contextId={props.anchorId}>
        <CorvuTooltip.Content contextId={props.anchorId} class={styles.tooltip} style={{ "pointer-events": "none" }}>
          <Show when={!props.hideArrow}>
            {/* TODO: make universal custom arrow SVG */}
            <CorvuTooltip.Arrow />
          </Show>

          <span class={styles.tooltipText}>{props.content}</span>
        </CorvuTooltip.Content>
      </CorvuTooltip.Portal>
    </CorvuTooltip>
  );
}

export const TooltipTrigger = CorvuTooltip.Trigger;
export const TooltipAnchor = CorvuTooltip.Anchor;

// In case manual use-cases want the default styles
export const tooltipClasses = styles;
