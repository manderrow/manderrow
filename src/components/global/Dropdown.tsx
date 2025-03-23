import { createEffect, createSignal, JSX, onCleanup, onMount } from "solid-js";
import styles from "./Dropdown.module.css";
import { Portal } from "solid-js/web";

export type Alignment = "center" | "left" | "right";

export interface DropdownOptions {
  align?: Alignment;
  maxWidth?: string;
  children: JSX.Element;
  class?: string;
  ref?: (element: HTMLElement) => void;
}

const VIEWPORT_PADDING = 16;

function getDropdownAlignClass(alignment: Alignment | undefined) {
  switch (alignment) {
    case "left":
      return styles.left;
    case "right":
      return styles.right;
    case "center":
    default:
      return styles.center;
  }
}

function getDropdownAlignMultiplier(alignment: Alignment | undefined) {
  switch (alignment) {
    case "left":
      return 0.0;
    case "right":
      return 1.0;
    case "center":
    default:
      return 0.5;
  }
}

function getDropdownOffset(alignment: Alignment | undefined, mountPointRect: DOMRect) {
  switch (alignment) {
    case "left":
      return mountPointRect.left;
    case "right":
      return mountPointRect.left + mountPointRect.width;
    case "center":
    default:
      return mountPointRect.left + mountPointRect.width / 2;
  }
}

export default function Dropdown(options: DropdownOptions) {
  let mountPointElement!: HTMLDivElement;
  let dropdownElement!: HTMLButtonElement;

  const [offsetX, setOffsetX] = createSignal(0);
  const [offsetY, setOffsetY] = createSignal(0);

  function checkVisibility() {
    const mountPointRect = mountPointElement.getBoundingClientRect();

    setOffsetX(
      Math.min(
        getDropdownOffset(options.align, mountPointRect),
        Math.max(Math.max(document.documentElement.clientWidth || 0, window.innerWidth || 0) - VIEWPORT_PADDING, 0) -
          dropdownElement.getBoundingClientRect().width * getDropdownAlignMultiplier(options.align),
      ),
    );
    setOffsetY(mountPointRect.top);
  }

  onMount(() => {
    checkVisibility();
    window.addEventListener("resize", checkVisibility);

    if (options.ref !== undefined) {
      options.ref(dropdownElement);
    }
  });

  onCleanup(() => {
    window.removeEventListener("resize", checkVisibility);
  });

  createEffect(() => {
    if (options.ref !== undefined) {
      options.ref(dropdownElement);
    }
  });

  return (
    <div class={styles.mountPoint} ref={mountPointElement}>
      <Portal>
        <button
          classList={{
            [styles.dropdown]: true,
            [getDropdownAlignClass(options.align)]: true,
            [options.class || ""]: true,
          }}
          ref={dropdownElement}
          // style={{ transform: `translate(${offsetX()}px, ${offsetY()}px)`, "max-width": options.maxWidth || "unset" }}
          style={{
            "--offset-y": `${offsetY()}px`,
            left: `${offsetX()}px`,
            top: "var(--offset-y)",
            "max-width": options.maxWidth || "unset",
          }}
        >
          <div
            class={styles.content}
            style={{
              // idk why, but this div really wants to overflow if I set max-height on the parent instead
              "max-height": `calc(100dvh - ${VIEWPORT_PADDING*2}px - var(--offset-y))`,
            }}
          >
            {options.children}
          </div>
        </button>
      </Portal>
    </div>
  );
}
