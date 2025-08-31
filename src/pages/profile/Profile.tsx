import {
  createEffect,
  createMemo,
  createResource,
  createSelector,
  createSignal,
  createUniqueId,
  For,
  Match,
  onCleanup,
  Show,
  Switch,
} from "solid-js";
import { Portal } from "solid-js/web";
import { A, useNavigate, useParams } from "@solidjs/router";
import { faTrashCan } from "@fortawesome/free-regular-svg-icons";
import {
  faCirclePlay,
  faFileImport,
  faPenToSquare,
  faPlus,
  faThumbTack,
  faGear,
  faArrowUpWideShort,
  faArrowDownShortWide,
  faSkullCrossbones,
  faXmark,
  faThumbTackSlash,
  faAnglesRight,
  faCopy,
  faClipboard,
  faFolderOpen,
  faShare,
  faEllipsisVertical,
  faChevronDown,
  IconDefinition,
  faArrowRightLong,
} from "@fortawesome/free-solid-svg-icons";
import { OverlayScrollbarsComponent } from "overlayscrollbars-solid";
import { Fa } from "solid-fa";

import Console, { DoctorReports } from "../../components/global/Console";
import { PromptDialog } from "../../components/global/Dialog";
import ErrorDialog from "../../components/global/ErrorDialog.tsx";
import SelectDropdown from "../../components/global/SelectDropdown";
import TabRenderer from "../../components/global/TabRenderer";
import { InstalledModList, ModInstallContext, OnlineModList } from "../../components/profile/ModList";

import { createProfile, deleteProfile, getProfileMods, overwriteProfileMetadata, ProfileWithId } from "../../api";
import * as globals from "../../globals";
import { initialGame, refetchProfiles, shifting, ctrling } from "../../globals";
// @ts-ignore: TS is unaware of `use:` directives despite using them for type definitions
import { autofocus, bindValue } from "../../components/global/Directives";

import styles from "./Profile.module.css";
import sidebarStyles from "./SidebarProfiles.module.css";
import ImportDialog from "../../components/profile/ImportDialog";
import { settings } from "../../api/settings";
import { useSearchParamsInPlace } from "../../utils/router";
import { killIpcClient } from "../../api/ipc";
import { t } from "../../i18n/i18n.ts";
import { launchProfile } from "../../api/launching";
import { ConsoleConnection, focusedConnection, setFocusedConnection } from "../../console";
import { ActionContext } from "../../components/global/AsyncButton.tsx";
import { setCurrentProfileName } from "../../components/global/TitleBar.tsx";
import Tooltip from "../../components/global/Tooltip.tsx";
import ContextMenu from "../../components/global/ContextMenu.tsx";
import GameSelect from "../../components/profile/game_select/GameSelect.tsx";
import StatusBar from "../../components/profile/StatusBar.tsx";

interface ProfileParams {
  profileId?: string;
  gameId?: string;
}

type TabId = "mod-list" | "mod-search" | "logs" | "config";

interface ProfileSearchParams {
  "profile-tab"?: TabId;
}

enum ProfileSortType {
  alphabetical = "alphabetical",
  creation_date = "creation_date",
}

export default function Profile() {
  // @ts-expect-error params.profileId is an optional param, it can be undefined, and we don't expect any other params
  const params = useParams<ProfileParams>();

  const navigate = useNavigate();

  createEffect(() => {
    const game = initialGame.latestOrThrow;
    if (game) {
      navigate(`/profile/${game}`, { replace: true });
    }
  });

  return (
    <Show when={params.gameId} fallback={<GameSelect replace={true} />}>
      {(gameId) => <ProfileWithGame gameId={gameId()} profileId={params.profileId} />}
    </Show>
  );
}

function ProfileWithGame(params: ProfileParams & { gameId: string }) {
  const [searchParams, setSearchParams] = useSearchParamsInPlace<ProfileSearchParams>();

  // TODO, handle undefined case
  const gameInfo = createMemo(() => globals.gamesById().get(params.gameId)!);

  const [profiles] = createResource(
    () => {
      // TODO: catch error and handle correctly
      const profiles = globals.profiles();
      if (profiles == null) return undefined;
      return { profiles, gameId: params.gameId };
    },
    ({ profiles, gameId }) => {
      return Object.fromEntries(
        profiles.filter((profile) => profile.game === gameId).map((profile) => [profile.id, profile]),
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

  // Update title bar with current profile name
  createEffect(() => {
    const gameId = params.gameId;
    const profile = currentProfile();
    setCurrentProfileName(profile == null || gameId !== profile.game ? "" : profile.name);
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

  const hasLiveConnection = () => focusedConnection() !== undefined && focusedConnection()?.status() !== "disconnected";

  // For mini sidebar on small app width only
  const [sidebarOpen, setSidebarOpen] = createSignal(false);

  const [pickingGame, setPickingGame] = createSignal(false);

  // For creating new profiles
  const [creatingProfile, setCreatingProfile] = createSignal(false);

  // For searching profiles
  const [profileQuery, setProfileQuery] = createSignal("");
  const [profileSortOrder, setProfileSortOrder] = createSignal(false); // true for ascending, false for descending
  const [profileSortType, setProfileSortType] = createSignal<ProfileSortType>(ProfileSortType.creation_date);
  const isProfileSortType = createSelector(profileSortType);

  const queriedProfiles = () => {
    const query = profileQuery().toLowerCase();
    return Object.values(profiles())
      .filter((profile) => profile.name.toLowerCase().includes(query))
      .sort((a, b) => {
        switch (profileSortType()) {
          case ProfileSortType.alphabetical:
          case ProfileSortType.creation_date:
            return profileSortOrder() ? a.name.localeCompare(b.name) : b.name.localeCompare(a.name);
          // TODO:
          // case ProfileSortType.creation_date:
          //   return profileSortOrder() ? a.createdAt - b.createdAt : b.createdAt - a.createdAt;
        }
      });
  };

  // For selecting multiple profiles
  const [pivotProfile, setPivotProfile] = createSignal<string | undefined>();
  const effectivePivot = () => pivotProfile() || currentProfile()?.id;
  const [selectedProfiles, setSelectedProfiles] = createSignal<string[]>([]);

  function onCtrlClickProfile(profile: ProfileWithId) {
    setSelectedProfiles((prev) => {
      if (prev.includes(profile.id)) {
        return prev.filter((id) => id !== profile.id);
      } else {
        return [...prev, profile.id];
      }
    });

    setPivotProfile(profile.id);
  }
  function onShiftClickProfile(profile: ProfileWithId) {
    setSelectedProfiles((prev) => {
      const pivot = effectivePivot();

      if (pivot == null || profile.id === pivot) return prev;

      const profilesList = queriedProfiles();
      const pivotIndex = profilesList.findIndex((p) => p.id === pivot);
      const currentIndex = profilesList.findIndex((p) => p.id === profile.id);
      const start = Math.min(pivotIndex, currentIndex);
      const end = Math.max(pivotIndex, currentIndex);
      return profilesList.slice(start, end + 1).map((p) => p.id);
    });
  }

  return (
    <main class={styles.main}>
      <aside class={styles.sidebar} data-sidebar-open={sidebarOpen()}>
        <section
          classList={{ [styles.sidebar__group]: true, [styles.sidebar__mainActions]: true }}
          style={{ "--game-hero--url": `url("/img/game_heros/${gameInfo().id}.webp")` }}
        >
          <nav class={styles.sidebar__nav}>
            <button class={styles.backBtn} on:click={() => setPickingGame(true)}>
              <Fa icon={faChevronDown} />
            </button>

            <h1>{gameInfo().name}</h1>
          </nav>
          <div class={styles.sidebar__mainActionBtns}>
            <Switch>
              <Match when={focusedConnection()?.status() === "connected"}>
                <button on:click={() => killGame()} data-kill>
                  <Fa icon={faSkullCrossbones} /> <span>{t("profile.sidebar.kill_game_btn")}</span>
                </button>
              </Match>
              <Match when={hasLiveConnection()}>
                <button on:click={() => setFocusedConnection(undefined)} data-cancel>
                  <Fa icon={faXmark} /> <span>{t("profile.sidebar.cancel_launch_btn")}</span>
                </button>
              </Match>
            </Switch>
            {
              // TODO: based on hasLiveConnection change the UI of these a bit
            }
            <button disabled={params.profileId === undefined} on:click={() => launch(true)} data-modded>
              <span data-icon>
                <Fa icon={faCirclePlay} />
              </span>
              <span>{t("profile.sidebar.launch_modded_btn")}</span>
              <span data-arrow>
                <Fa icon={faArrowRightLong} />
              </span>
            </button>

            <div class={styles.gameBtns}>
              <Tooltip content={t("profile.sidebar.game_settings_btn")}>
                <button>
                  <Fa icon={faGear} />
                </button>
              </Tooltip>
            </div>
          </div>
        </section>
        <section classList={{ [styles.sidebar__group]: true, [sidebarStyles.sidebar__profiles]: true }}>
          <h3 class={styles.sidebar__profilesTitle}>
            {t("profile.sidebar.profiles_title")}
            <div class={styles.sidebar__profilesActions}>
              <Tooltip content={t("profile.sidebar.create_profile_tooltip")}>
                <button class={styles.sidebar__profilesActionBtn} onClick={() => setCreatingProfile(true)}>
                  <Fa icon={faPlus} />
                </button>
              </Tooltip>
              <Tooltip content={t("profile.sidebar.import_profile_tooltip")}>
                <button class={styles.sidebar__profilesActionBtn} on:click={() => setImportDialogOpen(true)}>
                  <Fa icon={faFileImport} class={sidebarStyles.sidebar__profileActionsBtnIcon} />
                </button>
              </Tooltip>
            </div>
          </h3>

          <form on:submit={(e) => e.preventDefault()} class={sidebarStyles.sidebar__profilesSearch}>
            <input
              type="text"
              name="profile-search"
              id="profile-search"
              placeholder={t("global.phrases.search")}
              maxLength={100}
              use:bindValue={[profileQuery, setProfileQuery]}
            />
            <SelectDropdown<ProfileSortType>
              multiselect={false}
              options={[
                {
                  value: ProfileSortType.alphabetical,
                  label: "A-Z",
                  selected: () => isProfileSortType(ProfileSortType.alphabetical),
                },
                {
                  value: ProfileSortType.creation_date,
                  label: "Creation Date",
                  selected: () => isProfileSortType(ProfileSortType.creation_date),
                },
              ]}
              label={{ labelText: "preset", preset: "Sort" }}
              onChanged={(key) => setProfileSortType(key)}
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
            <ol
              class={sidebarStyles.sidebar__profilesList}
              id="profiles-list"
              onFocusOut={(e) => {
                if (
                  e.relatedTarget &&
                  e.relatedTarget instanceof HTMLElement &&
                  e.relatedTarget.closest(`#profiles-list`)
                )
                  return;
                setPivotProfile(undefined);
                setSelectedProfiles([]);
              }}
            >
              <Show when={creatingProfile()}>
                <li class={sidebarStyles.profileList__item}>
                  <SidebarProfileNameEditor
                    initialValue={t("profile.default_profile_name")}
                    onSubmit={async (value) => {
                      await createProfile(params.gameId, value);
                      await refetchProfiles();
                    }}
                    onCancel={() => setCreatingProfile(false)}
                  />
                </li>
              </Show>
              <Show when={queriedProfiles().length === 0 && !creatingProfile()}>
                <li class={sidebarStyles.profileList__noProfilesMsg}>{t("profile.sidebar.no_profiles_search_msg")}</li>
              </Show>
              <For each={queriedProfiles()}>
                {(profile) => (
                  <Show when={profile.pinned}>
                    <SidebarProfileComponent
                      gameId={params.gameId}
                      profile={profile}
                      refetchProfiles={refetchProfiles}
                      selected={profile.id === params.profileId}
                      highlighted={selectedProfiles().includes(profile.id)}
                      ctrlClick={onCtrlClickProfile}
                      shiftClick={onShiftClickProfile}
                      isPivot={effectivePivot() === profile.id}
                    />
                  </Show>
                )}
              </For>
              <For each={queriedProfiles()}>
                {(profile) => (
                  <Show when={!profile.pinned}>
                    <SidebarProfileComponent
                      gameId={params.gameId}
                      profile={profile}
                      refetchProfiles={refetchProfiles}
                      selected={profile.id === params.profileId}
                      highlighted={selectedProfiles().includes(profile.id)}
                      ctrlClick={onCtrlClickProfile}
                      shiftClick={onShiftClickProfile}
                      isPivot={effectivePivot() === profile.id}
                    />
                  </Show>
                )}
              </For>
            </ol>
          </OverlayScrollbarsComponent>
        </section>

        {/* Displayed on small window width */}
        <button class={styles.expandBtn} onClick={() => setSidebarOpen((open) => !open)}>
          <Fa icon={faAnglesRight} rotate={sidebarOpen() ? 180 : 0} />
        </button>

        <section class={styles.sidebar__group}>
          <A href="/settings" class={styles.sidebar__settingsLink}>
            <Fa icon={faGear} />

            <span>{t("settings.link_title")}</span>
          </A>
        </section>
      </aside>

      <div class={styles.content}>
        <Show
          when={params.profileId}
          fallback={
            <div class={styles.noProfilesMsg}>
              <h2>{t("profile.blank.no_profiles_title")}</h2>
              <p>{t("profile.blank.no_profiles_subtitle")}</p>

              <div class={styles.noProfilesMsg__btns}>
                <button data-btn="primary" onClick={() => setCreatingProfile(true)}>
                  {t("profile.blank.create_profile_btn")}
                </button>
              </div>
            </div>
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
                <TabRenderer
                  id="profile"
                  styles={{ preset: "moving-bg", classes: { container: styles.tabs } }}
                  tabs={[
                    {
                      id: "mod-list",
                      name: "Installed Mods",
                      component: () => <InstalledModList game={params.gameId} />,
                    },

                    {
                      id: "mod-search",
                      name: "Online Mods",
                      component: () => <OnlineModList game={params.gameId} />,
                    },

                    {
                      id: "logs",
                      name: "Logs",
                      component: () => (
                        <div class={styles.content__console}>
                          <Console />
                        </div>
                      ),
                    },

                    {
                      id: "config",
                      name: "Config",
                      component: () => <div></div>,
                    },
                  ]}
                />
              </ModInstallContext.Provider>
            );
          }}
        </Show>
      </div>

      <StatusBar />

      <Show when={importDialogOpen()}>
        <ImportDialog dismiss={() => setImportDialogOpen(false)} gameId={params.gameId} profile={params.profileId} />
      </Show>

      <DoctorReports />

      <Show when={err()}>{(err) => <ErrorDialog err={err()} reset={() => setErr(undefined)} />}</Show>

      <Show when={pickingGame()}>
        <Portal>
          <GameSelect replace={false} dismiss={() => setPickingGame(false)} />
        </Portal>
      </Show>
    </main>
  );
}

type Refetcher<T> = () => T | undefined | null | Promise<T | undefined | null>;

function SidebarContextMenuItem(props: { icon: IconDefinition; label: string; iconClass?: string }) {
  return (
    <>
      <div class={sidebarStyles.contextMenuItem}>
        <Fa icon={props.icon} class={props.iconClass} />
      </div>
      {props.label}
    </>
  );
}

function SidebarProfileComponent(props: {
  gameId: string;
  profile: ProfileWithId;
  refetchProfiles: Refetcher<ProfileWithId[]>;
  selected: boolean;
  highlighted: boolean;
  isPivot: boolean;

  ctrlClick: (profile: ProfileWithId) => void;
  shiftClick: (profile: ProfileWithId) => void;
}) {
  const [confirmingDeletion, setConfirmingDeletion] = createSignal(false);
  const [deleting, setDeleting] = createSignal(false);

  const navigate = useNavigate();

  const [renaming, setRenaming] = createSignal(false);

  const ellipsisAnchorId = createUniqueId();

  return (
    <li class={sidebarStyles.profileList__item}>
      <Show
        when={renaming()}
        fallback={
          <>
            <A
              class={sidebarStyles.profileList__itemName}
              data-highlighted={props.highlighted}
              data-pivot={props.isPivot}
              href={`/profile/${props.gameId}/${props.profile.id}`}
              onClick={(e) => {
                if (shifting() || ctrling()) e.preventDefault();

                // Shift clicks take priority over control clicks,
                // use shift click even if both keys are down
                if (shifting()) {
                  props.shiftClick(props.profile);
                } else if (ctrling()) {
                  props.ctrlClick(props.profile);
                }
              }}
              onDblClick={() => {
                if (!ctrling() && !shifting()) setRenaming(true);
              }}
            >
              {props.profile.name}
            </A>
            <div class={sidebarStyles.profileItem__options}>
              <ActionContext>
                {(busy, wrapAction) => (
                  <Tooltip
                    content={
                      props.profile.pinned
                        ? t("profile.sidebar.unpin_profile_btn")
                        : t("profile.sidebar.pin_profile_btn")
                    }
                  >
                    <button
                      data-pin
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
                  </Tooltip>
                )}
              </ActionContext>
              <Show when={shifting()}>
                <Tooltip content={t("profile.sidebar.duplicate_profile_btn")}>
                  <button data-duplicate>
                    <Fa icon={faCopy} />
                  </button>
                </Tooltip>
                <Tooltip content={t("profile.sidebar.copy_id_profile_btn")}>
                  <button data-copy-id>
                    <Fa icon={faClipboard} />
                  </button>
                </Tooltip>
                <Tooltip content={t("profile.sidebar.delete_profile_btn")}>
                  <button data-delete>
                    <Fa icon={faTrashCan} />
                  </button>
                </Tooltip>
              </Show>
              <Tooltip content={t("profile.sidebar.ellipsis_btn")} anchorId={ellipsisAnchorId}>
                <ContextMenu
                  anchorId={ellipsisAnchorId}
                  items={[
                    {
                      label: (
                        <SidebarContextMenuItem icon={faPenToSquare} label={t("profile.sidebar.rename_profile_btn")} />
                      ),
                      action() {
                        setRenaming(true);
                      },
                    },
                    {
                      label: (
                        <SidebarContextMenuItem icon={faTrashCan} label={t("profile.sidebar.delete_profile_btn")} />
                      ),
                      action() {
                        setConfirmingDeletion(true);
                      },
                    },
                    {
                      label: (
                        <SidebarContextMenuItem icon={faCopy} label={t("profile.sidebar.duplicate_profile_btn")} />
                      ),
                      action() {
                        // TODO: implement duplicate profile
                      },
                    },
                    {
                      label: <SidebarContextMenuItem icon={faShare} label={t("profile.sidebar.share_profile_btn")} />,
                      action() {
                        // TODO: implement share profile
                      },
                    },
                    {
                      label: "spacer",
                    },
                    {
                      label: (
                        <SidebarContextMenuItem icon={faClipboard} label={t("profile.sidebar.copy_id_profile_btn")} />
                      ),
                      action() {
                        // TODO: implement copy profile ID
                      },
                    },
                    {
                      label: (
                        <SidebarContextMenuItem
                          icon={faFolderOpen}
                          label={t("profile.sidebar.open_folder_profile_btn")}
                          iconClass={sidebarStyles.openFolderIcon}
                        />
                      ),
                      action() {
                        // TODO: implement open folder
                      },
                    },
                  ]}
                >
                  <Fa icon={faEllipsisVertical} />
                </ContextMenu>
              </Tooltip>
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
        <SidebarProfileNameEditor
          initialValue={props.profile.name}
          onSubmit={async (value) => {
            try {
              const obj: ProfileWithId = { ...props.profile };
              const id = obj.id;
              // @ts-ignore I want to remove the property
              delete obj.id;
              obj.name = value;
              await overwriteProfileMetadata(id, obj);
            } finally {
              await props.refetchProfiles();
            }
          }}
          onCancel={() => setRenaming(false)}
        />
      </Show>
    </li>
  );
}

function SidebarProfileNameEditor(props: {
  initialValue: string;
  onSubmit: (value: string) => Promise<void>;
  onCancel: () => void;
}) {
  return (
    <ActionContext>
      {(busy, wrapAction) => (
        <form
          class={sidebarStyles.profileList__itemName}
          on:submit={async (e) => {
            e.preventDefault();
            await wrapAction(async () => {
              await props.onSubmit((e.target.firstChild as HTMLInputElement).value);
            });
          }}
        >
          <input
            value={props.initialValue}
            disabled={busy()}
            use:autofocus
            on:focus={(e) => {
              const target = e.target as HTMLInputElement;
              target.setSelectionRange(0, target.value.length);
            }}
            on:focusout={() => props.onCancel()}
            on:keydown={(e) => {
              if (e.key === "Escape") {
                props.onCancel();
              }
            }}
          />
        </form>
      )}
    </ActionContext>
  );
}
