import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  createUniqueId,
  For,
  Match,
  Show,
  Switch,
  useContext,
} from "solid-js";
import { A, useNavigate, useParams } from "@solidjs/router";
import { faCirclePlay as faCirclePlayOutline, faTrashCan } from "@fortawesome/free-regular-svg-icons";
import {
  faChevronLeft,
  faCirclePlay,
  faDownload,
  faFileImport,
  faPenToSquare,
  faPlus,
  faThumbTack,
  faFileExport,
  faGear,
  faArrowUpWideShort,
  faArrowDownShortWide,
  faSkullCrossbones,
  faXmark,
  faThumbTackSlash,
} from "@fortawesome/free-solid-svg-icons";
import { OverlayScrollbarsComponent } from "overlayscrollbars-solid";
import { Fa } from "solid-fa";

import Console, { DoctorReports } from "../../components/global/Console";
import { PromptDialog } from "../../components/global/Dialog";
import { ErrorDialog } from "../../components/global/ErrorBoundary";
import SelectDropdown from "../../components/global/SelectDropdown";
import TabRenderer from "../../components/global/TabRenderer";
import ModList, { ModInstallContext } from "../../components/profile/ModList";
import ModSearch from "../../components/profile/ModSearch";

import { createProfile, deleteProfile, getProfileMods, overwriteProfileMetadata, ProfileWithId } from "../../api";
import * as globals from "../../globals";
import { refetchProfiles } from "../../globals";
import { Refetcher } from "../../types.d.ts";
import { autofocus, bindValue } from "../../components/global/Directives";

import styles from "./Profile.module.css";
import sidebarStyles from "./SidebarProfiles.module.css";
import ImportDialog from "../../components/profile/ImportDialog";
import TasksDialog from "../../components/global/TasksDialog";
import { settings } from "../../api/settings";
import { useSearchParamsInPlace } from "../../utils/router";
import { killIpcClient } from "../../api/ipc";
import { t } from "../../i18n/i18n.ts";
import { launchProfile } from "../../api/launching";
import { ConsoleConnection, focusedConnection, setFocusedConnection } from "../../console";
import { ActionContext } from "../../components/global/AsyncButton.tsx";

interface ProfileParams {
  profileId?: string;
  gameId: string;
}

type TabId = "mod-list" | "mod-search" | "logs" | "config";

interface ProfileSearchParams {
  "profile-tab"?: TabId;
}

export default function Profile() {
  // @ts-expect-error params.profileId is an optional param, it can be undefined, and we don't expect any other params
  const params = useParams<ProfileParams>();
  const [searchParams, setSearchParams] = useSearchParamsInPlace<ProfileSearchParams>();
  const navigate = useNavigate();

  const gameInfo = globals.gamesById().get(params.gameId)!; // TODO, handle undefined case

  const [profileSortOrder, setProfileSortOrder] = createSignal(false);

  const [profiles] = createResource(
    globals.profiles,
    (profiles) => {
      return Object.fromEntries(
        profiles.filter((profile) => profile.game === params.gameId).map((profile) => [profile.id, profile]),
      );
    },
    { initialValue: {} },
  );

  const currentProfile = createMemo(() => {
    if (params.profileId === undefined) return undefined;
    const profile = profiles()[params.profileId];
    if (profile !== undefined) return profile;
    throw new Error(`Unknown profile ${params.profileId}`);
  });

  createEffect(() => {
    const game = currentProfile()?.game;
    if (game !== undefined && game !== params.gameId) {
      throw new Error(`Profile ${params.profileId} is for ${game}, not ${params.gameId}`);
    }
  });

  // track launch errors here instead of reporting to the error boundary to avoid rebuilding the UI
  const [err, setErr] = createSignal<unknown>();

  async function launch(modded: boolean) {
    try {
      const conn = await ConsoleConnection.allocate();
      try {
        setFocusedConnection(conn);
        console.log(focusedConnection());
        if (settings().openConsoleOnLaunch.value && searchParams["profile-tab"] !== "logs") {
          setSearchParams({ "profile-tab": "logs" });
        }
        await launchProfile(
          conn.id,
          params.profileId !== undefined ? { profile: params.profileId } : { vanilla: params.gameId },
          { modded },
        );
      } catch (error) {
        conn.handleEvent({
          type: "Error",
          error,
        });
        setErr(error);
      }
    } catch (e) {
      setErr(e);
    }
  }

  async function killGame() {
    const conn = focusedConnection();
    if (conn !== undefined) {
      try {
        await killIpcClient(conn.id);
      } catch (error) {
        conn.handleEvent({
          type: "Error",
          error,
        });
        setErr(error);
      }
    }
  }

  const [importDialogOpen, setImportDialogOpen] = createSignal(false);

  const [tasksDialogOpen, setTasksDialogOpen] = createSignal(false);

  const hasLiveConnection = () => focusedConnection() !== undefined && focusedConnection()?.status() !== "disconnected";

  return (
    <main class={styles.main}>
      <aside class={styles.sidebar}>
        <nav class={styles.sidebar__nav}>
          <button class={styles.sidebar__btn} on:click={() => navigate(-1)}>
            <Fa icon={faChevronLeft} />
          </button>

          <h1>{gameInfo.name}</h1>
        </nav>
        <section classList={{ [styles.sidebar__group]: true, [styles.sidebar__mainActions]: true }}>
          <Switch>
            <Match when={focusedConnection()?.status() === "connected"}>
              <button on:click={() => killGame()} data-kill>
                <Fa icon={faSkullCrossbones} /> {t("profile.sidebar.kill_game_btn")}
              </button>
            </Match>
            <Match when={hasLiveConnection()}>
              <button on:click={() => setFocusedConnection(undefined)} data-cancel>
                <Fa icon={faXmark} /> {t("profile.sidebar.cancel_launch_btn")}
              </button>
            </Match>
          </Switch>
          {
            // TODO: based on hasLiveConnection change the UI of these a bit
          }
          <button disabled={params.profileId === undefined} on:click={() => launch(true)} data-modded>
            <Fa icon={faCirclePlay} /> {t("profile.sidebar.launch_modded_btn")}
          </button>
          <button on:click={() => launch(false)} data-vanilla>
            <Fa icon={faCirclePlayOutline} /> {t("profile.sidebar.launch_vanilla_btn")}
          </button>
        </section>
        <section classList={{ [styles.sidebar__group]: true, [sidebarStyles.sidebar__profiles]: true }}>
          <h3 class={styles.sidebar__profilesTitle}>
            {t("profile.sidebar.profiles_title")}
            <div class={styles.sidebar__profilesActions}>
              <A class={styles.sidebar__profilesActionBtn} href={`/profile/${params.gameId}`}>
                <Fa icon={faPlus} />
              </A>
              <button
                class={styles.sidebar__profilesActionBtn}
                title="Import"
                on:click={() => setImportDialogOpen(true)}
              >
                <Fa icon={faFileImport} class={sidebarStyles.sidebar__profileActionsBtnIcon} />
              </button>
            </div>
          </h3>

          <form on:submit={(e) => e.preventDefault()} class={sidebarStyles.sidebar__profilesSearch}>
            <input type="text" name="profile-search" id="profile-search" placeholder="Search" maxLength={100} />
            <SelectDropdown<"alphabetical" | "creationDate">
              class={sidebarStyles.sidebar__profilesSearchSortBtn}
              multiselect={false}
              options={{
                "A-Z": {
                  value: "alphabetical",
                },

                "Creation Date": {
                  value: "creationDate",
                },
              }}
              label={{ labelText: "preset", preset: "Sort" }}
              onChanged={(key, selected) => console.log(key, selected)}
            />
            <button
              class={sidebarStyles.sidebar__profilesSearchSortByBtn}
              on:click={() => setProfileSortOrder((order) => !order)}
            >
              {profileSortOrder() ? <Fa icon={faArrowUpWideShort} /> : <Fa icon={faArrowDownShortWide} />}
            </button>
          </form>

          <OverlayScrollbarsComponent
            defer
            options={{ scrollbars: { autoHide: "leave" } }}
            class={sidebarStyles.sidebar__profilesListContainer}
          >
            <ol class={sidebarStyles.sidebar__profilesList}>
              <For each={Object.keys(profiles())}>
                {(id) => (
                  <Show when={profiles()[id].pinned}>
                    <SidebarProfileComponent
                      gameId={params.gameId}
                      profile={profiles()[id]}
                      refetchProfiles={refetchProfiles}
                      selected={id === params.profileId}
                    />
                  </Show>
                )}
              </For>
              <For each={Object.keys(profiles())}>
                {(id) => (
                  <Show when={!profiles()[id].pinned}>
                    <SidebarProfileComponent
                      gameId={params.gameId}
                      profile={profiles()[id]}
                      refetchProfiles={refetchProfiles}
                      selected={id === params.profileId}
                    />
                  </Show>
                )}
              </For>
            </ol>
          </OverlayScrollbarsComponent>
        </section>
        <section class={styles.sidebar__group}>
          <div class={styles.sidebar__otherGrid}>
            <A href="/settings">
              <button>
                <Fa icon={faGear} class={styles.sidebar__otherGridIcon} />
                <br />
                Settings
              </button>
            </A>
            <button on:click={() => setTasksDialogOpen(true)}>
              <Fa icon={faDownload} class={styles.sidebar__otherGridIcon} />
              <br />
              Downloads
            </button>
          </div>
        </section>
      </aside>

      <div class={styles.content}>
        <Show
          when={params.profileId}
          fallback={
            <NoSelectedProfileContent gameId={params.gameId} profiles={profiles} refetchProfiles={refetchProfiles} />
          }
        >
          {(profileId) => {
            const [installed, { refetch: refetchInstalled0 }] = createResource(
              profileId,
              (profileId) => getProfileMods(profileId),
              { initialValue: [] },
            );

            const refetchInstalled = async () => {
              await refetchInstalled0();
            };

            return (
              <ModInstallContext.Provider value={{ profileId, installed, refetchInstalled }}>
                <TabRenderer<TabId>
                  id="profile"
                  styles={{
                    tabs: {
                      container: styles.tabs,
                      list: styles.tabs__list,
                      list__item: styles.tabs__tab,
                      list__itemActive: styles.tab__active,
                    },
                  }}
                  tabs={[
                    {
                      id: "mod-list",
                      name: "Installed",
                      component: <InstalledModsList game={params.gameId} />,
                    },

                    {
                      id: "mod-search",
                      name: "Online",
                      component: <ModSearch game={params.gameId} />,
                    },

                    {
                      id: "logs",
                      name: "Logs",
                      component: (
                        <div class={styles.content__console}>
                          <Console />
                        </div>
                      ),
                    },

                    {
                      id: "config",
                      name: "Config",
                      component: <div></div>,
                    },
                  ]}
                />
              </ModInstallContext.Provider>
            );
          }}
        </Show>
      </div>

      <Show when={importDialogOpen()}>
        <ImportDialog dismiss={() => setImportDialogOpen(false)} gameId={params.gameId} profile={params.profileId} />
      </Show>

      <Show when={tasksDialogOpen()}>
        <TasksDialog onDismiss={() => setTasksDialogOpen(false)} />
      </Show>

      <DoctorReports />

      <Show when={err()}>{(err) => <ErrorDialog err={err()} reset={() => setErr(undefined)} />}</Show>
    </main>
  );
}

function NoSelectedProfileContent(props: {
  gameId: string;
  profiles: () => { [id: string]: ProfileWithId };
  refetchProfiles: Refetcher<ProfileWithId[]>;
}) {
  const [name, setName] = createSignal("");

  const navigate = useNavigate();

  async function submit(e: SubmitEvent) {
    e.preventDefault();

    const id = await createProfile(props.gameId, name());
    await props.refetchProfiles();
    navigate(`/profile/${props.gameId}/${id}`, { replace: true });
  }

  const nameId = createUniqueId();

  return (
    <>
      <p>
        {Object.keys(props.profiles()).length !== 0
          ? "Select a profile from the sidebar or create a new one"
          : "Create a new profile"}
      </p>
      <form on:submit={submit}>
        <label for={nameId}>Name</label>
        <input id={nameId} value={name()} on:input={(e) => setName(e.target.value)} use:autofocus />
        <button type="submit">Create</button>
      </form>
    </>
  );
}

function InstalledModsList(props: { game: string }) {
  const context = useContext(ModInstallContext)!;

  return (
    <Show when={context.installed.latest.length !== 0} fallback={<p>Looks like you haven't installed any mods yet.</p>}>
      <ModList
        // kinda gross
        mods={(() => {
          const data = context.installed();
          return async (page) => (page === 0 ? data : []);
        })()}
      />
    </Show>
  );
}

function SidebarProfileComponent(props: {
  gameId: string;
  profile: ProfileWithId;
  refetchProfiles: Refetcher<ProfileWithId[]>;
  selected: boolean;
}) {
  const [confirmingDeletion, setConfirmingDeletion] = createSignal(false);
  const [deleting, setDeleting] = createSignal(false);

  const navigate = useNavigate();

  const [renaming, setRenaming] = createSignal(false);

  return (
    <li class={sidebarStyles.profileList__item}>
      <Show
        when={renaming()}
        fallback={
          <>
            <A class={sidebarStyles.profileList__itemName} href={`/profile/${props.gameId}/${props.profile.id}`}>
              {props.profile.name}
            </A>
            <div class={sidebarStyles.profileItem__options}>
              <ActionContext>
                {(busy, wrapAction) => (
                  <button
                    data-pin
                    title={t("profile.sidebar.pin_profile_ptn")}
                    disabled={busy()}
                    on:click={async (e) => {
                      e.stopPropagation();
                      await wrapAction(async () => {
                        try {
                          const obj: ProfileWithId = { ...props.profile };
                          const id = obj.id;
                          // @ts-ignore I want to remove the property
                          delete obj.id;
                          obj.pinned = !obj.pinned;
                          await overwriteProfileMetadata(id, obj);
                        } finally {
                          await props.refetchProfiles();
                        }
                      });
                    }}
                  >
                    <Fa icon={props.profile.pinned ? faThumbTackSlash : faThumbTack} rotate={90} />
                  </button>
                )}
              </ActionContext>
              <button data-pin title={t("profile.sidebar.rename_profile_btn")} on:click={() => setRenaming(true)}>
                <Fa icon={faPenToSquare} />
              </button>
              <button
                data-delete
                title={t("profile.sidebar.delete_profile_btn")}
                on:click={() => setConfirmingDeletion(true)}
              >
                <Fa icon={faTrashCan} />
              </button>
              <button data-export title={t("profile.sidebar.export_profile_btn")}>
                <Fa icon={faFileExport} />
              </button>
            </div>

            <Show when={confirmingDeletion()}>
              <PromptDialog
                options={{
                  title: "Confirm",
                  question: `You are about to delete ${props.profile.name}`,
                  btns: {
                    ok: {
                      type: "danger",
                      text: "Delete",
                      async callback() {
                        if (props.selected) {
                          navigate(`/profile/${props.gameId}`, { replace: true });
                        }
                        if (deleting()) return;
                        setDeleting(true);
                        try {
                          await deleteProfile(props.profile.id);
                        } finally {
                          setConfirmingDeletion(false);
                          setDeleting(false);
                          await props.refetchProfiles();
                        }
                      },
                    },
                    cancel: {
                      callback() {
                        setConfirmingDeletion(false);
                      },
                    },
                  },
                }}
              />
            </Show>
          </>
        }
      >
        <ActionContext>
          {(busy, wrapAction) => (
            <form
              class={sidebarStyles.profileList__itemName}
              on:submit={async (e) => {
                e.preventDefault();
                await wrapAction(async () => {
                  try {
                    const obj: ProfileWithId = { ...props.profile };
                    const id = obj.id;
                    // @ts-ignore I want to remove the property
                    delete obj.id;
                    obj.name = (e.target.firstChild as HTMLInputElement).value;
                    await overwriteProfileMetadata(id, obj);
                  } finally {
                    await props.refetchProfiles();
                  }
                });
              }}
            >
              <input
                value={props.profile.name}
                disabled={busy()}
                use:autofocus
                on:focus={(e) => {
                  const target = e.target as HTMLInputElement;
                  target.setSelectionRange(0, target.value.length);
                }}
                on:focusout={() => setRenaming(false)}
                on:keydown={(e) => {
                  if (e.key === "Escape") {
                    setRenaming(false);
                  }
                }}
              />
            </form>
          )}
        </ActionContext>
      </Show>
    </li>
  );
}
