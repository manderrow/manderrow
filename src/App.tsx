import "./styles/App.css";
import "./styles/Markdown.css";

import { Route, Router } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { platform } from "@tauri-apps/plugin-os";
import { open } from "@tauri-apps/plugin-shell";
import { Resource, Show, createEffect, createSignal, onCleanup, onMount } from "solid-js";

import { gamesModDownloadsResource, gamesPopularityResource, gamesResource } from "./globals";

import ErrorPage from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";
import ErrorBoundary from "./components/global/ErrorBoundary";

const resources: Resource<any>[] = [gamesResource, gamesPopularityResource, gamesModDownloadsResource];

export default function App() {
  const [loaded, setLoaded] = createSignal(false);
  const [ready, setReady] = createSignal(false);

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

  onMount(async () => {
    // 64px taken from the title on game select screen
    await document.fonts.load("64px Inter");
    setLoaded(true);
  });

  createEffect(() => {
    if (resources.every((resource) => resource.latest != null)) setReady(true);
  });

  createEffect(async () => {
    if (ready() && loaded()) {
      // App ready, close splashscreen and show main window
      await invoke("close_splashscreen");
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
    <ErrorBoundary>
      <Show when={ready()}>
        <Router>
          <Route path="/" component={GameSelect} />
          <Route path="/profile/:gameId/:profileId?" component={Profile} />
          <Route path="*path" component={ErrorPage} />
        </Router>
      </Show>
    </ErrorBoundary>
  );
}
