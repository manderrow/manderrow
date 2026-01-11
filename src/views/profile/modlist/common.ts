import { Accessor, createMemo } from "solid-js";

import { Mod, ModPackage } from "../../../types";
import { ModInstallContext } from "./ModList";

export function getIconUrl(qualifiedModName: string) {
  return `https://gcdn.thunderstore.io/live/repository/icons/${qualifiedModName}.png`;
}
export function getQualifiedModName(owner: string, name: string, version: string) {
  return `${owner}-${name}-${version}`;
}
export function getModVersionUrl(gameId: string, owner: string, name: string, version: string) {
  return getModAuthorUrl(gameId, owner) + `${name}/versions#:~:text=${version}`;
}
export function getModUrl(gameId: string, owner: string, name: string) {
  return getModAuthorUrl(gameId, owner) + `${name}/`;
}
export function getModAuthorUrl(gameId: string, owner: string) {
  return `https://thunderstore.io/c/${gameId}/p/${owner}/`;
}

export function useInstalled(
  installContext: typeof ModInstallContext.defaultValue,
  modAccessor: Accessor<Mod>,
): Accessor<ModPackage | undefined> {
  return createMemo(() => {
    const mod = modAccessor();
    if ("version" in mod) {
      return mod;
    } else {
      return installContext?.installed.latest.find((pkg) => pkg.owner === mod.owner && pkg.name === mod.name);
    }
  });
}
