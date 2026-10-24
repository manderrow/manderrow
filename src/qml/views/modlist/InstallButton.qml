import QtQuick
import Manderrow

// Installs a mod into the current profile, ported from InstallButton in InstallationBtns.tsx.
// The progress shown is that of the mod's download.
AsyncButton {
    id: button

    /// A mod listing or summary (the latest version is installed) or a package (its version is
    /// installed).
    required property var mod
    required property var installContext

    progressMode: "listener"
    btn.bgColor: Theme.primary40
    btn.activeBgColor: Theme.primary50

    action: listener => {
        const mod = button.mod;
        const metadata = Util.removeProperty(mod, mod.versions !== undefined ? "versions" : "version");
        // a summary's, which isn't part of the metadata
        delete metadata.total_downloads;
        const version = mod.versions !== undefined ? mod.versions[0] : mod.version;
        let foundDownloadTask = false;
        return Api.installProfileMod(button.installContext.profileId, metadata, version, event => {
            if (foundDownloadTask || event.event !== "dependency")
                return;
            const dependency = Tasks.get(event.dependency);
            if (dependency.metadata.kind === "Download") {
                foundDownloadTask = true;
                Tasks.registerTaskListener(event.dependency, listener);
            } else if (dependency.status.status === "Unstarted") {
                // wait for metadata to be filled in and check again
                Tasks.registerTaskListener(event.dependency, depEvent => {
                    if (!foundDownloadTask && depEvent.event === "created" && depEvent.metadata.kind === "Download") {
                        foundDownloadTask = true;
                        Tasks.registerTaskListener(event.dependency, listener);
                    }
                });
            }
        }).then(() => button.installContext.refetchInstalled());
    }
}
