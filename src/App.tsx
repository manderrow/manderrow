import "./styles/App.css";
import "./styles/Markdown.css";

import { Route, Router } from "@solidjs/router";
import { platform } from "@tauri-apps/plugin-os";
import { open } from "@tauri-apps/plugin-shell";
import { Show, createResource, onCleanup, onMount } from "solid-js";

import { relaunch } from "./api/app";
import { coreResources } from "./globals";

import ErrorBoundary, { ErrorDialog } from "./components/global/ErrorBoundary";
import ErrorPage from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";
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
    open(link.href).catch(() => alert(`Failed to open link: ${link.href}`));
  }

  function onLinkAuxClick(event: MouseEvent) {
    const link = getLink(event);
    if (link == null) return;

    event.preventDefault();
    if (link.target === "_blank" && event.button !== 2) {
      // Link is to open in external browser and not right clicked
      open(link.href).catch(() => alert(`Failed to open link: ${link.href}`));
    }
  }

  onMount(() => {
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
    <Show
      when={
        coreResources.every((resource) => !resource.loading && resource.state !== "unresolved") && !fontLoaded.loading
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
      <ErrorBoundary>
        <Router>
          <Route path="/" component={GameSelect} />
          <Route path="/profile/:gameId/:profileId?" component={Profile} />
          <Route path="*path" component={ErrorPage} />
        </Router>
      </ErrorBoundary>
    </Show>
  );
}
