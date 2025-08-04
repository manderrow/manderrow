import { JSX } from "solid-js/jsx-runtime";
import styles from "./ContextMenu.module.css";
import TogglableDropdown from "./TogglableDropdown";

interface ContextMenuItem {
  label: JSX.Element | string;
  action: () => void;
}

interface ContextMenuSpacer {
  label: "spacer";
}

interface ContextMenuProps {
  items: (ContextMenuItem | ContextMenuSpacer)[];
  children: string | JSX.Element; // label element
  anchorId?: string; // Explicit ID of element to anchor to (needed for multiple floating elements on the same anchor)
}

/**
 * A wrapper for the TogglableDropdown component to create a context menu.
 * @param props Context menu data
 * @returns A button that toggles the context menu
 */
export default function ContextMenu(props: ContextMenuProps) {
  return (
    <TogglableDropdown
      label={props.children}
      labelBtnFocusable
      hideCaret
      anchorId={props.anchorId}
      dropdownOptions={{
        placement: "bottom-start",
        strategy: "fixed",
      }}
    >
      <ul class={styles.contextMenu}>
        {props.items.map((item) =>
          item.label === "spacer" ? (
            <li>
              <hr class={styles.contextMenuSpacer} />
            </li>
          ) : (
            <li class={styles.contextMenuItem}>
              <button onClick={(item as ContextMenuItem).action}>{item.label}</button>
            </li>
          ),
        )}
      </ul>
    </TogglableDropdown>
  );
}
