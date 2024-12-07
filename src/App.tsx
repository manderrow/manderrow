import { Route, Router } from "@solidjs/router";

import "./App.css";

import { ErrorBoundary, Show, createEffect, createRenderEffect, createSignal, onCleanup, onMount } from "solid-js";

import { gamesResource } from "./globals";

import ErrorPage from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";
import { invoke } from "@tauri-apps/api/core";

export default function App() {
  const [loaded, setLoaded] = createSignal(false);

  onMount(async () => {
    // 64px taken from the title on game select screen
    await document.fonts.load('64px Inter');
    setLoaded(true);
  });

  createEffect(async () => {
    if (gamesResource.latest != null && loaded()) {
      // App ready, close splashscreen and show main window
      await invoke("close_splashscreen");
    }
  });

  return (
    <ErrorBoundary fallback={ErrorPage}>
      <Show when={gamesResource.latest != null}>
        <Router>
          <Route path="/" component={GameSelect} />
          <Route path="/profile/:gameId/:profileId?" component={Profile} />
          <Route path="*path" component={ErrorPage} />
        </Router>
      </Show>
    </ErrorBoundary>
  );
}
