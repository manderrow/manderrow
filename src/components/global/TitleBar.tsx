import { getCurrentWindow } from "@tauri-apps/api/window";
import { createSignal, onMount } from "solid-js";

import { t } from "../../i18n/i18n";
import styles from "./Titlebar.module.css";

const appWindow = getCurrentWindow();

// Global signal for current profile name displayed in title bar
const [currentProfileName, _setCurrentProfileName] = createSignal("");

export const setCurrentProfileName = _setCurrentProfileName;

export default function TitleBar() {
  const [isMaximized, setIsMaximized] = createSignal(false);

  onMount(async () => {
    setIsMaximized(await appWindow.isMaximized());
    appWindow.onResized(async () => {
      setIsMaximized(await appWindow.isMaximized());
    });
  });

  return (
    <div class={styles.titlebar}>
      <div data-tauri-drag-region class={styles.titlebar__content}>
        <div class={styles.appTitleContainer}>
          {/* TODO: insert app logo */}
          <span class={styles.appTitle}>Manderrow</span>
        </div>
        <p class={styles.profileName}>{currentProfileName()}</p>
      </div>
      <div class={styles.controls}>
        <button title={t("titlebar.minimize_btn")} on:click={() => appWindow.minimize()} data-minimize>
          {/* <!-- https://api.iconify.design/mdi:window-minimize.svg --> */}
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
            <path fill="currentColor" d="M19 13H5v-2h14z"></path>
          </svg>
        </button>
        <button
          title={isMaximized() ? t("titlebar.restore_btn") : t("titlebar.maximize_btn")}
          on:click={async () => {
            isMaximized() ? appWindow.unmaximize() : appWindow.maximize();
          }}
          data-maximize
        >
          {/* <!-- https://api.iconify.design/mdi:window-maximize.svg --> */}
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
            <path fill="currentColor" d="M4 4h16v16H4zm2 4v10h12V8z"></path>
          </svg>
        </button>
        <button title={t("titlebar.close_btn")} on:click={() => appWindow.close()} data-close>
          {/* <!-- https://api.iconify.design/mdi:close.svg --> */}
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">
            <path
              fill="currentColor"
              d="M13.46 12L19 17.54V19h-1.46L12 13.46L6.46 19H5v-1.46L10.54 12L5 6.46V5h1.46L12 10.54L17.54 5H19v1.46z"
            ></path>
          </svg>
        </button>
      </div>
    </div>
  );
}
