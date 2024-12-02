import { createMemo, For, Match, Switch } from "solid-js";
import { A, useParams, useSearchParams } from "@solidjs/router";
import { faTrashCan, faCirclePlay as faCirclePlayOutline } from "@fortawesome/free-regular-svg-icons";
import { faChevronLeft, faCirclePlay, faFileImport, faThumbTack } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";

import ModSearch from "../../components/profile/ModSearch";
import ModList from "../../components/profile/ModList";

import styles from "./Profile.module.css";
import sidebarStyles from "./SidebarProfiles.module.css";
import { gamesById } from "../../globals";

interface ProfileParams {
  [key: string]: string | undefined;
  profileId?: string;
  gameId: string;
}

interface ProfileQueryParams {
  [key: string]: string | string[];
  tab: string;
}

export default function Profile() {
  // @ts-expect-error params.profileId is an optional param, it can be undefined
  const params = useParams<ProfileParams>();
  const [searchParams] = useSearchParams<ProfileQueryParams>();

  const currentTab = () => searchParams.tab ?? "mod-list";
  const gameInfo = gamesById().get(params.gameId)!; // TODO, handle undefined case

  return (
    <main class={styles.main}>
      <aside class={styles.sidebar}>
        <nav class={styles.sidebar__nav}>
          <A href="/">
            <button class={styles.sidebar__btn}>
              <Fa icon={faChevronLeft} />
            </button>
          </A>

          <h1>{gameInfo.name}</h1>
        </nav>
        <section class={styles.sidebar__group}>
          <h2 class={styles.profileTitle}>{params.profileId}</h2>
          <button>
            <Fa icon={faCirclePlay} /> Start modded
          </button>
          <button>
            <Fa icon={faCirclePlayOutline} /> Start vanilla
          </button>
        </section>
        <section class={styles.sidebar__group}>
          <h3>Profiles</h3>
          <form action="#" class={sidebarStyles.sidebar__profilesSearch}>
            <input type="text" name="profile-search" id="profile-search" placeholder="Search" maxLength={100} />
          </form>
          <ol class={sidebarStyles.sidebar__profilesList}>
            <For
              each={[
                { id: "a", name: "Test profile" },
                { id: "b", name: "Another profile" },
              ]}
            >
              {(profile) => <SidebarProfileComponent gameId={params.gameId} profileId={profile.id} profileName={profile.name} />}
            </For>
          </ol>
        </section>
        <section class={styles.sidebar__group}>
          <h3>Other</h3>
        </section>
      </aside>

      <div class={styles.content}>
        <ul class={styles.tabs}>
          <li class={styles.tabs__tab}>
            <A href="?">Installed</A>
          </li>
          <li class={styles.tabs__tab}>
            <A href="?tab=mod-search">Online</A>
          </li>
        </ul>

        <Switch>
          <Match when={currentTab() === "mod-list"} children={<ModList mods={async () => []} />} />
          <Match when={currentTab() === "mod-search"} children={<ModSearch game={params.gameId} />} />
        </Switch>
      </div>
    </main>
  );
}

function SidebarProfileComponent({ gameId, profileId, profileName }: { gameId: string; profileId: string; profileName: string }) {
  return (
    <li class={sidebarStyles.profileList__item}>
      <A href={`/profile/${gameId}/${profileId}`}>{profileName}</A>
      <div class={sidebarStyles.profileItem__options}>
        <button>
          <Fa icon={faFileImport} />
        </button>
        <button>
          <Fa icon={faThumbTack} rotate={90} />
        </button>
        <button>
          <Fa icon={faTrashCan} />
        </button>
      </div>
    </li>
  );
}
