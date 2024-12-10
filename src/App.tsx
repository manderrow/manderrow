import "./App.css";

import { Route, Router } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { platform } from "@tauri-apps/plugin-os";
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
  });

  onCleanup(() => {
    delete document.body.dataset.webview;
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
