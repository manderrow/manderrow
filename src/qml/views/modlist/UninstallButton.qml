import QtQuick
import Manderrow

// Uninstalls a mod from the profile, ported from UninstallButton in InstallationBtns.tsx.
AsyncButton {
    id: button

    /// The mod installed in the profile.
    required property Mod mod
    required property Profile profile

    signal uninstalled

    btn.bgColor: Theme.danger4
    btn.activeBgColor: Theme.danger5

    action: () => Async.promise(button.profile.uninstallMod(button.mod.owner, button.mod.name)).then(() => {
        // the button may have been destroyed by the time this runs
        if (button)
            button.uninstalled();
    })
}
