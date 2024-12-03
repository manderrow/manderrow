import { Route, Router } from "@solidjs/router";

import "./App.css";

import { ErrorBoundary, Show } from "solid-js";

import { gamesResource } from "./globals";

import ErrorPage from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";

export default function App() {
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
