import QtQuick
import Manderrow

// Uninstalls a mod from the current profile, ported from UninstallButton in InstallationBtns.tsx.
AsyncButton {
    id: button

    /// The installed mod package.
    required property var mod
    required property var installContext

    signal uninstalled

    progressMode: "listener"
    btn.bgColor: Theme.danger4
    btn.activeBgColor: Theme.danger5

    action: () => Api.uninstallProfileMod(button.installContext.profileId, button.mod.owner, button.mod.name).then(() => {
        button.uninstalled();
        return button.installContext.refetchInstalled();
    })
}
