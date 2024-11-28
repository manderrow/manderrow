import { Route, Router } from "@solidjs/router";

import "./App.css";

import GameSelect from "./pages/game_select/GameSelect";
import Profile from "./pages/profile/Profile";
import Error from "./pages/error/Error";

function App() {
  return (
    <Router>
      <Route path="/" component={GameSelect} />
      <Route path="/profile/:gameId/:profileId" component={Profile} />
      <Route path="*404" component={Error} />
    </Router>
  );
}

export default App;
