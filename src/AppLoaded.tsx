import { Route, Router } from "@solidjs/router";

import ErrorPage from "./views/error/Error";
import Profile from "./views/profile/Profile";
import Settings from "./views/settings/Settings";
import ErrorBoundary from "./components/ErrorBoundary";
import { onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export default function AppLoaded() {
  onMount(() => {
    invoke("bench_exit_interactive");
  });
  return (
    <ErrorBoundary>
      <Router>
        <Route path="/profile/:gameId?/:profileId?" component={Profile} />
        <Route path="/settings" component={Settings} />
        <Route path="*path" component={ErrorPage} />
      </Router>
    </ErrorBoundary>
  );
}
