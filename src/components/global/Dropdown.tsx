import { createSignal, onCleanup, onMount } from "solid-js";
import styles from "./Dropdown.module.css";

type Alignment = "center" | "left" | "right";
interface DropdownOptions {
  align: Alignment;
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
    window.addEventListener("resize", checkVisibility);
    checkVisibility();
  });

  onCleanup(() => {
    window.removeEventListener("resize", checkVisibility);
  });

  return (
    <div
      classList={{ [styles.dropdown]: true, [getDropdownTypeClass(options.align)]: true }}
      ref={dropdownElement}
      style={{ transform: `translateX(${offsetX()}px)` }}
    >
      <p>
        Lorem ipsum dolor sit amet consectetur adipisicing elit. Provident, illo! Lorem ipsum dolor sit amet consectetur, adipisicing elit. Accusamus omnis eius
      </p>
      <p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Provident, illo!</p>
      <p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Provident, illo!</p>
      <p>Lorem ipsum dolor sit amet consectetur adipisicing elit. Provident, illo!</p>
    </div>
  );
}
