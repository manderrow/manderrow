import {
  createEffect,
  createMemo,
  createResource,
  createSelector,
  createSignal,
  For,
  Match,
  Show,
  Switch,
} from "solid-js";
import { createStore, SetStoreFunction } from "solid-js/store";
import { A, useNavigate, useParams } from "@solidjs/router";
import {
  faCirclePlay,
  faFileImport,
  faPlus,
  faGear,
  faArrowUpWideShort,
  faArrowDownShortWide,
  faSkullCrossbones,
  faXmark,
  faAnglesRight,
  faChevronDown,
  faArrowRightLong,
} from "@fortawesome/free-solid-svg-icons";
import { OverlayScrollbarsComponent } from "overlayscrollbars-solid";
import { Fa } from "solid-fa";

import Console, { DoctorReports } from "../../components/Console.tsx";
import { DialogTrigger } from "../../widgets/Dialog.tsx";
import ErrorDialog from "../../components/ErrorDialog.tsx";
import SelectDropdown from "../../widgets/SelectDropdown.tsx";
import TabRenderer from "../../widgets/TabRenderer";
import { InstalledModList, ModInstallContext, OnlineModList } from "./modlist/ModList.tsx";

import { createProfile, getProfileMods, ProfileWithId } from "../../api/api";
import * as globals from "../../globals.ts";
import { initialGame, refetchProfiles } from "../../globals.ts";
// @ts-ignore: TS is unaware of `use:` directives despite using them for type definitions
import { bindValue } from "../../components/Directives";

import styles from "./Profile.module.css";
import sidebarStyles from "./SidebarProfiles.module.css";
import ImportDialog from "./ImportDialog.tsx";
import { settings } from "../../api/settings.ts";
import { useSearchParamsInPlace } from "../../utils/router.ts";
import { killIpcClient } from "../../api/ipc.ts";
import { t } from "../../i18n/i18n.ts";
import { launchProfile } from "../../api/launching.ts";
import { ConsoleConnection, focusedConnection, setFocusedConnection } from "../../api/console";
import { setCurrentProfileName } from "../../components/TitleBar.tsx";
import Tooltip, { TooltipTrigger } from "../../widgets/Tooltip.tsx";
import GameSelect from "../game_select/GameSelect.tsx";
import StatusBar from "../profile/StatusBar.tsx";
import { createMultiselectableList } from "../../utils/utils.ts";
import { SidebarProfileComponent, SidebarProfileNameEditor } from "./SidebarProfileItems.tsx";

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

interface ShowGameSelectState {
  shouldShow: boolean;
  showing: boolean;
}

export default function Profile() {
  // @ts-expect-error params.profileId is an optional param, it can be undefined, and we don't expect any other params
  const params = useParams<ProfileParams>();
  const navigate = useNavigate();

  const [gameSelect, setGameSelect] = createStore<ShowGameSelectState>({
    shouldShow: params.gameId == null,
    showing: params.gameId == null,
  });

  createEffect(() => {
    const game = initialGame.latestOrThrow;
    if (game) {
      navigate(`/profile/${game}`, { replace: true });
    }
  });

  return (
    <>
      <Show when={gameSelect.showing}>
        <GameSelect
          replace={true}
          shouldShow={gameSelect.shouldShow}
          beginDismiss={() => setGameSelect("shouldShow", false)}
          finishDismiss={() => setGameSelect("showing", false)}
        />
      </Show>
      <Show when={params.gameId}>
        {(gameId) => (
          <ProfileWithGame gameId={gameId()} profileId={params.profileId} gameSelect={[gameSelect, setGameSelect]} />
        )}
      </Show>
    </>
  );
}

function ProfileWithGame(
  params: ProfileParams & {
    gameId: string;
    gameSelect: [ShowGameSelectState, SetStoreFunction<ShowGameSelectState>];
  },
) {
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
    const profile = currentProfile();
    setCurrentProfileName(profile == null || params.gameSelect[0].showing ? "" : profile.name);
  });

  // track launch errors here instead of reporting to the error boundary to avoid rebuilding the UI
  const [err, setErr] = createSignal<unknown>();

  async function launch(modded: boolean) {
    try {
      const conn = await ConsoleConnection.allocate(params.profileId);
      try {
        setFocusedConnection(conn);
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

  const hasLiveConnection = () => focusedConnection() !== undefined && focusedConnection()?.status() !== "disconnected";

  // For mini sidebar on small app width only
  const [sidebarOpen, setSidebarOpen] = createSignal(false);

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

  // Profiles are stored in the key values of the `selected` Map
  const { onCtrlClickItem, onShiftClickItem, clearSelection, isPivot, isSelected } = createMultiselectableList<
    ProfileWithId,
    string,
    string
  >(
    queriedProfiles,
    (profile) => profile.id,
    (profile) => profile.id,
    currentProfile,
  );

  return (
    <main class={styles.main}>
      <aside class={styles.sidebar} data-sidebar-open={sidebarOpen()}>
        <section
          classList={{ [styles.sidebar__group]: true, [styles.sidebar__mainActions]: true }}
          style={{ "--game-hero--url": `url("/img/game_heros/${gameInfo().id}.webp")` }}
        >
          <nav class={styles.sidebar__nav}>
            <button class={styles.backBtn} on:click={() => params.gameSelect[1]({ shouldShow: true, showing: true })}>
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
            <button disabled={params.profileId === undefined} on:click={() => launch(true)} data-launch>
              <Fa icon={faCirclePlay} data-icon />
              <span>{t("profile.sidebar.launch_modded_btn")}</span>
              <span data-arrow>
                <Fa icon={faArrowRightLong} />
              </span>
            </button>

            <div class={styles.gameBtns}>
              <Tooltip content={t("profile.sidebar.game_settings_btn")}>
                <TooltipTrigger>
                  <Fa icon={faGear} />
                </TooltipTrigger>
              </Tooltip>
            </div>
          </div>
        </section>
        <section classList={{ [styles.sidebar__group]: true, [sidebarStyles.sidebar__profiles]: true }}>
          <h3 class={styles.sidebar__profilesTitle}>
            {t("profile.sidebar.profiles_title")}
            <div class={styles.sidebar__profilesActions}>
              <Tooltip content={t("profile.sidebar.create_profile_tooltip")}>
                <TooltipTrigger class={styles.sidebar__profilesActionBtn} onClick={() => setCreatingProfile(true)}>
                  <Fa icon={faPlus} />
                </TooltipTrigger>
              </Tooltip>
              <Tooltip content={t("profile.sidebar.import_profile_tooltip")}>
                <ImportDialog
                  gameId={params.gameId}
                  profile={params.profileId}
                  trigger={
                    <DialogTrigger as={TooltipTrigger} class={styles.sidebar__profilesActionBtn}>
                      <Fa icon={faFileImport} class={sidebarStyles.sidebar__profileActionsBtnIcon} />
                    </DialogTrigger>
                  }
                />
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
                  label: t("global.sort_type.creation_date"),
                  selected: () => isProfileSortType(ProfileSortType.creation_date),
                },
              ]}
              label={{ labelText: "preset", preset: t("global.sort_title") }}
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
                clearSelection();
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
                {(profile, i) => (
                  <Show when={profile.pinned}>
                    <SidebarProfileComponent
                      gameId={params.gameId}
                      profile={profile}
                      refetchProfiles={refetchProfiles}
                      selected={profile.id === params.profileId}
                      highlighted={isSelected(profile)}
                      ctrlClick={() => onCtrlClickItem(profile, i())}
                      shiftClick={() => onShiftClickItem(profile, i())}
                      isPivot={isPivot(profile)}
                    />
                  </Show>
                )}
              </For>
              <For each={queriedProfiles()}>
                {(profile, i) => (
                  <Show when={!profile.pinned}>
                    <SidebarProfileComponent
                      gameId={params.gameId}
                      profile={profile}
                      refetchProfiles={refetchProfiles}
                      selected={profile.id === params.profileId}
                      highlighted={isSelected(profile)}
                      ctrlClick={() => onCtrlClickItem(profile, i())}
                      shiftClick={() => onShiftClickItem(profile, i())}
                      isPivot={isPivot(profile)}
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
                      name: t("profile.tabs.installed"),
                      component: () => <InstalledModList game={params.gameId} />,
                    },

                    {
                      id: "mod-search",
                      name: t("profile.tabs.online"),
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
                      name: t("profile.tabs.config"),
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

      <DoctorReports />

      <Show when={err()}>{(err) => <ErrorDialog err={err()} reset={() => setErr(undefined)} />}</Show>
    </main>
  );
}
