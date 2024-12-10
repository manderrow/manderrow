import { createResource, createSignal, createUniqueId, For, Match, onMount, Show, Switch } from "solid-js";
import { A, useNavigate, useParams, useSearchParams } from "@solidjs/router";
import { faTrashCan, faCirclePlay as faCirclePlayOutline } from "@fortawesome/free-regular-svg-icons";
import { faChevronLeft, faCirclePlay, faFileImport, faPlus, faThumbTack } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";

import ModSearch from "../../components/profile/ModSearch";
import ModList from "../../components/profile/ModList";

import styles from "./Profile.module.css";
import sidebarStyles from "./SidebarProfiles.module.css";
import { gamesById } from "../../globals";
import { createProfile, deleteProfile, getProfiles, ProfileWithId } from "../../api";
import { Portal } from "solid-js/web";
import { Refetcher } from "../../types";
import Dialog from "../../components/Dialog";

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

  const [profiles, { refetch: refetchProfiles }] = createResource(async () => {
    const profiles = await getProfiles(params.gameId);
    profiles.sort((a, b) => a.name.localeCompare(b.name));
    return profiles;
  }, { initialValue: [] });

  const [activeProfileMods, { refetch: refetchActiveProfileMods }] = createResource(() => [], { initialValue: [] })

  return (
    <main class={styles.main}>
      <aside class={styles.sidebar}>
        <nav class={styles.sidebar__nav}>
          <A href="/" tabIndex="-1">
            <button class={styles.sidebar__btn}>
              <Fa icon={faChevronLeft} />
            </button>
          </A>

          <h1>{gameInfo.name}</h1>
        </nav>
        <section class={styles.sidebar__group}>
          <button>
            <Fa icon={faCirclePlay} /> Start modded
          </button>
          <button>
            <Fa icon={faCirclePlayOutline} /> Start vanilla
          </button>
        </section>
        <section class={styles.sidebar__group}>
          <h3 class={styles.sidebar__profilesTitle}>
            Profiles
            <A class={styles.sidebar__profilesAddBtn} href={`/profile/${params.gameId}`}>
              <Fa icon={faPlus} />
            </A>
          </h3>
          <form on:submit={e => e.preventDefault()} class={sidebarStyles.sidebar__profilesSearch}>
            <input type="text" name="profile-search" id="profile-search" placeholder="Search" maxLength={100} />
          </form>
          <ol class={sidebarStyles.sidebar__profilesList}>
            <For each={profiles()}>
              {(profile) => <SidebarProfileComponent gameId={params.gameId} profileId={profile.id} profileName={profile.name} refetchProfiles={refetchProfiles} selected={profile.id === params.profileId} />}
            </For>
          </ol>
        </section>
        <section class={styles.sidebar__group}>
          <h3>Other</h3>
        </section>
      </aside>

      <div class={styles.content}>
        <Show when={params.profileId !== undefined} fallback={<NoSelectedProfileContent gameId={params.gameId} profiles={profiles} refetchProfiles={refetchProfiles} />}>
          <ul class={styles.tabs}>
            <li classList={{ [styles.tabs__tab]: true, [styles.tab__active]: currentTab() === "mod-list" }}>
              <A href="">Installed</A>
            </li>
            <li classList={{ [styles.tabs__tab]: true, [styles.tab__active]: currentTab() === "mod-search" }}>
              <A href="?tab=mod-search">Online</A>
            </li>
          </ul>

          <Switch>
            <Match when={currentTab() === "mod-list"}>
              <Show when={activeProfileMods.latest.length !== 0} fallback={<p>Looks like you haven't installed any mods yet.</p>}>
                <ModList mods={async () => []} />
              </Show>
            </Match>
            <Match when={currentTab() === "mod-search"}>
              <ModSearch game={params.gameId} />
            </Match>
          </Switch>
        </Show>
      </div>
    </main>
  );
}

function NoSelectedProfileContent(props: { gameId: string, profiles: () => ProfileWithId[], refetchProfiles: Refetcher<ProfileWithId[]> }) {
  const [name, setName] = createSignal('');

  const navigator = useNavigate();

  async function submit(e: SubmitEvent) {
    e.preventDefault();

    const id = await createProfile(props.gameId, name());
    await props.refetchProfiles();
    navigator(`/profile/${props.gameId}/${id}`, { replace: true });
  }

  const nameId = createUniqueId();

  let inputRef: HTMLInputElement;

  onMount(() => {
    inputRef.focus();
  });

  return <>
    <p>{props.profiles().length !== 0 ? 'Select a profile from the sidebar or create a new one' : 'Create a new profile'}</p>
    <form on:submit={submit}>
      <label for={nameId}>Name</label>
      <input id={nameId} value={name()} on:input={e => setName(e.target.value)} ref={inputRef} />
      <button type="submit">Create</button>
    </form>
  </>;
}

function SidebarProfileComponent(props: { gameId: string; profileId: string; profileName: string, refetchProfiles: Refetcher<ProfileWithId[]>, selected: boolean }) {
  const [confirmingDeletion, setConfirmingDeletion] = createSignal(false);
  const [deleting, setDeleting] = createSignal(false);

  const navigator = useNavigate();

  return (
    <li class={sidebarStyles.profileList__item}>
      <A href={`/profile/${props.gameId}/${props.profileId}`}>{props.profileName}</A>
      <div class={sidebarStyles.profileItem__options}>
        <button data-import title="Import onto">
          <Fa icon={faFileImport} />
        </button>
        <button data-pin title="Pin">
          <Fa icon={faThumbTack} rotate={90} />
        </button>
        <button data-delete title="Delete" on:click={() => setConfirmingDeletion(true)}>
          <Fa icon={faTrashCan} />
        </button>
      </div>

      <Show when={confirmingDeletion()}>
        <Dialog>
          <p>You are about to delete the profile <strong>{props.profileId}</strong>.</p>
          <button disabled={deleting()} on:click={async () => {
            setDeleting(true);
            if (props.selected) {
              navigator(`/profile/${props.gameId}`, { replace: true });
            }
            try {
              await deleteProfile(props.gameId, props.profileId);
            } finally {
              setConfirmingDeletion(false);
              setDeleting(false);
              await props.refetchProfiles();
            }
          }}>Delete</button>
          <button disabled={deleting()} on:click={() => setConfirmingDeletion(false)}>Cancel</button>
        </Dialog>
      </Show>
    </li>
  );
}
