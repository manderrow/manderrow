import QtQuick
import Manderrow

// The mods installed in the current profile, shared by the mod lists. Ported from
// ModInstallContext in ModList.tsx.
QtObject {
    id: context

    property string profileId
    /// The installed mod packages.
    property var installed: []
    property int _generation: 0

    function refetchInstalled() {
        const generation = ++_generation;
        const profileId = context.profileId;
        return Api.getProfileMods(profileId).then(mods => {
            if (context && generation === context._generation)
                context.installed = mods;
        });
    }

    function getInstalled(owner, name) {
        return installed.find(pkg => pkg.owner === owner && pkg.name === name);
    }

    /// The installed package of a mod, or the mod itself if it is a package.
    function installedOf(mod) {
        if (mod.version !== undefined)
            return mod;
        return getInstalled(mod.owner, mod.name);
    }

    onProfileIdChanged: {
        installed = [];
        if (profileId !== "")
            refetchInstalled().catch(e => Ui.window.reportError(e));
    }
}
