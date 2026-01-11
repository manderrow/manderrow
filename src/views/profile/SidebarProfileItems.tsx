import {
  faClipboard,
  faCopy,
  faEllipsisVertical,
  faFolderOpen,
  faPenToSquare,
  faPlayCircle,
  faShare,
  faThumbTack,
  faThumbTackSlash,
  faTrashCan,
  IconDefinition,
} from "@fortawesome/free-solid-svg-icons";
import { A, useNavigate } from "@solidjs/router";
import Fa from "solid-fa";
import { createSignal, createUniqueId, Show } from "solid-js";

import { deleteProfile, overwriteProfileMetadata, ProfileWithId } from "../../api/api";
import { connections, connectionsUpdate } from "../../api/console";
import { ctrling, shifting } from "../../globals";
import { autofocus } from "../../components/Directives";

import { t } from "../../i18n/i18n";
import { ActionContext } from "../../widgets/AsyncButton";
import ContextMenu from "../../widgets/ContextMenu";
import { PromptDialog } from "../../widgets/Dialog";
import Tooltip, { TooltipAnchor, TooltipTrigger } from "../../widgets/Tooltip";

import sidebarStyles from "./SidebarProfiles.module.css";

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

export function SidebarProfileComponent(props: {
  gameId: string;
  profile: ProfileWithId;
  refetchProfiles: Refetcher<ProfileWithId[]>;
  selected: boolean;
  highlighted: boolean;
  isPivot: boolean;

  ctrlClick: () => void;
  shiftClick: () => void;
}) {
  const [confirmingDeletion, setConfirmingDeletion] = createSignal(false);
  const [deleting, setDeleting] = createSignal(false);

  const navigate = useNavigate();

  const [renaming, setRenaming] = createSignal(false);

  const ellipsisAnchorId = createUniqueId();

  const activeConnection = () => connections.get(connectionsUpdate());

  return (
    <li class={sidebarStyles.profileList__item} data-highlighted={props.highlighted} data-pivot={props.isPivot}>
      <Show
        when={renaming()}
        fallback={
          <>
            <A
              class={sidebarStyles.profileList__itemName}
              href={`/profile/${props.gameId}/${props.profile.id}`}
              onClick={(e) => {
                if (shifting() || ctrling()) e.preventDefault();

                // Shift clicks take priority over control clicks,
                // use shift click even if both keys are down
                if (shifting()) {
                  props.shiftClick();
                } else if (ctrling()) {
                  props.ctrlClick();
                }
              }}
              onDblClick={() => {
                if (!ctrling() && !shifting()) setRenaming(true);
              }}
            >
              <Show
                when={
                  activeConnection()?.status() !== "disconnected" && activeConnection()?.profileId === props.profile.id
                }
              >
                <Tooltip content={t("profile.sidebar.profile_running_icon")}>
                  <TooltipAnchor as={Fa} icon={faPlayCircle} class={sidebarStyles.profileItem__playingIcon} />
                </Tooltip>
              </Show>
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
                    <TooltipTrigger
                      data-pin
                      disabled={busy()}
                      onClick={async (e) => {
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
                    </TooltipTrigger>
                  </Tooltip>
                )}
              </ActionContext>
              <Show when={shifting()}>
                <Tooltip content={t("profile.sidebar.duplicate_profile_btn")}>
                  <TooltipTrigger data-duplicate>
                    <Fa icon={faCopy} />
                  </TooltipTrigger>
                </Tooltip>
                <Tooltip content={t("profile.sidebar.copy_id_profile_btn")}>
                  <TooltipTrigger data-copy-id>
                    <Fa icon={faClipboard} />
                  </TooltipTrigger>
                </Tooltip>
                <Tooltip content={t("global.phrases.delete")}>
                  <TooltipTrigger
                    data-delete
                    onClick={async () => {
                      try {
                        await deleteProfile(props.profile.id);
                      } finally {
                        await props.refetchProfiles();
                      }
                    }}
                  >
                    <Fa icon={faTrashCan} />
                  </TooltipTrigger>
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
                      label: <SidebarContextMenuItem icon={faTrashCan} label={t("global.phrases.delete")} />,
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
                  title: t("global.phrases.confirm"),
                  question: t("profile.delete_msg", { profileName: props.profile.name }),
                  btns: {
                    ok: {
                      type: "danger",
                      text: t("global.phrases.delete"),
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

export function SidebarProfileNameEditor(props: {
  initialValue: string;
  onSubmit: (value: string) => Promise<void>;
  onCancel: () => void;
}) {
  return (
    <ActionContext>
      {(busy, wrapAction) => {
        async function submit(name: string) {
          await wrapAction(async () => {
            await props.onSubmit(name);
          });

          props.onCancel(); // close editor after submitting
        }

        return (
          <form
            class={sidebarStyles.profileList__itemName}
            on:submit={async (e) => {
              e.preventDefault();
              await submit((e.target.firstChild as HTMLInputElement).value);
            }}
          >
            <input
              value={props.initialValue}
              disabled={busy()}
              use:autofocus
              onFocus={(e) => {
                const target = e.target as HTMLInputElement;
                target.setSelectionRange(0, target.value.length);
              }}
              onFocusOut={async (e) => {
                if (e.target.value !== "") await submit(e.target.value);
                props.onCancel();
              }}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  props.onCancel();
                }
              }}
            />
          </form>
        );
      }}
    </ActionContext>
  );
}
