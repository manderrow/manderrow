import QtQuick
import Manderrow

// Installs a mod into the profile, ported from InstallButton in InstallationBtns.tsx. The progress
// shown is that of the mod's download.
AsyncButton {
    id: button

    /// A mod listing or summary, or a mod installed in the profile.
    required property Mod mod
    required property Profile profile
    /// The version to install: by default, the latest of a listing, or the one installed.
    property string versionNumber: mod.displayVersion ? mod.displayVersion.versionNumber : ""

    progressMode: "task"
    downloadProgress: true
    btn.bgColor: Theme.primary40
    btn.activeBgColor: Theme.primary50

    action: () => button.profile.installMod(button.mod.owner, button.mod.name, button.versionNumber)
}
