import "./styles/App.css";
import "./styles/Markdown.css";

import { Route, Router } from "@solidjs/router";
import { platform } from "@tauri-apps/plugin-os";
import { open } from "@tauri-apps/plugin-shell";
import { Show, createEffect, createResource, createSignal, onCleanup, onMount } from "solid-js";

import { coreResources as coreResources } from "./globals";

import ErrorPage from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";
import ErrorBoundary, { Error } from "./components/global/ErrorBoundary";
import { closeSplashscreen, relaunch } from "./api/app";

export default function App() {
  const [fontLoaded] = createResource(async () => {
    // 64px taken from the title on game select screen
    await document.fonts.load("64px Inter");
  });
  const [coreResourcesLoaded, setCoreResourcesLoaded] = createSignal(false);

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

  createEffect(() => {
    if (coreResources.every((resource) => !resource.loading && resource.state !== "unresolved"))
      setCoreResourcesLoaded(true);
  });

  createEffect(async () => {
    if (coreResourcesLoaded() && !fontLoaded.loading) {
      // App ready, close splashscreen and show main window
      await closeSplashscreen();
    }
  });

  onMount(async () => {
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
      when={coreResources.every((resource) => !resource.loading)}
      fallback={
        <Show when={coreResources.find((resource) => resource.error != null)?.error}>
          {(err) => (
            <Error
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
