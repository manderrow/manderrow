import { ComponentProps, JSX, splitProps } from "solid-js";

import { installProfileMod, uninstallProfileMod } from "../../../api/api";
import { registerTaskListener, tasks } from "../../../api/tasks";
import { Mod, ModPackage } from "../../../types";
import { removeProperty } from "../../../utils/utils";
import { ModInstallContext } from "./ModList";

import { ProgressStyle, SimpleAsyncButton } from "../../../widgets/AsyncButton";

export function InstallButton(
  props: ComponentProps<"button"> & {
    mod: Mod;
    installContext: NonNullable<typeof ModInstallContext.defaultValue>;
    busyClass?: JSX.HTMLAttributes<Element>["class"];
    progressStyle?: ProgressStyle;
  },
) {
  const [local, rest] = splitProps(props, ["mod", "installContext", "busyClass", "progressStyle", "onClick"]);

  return (
    <SimpleAsyncButton
      progressStyle={local.progressStyle}
      progress
      busyClass={local.busyClass}
      data-install
      onClick={async (listener) => {
        let foundDownloadTask = false;
        await installProfileMod(
          local.installContext.profileId(),
          "versions" in local.mod ? removeProperty(local.mod, "versions") : removeProperty(local.mod, "version"),
          "versions" in local.mod ? local.mod.versions[0] : local.mod.version,
          (event) => {
            if (!foundDownloadTask && event.event === "dependency") {
              const dependency = tasks().get(event.dependency)!;
              if (dependency.metadata.kind === "Download") {
                foundDownloadTask = true;
                registerTaskListener(event.dependency, listener);
              } else if (dependency.status.status === "Unstarted") {
                // wait for metadata to be filled in and check again
                registerTaskListener(event.dependency, (depEvent) => {
                  if (!foundDownloadTask && depEvent.event === "created" && depEvent.metadata.kind === "Download") {
                    foundDownloadTask = true;
                    registerTaskListener(event.dependency, listener);
                  }
                });
              }
            }
          },
        );
        await local.installContext.refetchInstalled();
      }}
      {...rest}
    >
      {props.children}
    </SimpleAsyncButton>
  );
}

export function UninstallButton(
  props: ComponentProps<"button"> & {
    mod: ModPackage;
    installContext: NonNullable<typeof ModInstallContext.defaultValue>;
    busyClass?: JSX.HTMLAttributes<Element>["class"];
    progressStyle?: ProgressStyle;
  },
) {
  const [local, rest] = splitProps(props, ["mod", "installContext", "busyClass", "progressStyle", "onClick"]);

  return (
    <SimpleAsyncButton
      progressStyle={local.progressStyle}
      progress
      data-uninstall
      busyClass={local.busyClass}
      onClick={async (_listener) => {
        await uninstallProfileMod(local.installContext.profileId(), local.mod.owner, local.mod.name);
        await local.installContext.refetchInstalled();
      }}
      {...rest}
    >
      {props.children}
    </SimpleAsyncButton>
  );
}
