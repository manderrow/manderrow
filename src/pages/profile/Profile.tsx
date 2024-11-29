import { A, useParams, useSearchParams } from "@solidjs/router";
import ModSearch from "../../components/profile/ModSearch";

import styles from "./Profile.module.css";
import { Match, Switch } from "solid-js";
import ModList from "../../components/profile/ModList";

interface ProfileParams {
  [key: string]: string;
  profileId: string;
  gameId: string;
}

interface ProfileQueryParams {
  [key: string]: string | string[];
  tab: string;
}

export default function Profile() {
  const params = useParams<ProfileParams>();
  const [searchParams] = useSearchParams<ProfileQueryParams>();

  const currentTab = () => searchParams.tab ?? "mod-list";

  return (
    <main class={styles.main}>
      <aside class={styles.sidebar}>
        <nav>
          <h1>{params.gameId}</h1>
          <button>Back</button>
        </nav>
        <hr />
        <section class={styles.sidebar__group}>
          <button>Start modded</button>
          <button>Start vanilla</button>
        </section>
        <hr />
        <section class={styles.sidebar__group}>
          <form action="#">
            <h3>Profiles</h3>
            <input type="text" name="profile-search" id="profile-search" placeholder="Search" maxLength={100} />
          </form>
          <ol class={styles.sidebar__profilesList}>
            <li class={styles.profileList__item}>
              <A href="../base">Base</A>
            </li>
            <li class={styles.profileList__item}>
              <A href="../cheats">Cheats</A>
            </li>
          </ol>
        </section>
      </aside>

      <div>
        <h2>{params.profileId}</h2>
        <ul class={styles.tabs}>
          <li class={styles.tabs__tab}>
            <A href="?tab=mod-list">Installed</A>
          </li>
          <li class={styles.tabs__tab}>
            <A href="?tab=mod-search">Online</A>
          </li>
        </ul>
        <Switch>
          <Match when={currentTab() === "mod-list"} children={<ModList />} />
          <Match when={currentTab() === "mod-search"} children={<ModSearch />} />
        </Switch>
      </div>
    </main>
  );
}
