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
        <nav class={styles.sidebar__nav}>
          <A href="/">
            <button class={styles.sidebar__btn}>Back</button>
          </A>

          <h1>{params.gameId}</h1>
        </nav>
        <hr />
        <section class={styles.sidebar__group}>
          <button>Start modded</button>
          <button>Start vanilla</button>
        </section>
        <hr />
        <section class={styles.sidebar__group}>
          <h3>Profiles</h3>
          <form action="#">
            <input type="text" name="profile-search" id="profile-search" placeholder="Search" maxLength={100} />
          </form>
          <ol class={styles.sidebar__profilesList}>
            <li class={styles.profileList__item}>
              <A href="../base">Base</A>
              <div class={styles.profileItem__options}>
                <button>1</button>
                <button>2</button>
                <button>3</button>
              </div>
            </li>
          </ol>
        </section>
        <section class={styles.sidebar__group}>
          <h3>Other</h3>
        </section>
      </aside>

      <div class={styles.content}>
        <h2 class={styles.profileTitle}>{params.profileId}</h2>
        <ul class={styles.tabs}>
          <li class={styles.tabs__tab}>
            <A href="?">Installed</A>
          </li>
          <li class={styles.tabs__tab}>
            <A href="?tab=mod-search">Online</A>
          </li>
        </ul>
        <div class={styles.content__substance}>
          <Switch>
            <Match when={currentTab() === "mod-list"} children={<ModList />} />
            <Match when={currentTab() === "mod-search"} children={<ModSearch />} />
          </Switch>
        </div>
      </div>
    </main>
  );
}
