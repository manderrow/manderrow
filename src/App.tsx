import "./App.css";

import { Route, Router } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { platform } from "@tauri-apps/plugin-os";
import { open } from "@tauri-apps/plugin-shell";
import { Resource, Show, createEffect, createSignal, onCleanup, onMount } from "solid-js";

import { gamesPopularityResource, gamesResource } from "./globals";

import ErrorPage from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";
import ErrorBoundary from "./components/global/ErrorBoundary";

const resources: Resource<any>[] = [gamesResource, gamesPopularityResource];

export default function App() {
  const [loaded, setLoaded] = createSignal(false);
  const [ready, setReady] = createSignal(false);

  function onLinkClick(event: MouseEvent) {
    if (!(event.target instanceof HTMLAnchorElement)) return;
    if (event.target.href.startsWith(`${window.location.protocol}//${window.location.host}`)) return;

    event.preventDefault();
    open(event.target.href);
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
  });

  onCleanup(() => {
    delete document.body.dataset.webview;
    document.removeEventListener("click", onLinkClick);
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
