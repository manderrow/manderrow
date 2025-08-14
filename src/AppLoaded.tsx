import { Route, Router } from "@solidjs/router";

import ErrorPage from "./pages/error/Error";
import Profile from "./pages/profile/Profile";
import Settings from "./pages/settings/Settings";
import ErrorBoundary from "./components/global/ErrorBoundary";

export default function AppLoaded() {
  return <ErrorBoundary>
    <Router>
      <Route path="/profile/:gameId?/:profileId?" component={Profile} />
      <Route path="/settings" component={Settings} />
      <Route path="*path" component={ErrorPage} />
    </Router>
  </ErrorBoundary>;
}
