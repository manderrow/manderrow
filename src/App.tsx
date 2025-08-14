import "./styles/App.css";
import "./styles/Theme.css";
import "./styles/Fonts.css";
import "./styles/Markdown.css";

import { openUrl } from "@tauri-apps/plugin-opener";
import { platform } from "@tauri-apps/plugin-os";
import { Show, createResource, lazy, onCleanup, onMount } from "solid-js";

import { relaunch } from "./api/app";
import { coreResources } from "./globals";

import ErrorDialog from "./components/global/ErrorDialog";
import TitleBar from "./components/global/TitleBar.tsx";
import Splashscreen from "./pages/splashscreen/Splashscreen.tsx";

export default function App() {
  const [fontLoaded] = createResource(async () => {
    await document.fonts.load("16px Inter");
    document.fonts.load("16px SourceCodePro"); // non-blocking load
  });

  function getLink(event: MouseEvent) {
    if (!(event.target instanceof HTMLElement)) return;
    return event.target.closest("a");
  }

  function onLinkClick(event: MouseEvent) {
    const link = getLink(event);
    if (link == null || link.target === "_blank") return;

    if (link.href.startsWith(`${location.protocol}//${location.host}`)) return;

    event.preventDefault();
    openUrl(link.href).catch(() => alert(`Failed to open link: ${link.href}`));
  }

  function onLinkAuxClick(event: MouseEvent) {
    const link = getLink(event);
    if (link == null) return;

    event.preventDefault();
    if (event.button !== 2) {
      // Link was not right clicked, likely middle click
      openUrl(link.href).catch(() => alert(`Failed to open link: ${link.href}`));
    }
  }

  const AppLoaded = lazy(() => import("./AppLoaded"));

  onMount(() => {
    // Preload the AppLoaded component while waiting for globals and performing other
    // initialization. This simply loads the component's code so it is ready for
    // rendering when globals are ready.
    AppLoaded.preload();

    const platformName = platform();
    document.body.dataset.webview = platformName === "macos" || platformName === "ios" ? "safari" : platformName;

    document.addEventListener("click", onLinkClick);
    document.addEventListener("auxclick", onLinkAuxClick);
  });

  onCleanup(() => {
    delete document.body.dataset.webview;
    document.removeEventListener("click", onLinkClick);
    document.removeEventListener("auxclick", onLinkAuxClick);
  });

  return (
    <>
      <TitleBar />
      <Show
        when={
          coreResources.every((resource) => resource.state !== "pending" && resource.state !== "unresolved") &&
          !fontLoaded.loading
        }
        fallback={
          <Show when={coreResources.find((resource) => resource.error != null)?.error} fallback={<Splashscreen />}>
            {(err) => (
              <ErrorDialog
                err={err}
                reset={async () => {
                  await relaunch();
                }}
              />
            )}
          </Show>
        }
      >
        <AppLoaded />
      </Show>
    </>
  );
}
