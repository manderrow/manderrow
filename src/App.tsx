import { Route, Router } from "@solidjs/router";

import "./App.css";

import { Show } from "solid-js";
import { gamesResource } from "./globals";
import Error from "./pages/error/Error";
import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";

export default function App() {
  return (
    <Show when={gamesResource.latest}>
      <Router>
        <Route path="/" component={GameSelect} />
        <Route path="/profile/:gameId/:profileId?" component={Profile} />
        <Route path="*404" component={Error} />
      </Router>
    </Show>
  );
}
