import { createSignal, JSX, onCleanup, onMount } from "solid-js";
import styles from "./Dropdown.module.css";

export type Alignment = "center" | "left" | "right";

export interface DropdownOptions {
  align?: Alignment;
  maxWidth?: string;
  children: JSX.Element;
}

const VIEWPORT_PADDING = 16;

function getDropdownTypeClass(type?: Alignment) {
  switch (type) {
    case "left":
      return styles.dropdownLeft;
    case "right":
      return styles.dropdownRight;
    case "center":
    default:
      return styles.dropdownCenter;
  }
}

export default function Dropdown(options: DropdownOptions) {
  let dropdownElement!: HTMLDivElement;
  let cutoffX = 0;

  const [offsetX, setOffsetX] = createSignal(0);

  function checkVisibility() {
    const delta =
      Math.max(document.documentElement.clientWidth || 0, window.innerWidth || 0) - VIEWPORT_PADDING - dropdownElement.getBoundingClientRect().right;
    cutoffX = Math.min(cutoffX + delta - VIEWPORT_PADDING, 0);

    setOffsetX(cutoffX);
  }

  onMount(() => {
    checkVisibility();
    window.addEventListener("resize", checkVisibility);
  });

  onCleanup(() => {
    window.removeEventListener("resize", checkVisibility);
  });

  return (
    <div
      classList={{ [styles.dropdown]: true, [getDropdownTypeClass(options.align)]: true }}
      ref={dropdownElement}
      style={{ transform: `translateX(${offsetX()}px)`, "max-width": options.maxWidth || "unset" }}
    >
      {options.children}
    </div>
  );
}
