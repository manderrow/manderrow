import { createResource, createSignal, createUniqueId, For, onMount, Show, useContext } from "solid-js";
import { A, useNavigate, useParams } from "@solidjs/router";
import { faTrashCan, faCirclePlay as faCirclePlayOutline } from "@fortawesome/free-regular-svg-icons";
import { faChevronLeft, faCirclePlay, faFileImport, faPlus, faThumbTack } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";

import ModSearch from "../../components/profile/ModSearch";
import ModList from "../../components/profile/ModList";

import styles from "./Profile.module.css";
import sidebarStyles from "./SidebarProfiles.module.css";
import * as globals from "../../globals";
import { gamesById, refetchProfiles } from "../../globals";
import { createProfile, deleteProfile, launchProfile, ProfileWithId } from "../../api";
import { Refetcher } from "../../types";
import Dialog from "../../components/global/Dialog";
import { ErrorContext } from "../../components/global/ErrorBoundary";
import Console, { C2SChannel, clearConsole, createC2SChannel } from "../../components/global/Console";
import TabRenderer from "../../components/global/TabRenderer";

interface ProfileParams {
  [key: string]: string | undefined;
  profileId?: string;
  gameId: string;
}

export default function Profile() {
  // @ts-expect-error params.profileId is an optional param, it can be undefined
  const params = useParams<ProfileParams>();

  const gameInfo = gamesById().get(params.gameId)!; // TODO, handle undefined case

  const [profiles] = createResource(
    globals.profiles,
    (profiles) => {
      return profiles.filter((profile) => profile.game === params.gameId);
    },
    { initialValue: [] }
  );

  const [activeProfileMods, { refetch: refetchActiveProfileMods }] = createResource(() => [], { initialValue: [] });

  const reportErr = useContext(ErrorContext)!;

  const [consoleChannel, setConsoleChannel] = createSignal<C2SChannel>();

  async function launch(modded: boolean) {
    try {
      clearConsole();
      const channel = createC2SChannel();
      setConsoleChannel(channel);
      await launchProfile(params.profileId!, channel, { modded });
    } catch (e) {
      reportErr(e);
    }
  }

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
          <button disabled={params.profileId === undefined} on:click={() => launch(true)}>
            <Fa icon={faCirclePlay} /> Start modded
          </button>
          <button disabled={params.profileId === undefined} on:click={() => launch(false)}>
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
          <form on:submit={(e) => e.preventDefault()} class={sidebarStyles.sidebar__profilesSearch}>
            <input type="text" name="profile-search" id="profile-search" placeholder="Search" maxLength={100} />
          </form>
          <ol class={sidebarStyles.sidebar__profilesList}>
            <For each={profiles()}>
              {(profile) => (
                <SidebarProfileComponent
                  gameId={params.gameId}
                  profileId={profile.id}
                  profileName={profile.name}
                  refetchProfiles={refetchProfiles}
                  selected={profile.id === params.profileId}
                />
              )}
            </For>
          </ol>
        </section>
        <section class={styles.sidebar__group}>
          <h3>Other</h3>
        </section>
      </aside>

      <div class={styles.content}>
        <Show
          when={params.profileId !== undefined}
          fallback={<NoSelectedProfileContent gameId={params.gameId} profiles={profiles} refetchProfiles={refetchProfiles} />}
        >
          <TabRenderer
            styles={{ tabs: { list: styles.tabs, list__item: styles.tabs__tab, list__itemActive: styles.tab__active } }}
            tabs={[
              {
                id: "mod-list",
                name: "Installed",
                component: (
                  <Show when={activeProfileMods.latest.length !== 0} fallback={<p>No mods installed yet.</p>}>
                    <ModList mods={async () => activeProfileMods.latest} />
                  </Show>
                ),
              },

              {
                id: "mod-search",
                name: "Online",
                component: <ModSearch game={params.gameId} />,
              },

              {
                id: "console",
                name: "Console",
                component: (
                  <div class={styles.content__console}>
                    <h2 class={styles.content__consoleHeading}>Log Output</h2>
                    <Console channel={consoleChannel} />
                  </div>
                ),
              },
            ]}
          />
        </Show>
      </div>
    </main>
  );
}

function NoSelectedProfileContent(props: { gameId: string; profiles: () => ProfileWithId[]; refetchProfiles: Refetcher<ProfileWithId[]> }) {
  const [name, setName] = createSignal("");

  const navigator = useNavigate();

  async function submit(e: SubmitEvent) {
    e.preventDefault();

    const id = await createProfile(props.gameId, name());
    await props.refetchProfiles();
    navigator(`/profile/${props.gameId}/${id}`, { replace: true });
  }

  const nameId = createUniqueId();

  let inputRef!: HTMLInputElement;

  onMount(() => {
    inputRef.focus();
  });

  return (
    <>
      <p>{props.profiles().length !== 0 ? "Select a profile from the sidebar or create a new one" : "Create a new profile"}</p>
      <form on:submit={submit}>
        <label for={nameId}>Name</label>
        <input id={nameId} value={name()} on:input={(e) => setName(e.target.value)} ref={inputRef} />
        <button type="submit">Create</button>
      </form>
    </>
  );
}

function SidebarProfileComponent(props: {
  gameId: string;
  profileId: string;
  profileName: string;
  refetchProfiles: Refetcher<ProfileWithId[]>;
  selected: boolean;
}) {
  const [confirmingDeletion, setConfirmingDeletion] = createSignal(false);
  const [deleting, setDeleting] = createSignal(false);

  const navigate = useNavigate();

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
          <p>
            You are about to delete the profile <strong>{props.profileName}</strong>.
          </p>
          <button
            disabled={deleting()}
            on:click={async () => {
              setDeleting(true);
              if (props.selected) {
                navigate(`/profile/${props.gameId}`, { replace: true });
              }
              try {
                await deleteProfile(props.profileId);
              } finally {
                setConfirmingDeletion(false);
                setDeleting(false);
                await props.refetchProfiles();
              }
            }}
          >
            Delete
          </button>
          <button disabled={deleting()} on:click={() => setConfirmingDeletion(false)}>
            Cancel
          </button>
        </Dialog>
      </Show>
    </li>
  );
}
