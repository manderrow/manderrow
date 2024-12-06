import { Route, Router } from "@solidjs/router";

import "./App.css";

import { ErrorBoundary, Show, createEffect, createSignal, onCleanup, onMount } from "solid-js";

import { gamesResource } from "./globals";

import ErrorPage from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";
import { invoke } from "@tauri-apps/api/core";

export default function App() {
  const [loaded, setLoaded] = createSignal(false);

  function onLoaded() {
    setLoaded(true);
  }

  onMount(() => {
    // Note: This method is better than awaiting document.fonts.ready, despite requiring more code
    document.fonts.addEventListener("loadingdone", onLoaded);
  });

  createEffect(async () => {
    if (gamesResource.latest != null && loaded()) {
      // App ready, close splashscreen and show main window
      await invoke("close_splashscreen");
      document.fonts.removeEventListener("loadingdone", onLoaded);
    }
  });

  onCleanup(() => {
    document.fonts.removeEventListener("loadingdone", onLoaded);
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
