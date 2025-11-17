import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { createSignal, onMount, Show } from "solid-js";

import { close, isMaximized, minimize, setMaximized, startDragging } from "../../api/app";
import { t } from "../../i18n/i18n";

import logo from "../../assets/Manderrow logo.svg";
import styles from "./TitleBar.module.css";

const appWindow = getCurrentWindow();

// Global signal for current profile name displayed in title bar
const [currentProfileName, _setCurrentProfileName] = createSignal("");

export const setCurrentProfileName = _setCurrentProfileName;

export default function TitleBar() {
  const [isMaximizedCached, setIsMaximizedCached] = createSignal(false);
  const [isFocusedCached, setIsFocusedCached] = createSignal(false);

  onMount(async () => {
    setIsMaximizedCached(await isMaximized());
    // TODO: custom event that includes the new status so we don't need to make another request
    appWindow.onResized(async () => {
      setIsMaximizedCached(await isMaximized());
    });

    appWindow.onFocusChanged((event) => {
      setIsFocusedCached(event.payload);
    });
  });

  return (
    <div class={styles.titlebar} data-focused={isFocusedCached()}>
      <div
        data-tauri-drag-region={platform() === "macos" ? undefined : true}
        class={styles.titlebar__content}
        on:mousedown={(e) => {
          if (e.buttons === 1) {
            // left click
            e.detail === 2 // if double click
              ? setMaximized()
              : startDragging();
          }
        }}
      >
        <div class={styles.appTitleContainer}>
          <img src={logo} alt="Manderrow logo" />
          <span class={styles.appTitle}>Manderrow</span>
        </div>
        <p class={styles.profileName}>{currentProfileName()}</p>
      </div>
      <div class={styles.controls}>
        <button title={t("titlebar.minimize_btn")} on:click={() => minimize()} data-minimize>
          {/* <!-- https://api.iconify.design/mdi:window-minimize.svg --> */}
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
            <path fill="currentColor" d="M19 13H5v-2h14z" />
          </svg>
        </button>
        <button
          title={isMaximizedCached() ? t("titlebar.restore_btn") : t("titlebar.maximize_btn")}
          on:click={() => setMaximized(!isMaximizedCached())}
          data-maximize
        >
          <Show
            when={isMaximizedCached()}
            fallback={
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
                {/* <!-- https://api.iconify.design/mdi:window-maximize.svg --> */}
                <path fill="currentColor" d="M4 4h16v16H4zm2 4v10h12V8z" />
              </svg>
            }
          >
            {/* <!-- https://api.iconify.design/mdi:window-restore.svg --> */}
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
              <path fill="currentColor" d="M4 8h4V4h12v12h-4v4H4zm12 0v6h2V6h-8v2zM6 12v6h8v-6z" />
            </svg>
          </Show>
        </button>
        <button title={t("global.phrases.close")} on:click={() => close()} data-close>
          {/* <!-- https://api.iconify.design/mdi:close.svg --> */}
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
            <path
              fill="currentColor"
              d="M13.46 12L19 17.54V19h-1.46L12 13.46L6.46 19H5v-1.46L10.54 12L5 6.46V5h1.46L12 10.54L17.54 5H19v1.46z"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}
