pragma Singleton
import QtQuick

// General purpose helpers, ported from utils/utils.ts and views/profile/modlist/common.ts.
QtObject {
    /// Runs a generator function as an async function: each `yield`ed promise is awaited and its
    /// result sent back into the generator. QML's JavaScript engine has no async/await, so this
    /// stands in for it. Returns a promise of the generator's return value.
    function co(generator, thisArg) {
        return new Promise((resolve, reject) => {
            const it = generator.call(thisArg);
            function step(method, arg) {
                let result;
                try {
                    result = it[method](arg);
                } catch (e) {
                    reject(e);
                    return;
                }
                if (result.done) {
                    resolve(result.value);
                } else {
                    Promise.resolve(result.value).then(v => step("next", v), e => step("throw", e));
                }
            }
            step("next", undefined);
        });
    }

    /// Like `Intl.NumberFormat()`: groups digits according to the locale.
    function formatNumber(n) {
        return Number(n).toLocaleString(Qt.locale(), "f", 0);
    }

    function _roundSignificant(n, digits) {
        if (n === 0)
            return 0;
        const magnitude = Math.floor(Math.log10(Math.abs(n)));
        const factor = Math.pow(10, digits - 1 - magnitude);
        return Math.round(n * factor) / factor;
    }

    /// Like `Intl.NumberFormat(undefined, { maximumSignificantDigits: 3, notation: "compact" })`.
    function formatRounded(n) {
        if (!isFinite(n))
            return String(n);
        const suffixes = ["", "K", "M", "B", "T"];
        let value = _roundSignificant(n, 3);
        let i = 0;
        while (Math.abs(value) >= 1000 && i < suffixes.length - 1) {
            value = _roundSignificant(value / 1000, 3);
            i++;
        }
        // drop trailing zeros, as Intl does
        return String(parseFloat(value.toPrecision(3))) + suffixes[i];
    }

    /// Like `Intl.DateTimeFormat(undefined, { year: "numeric", month: "short", day: "numeric" })`.
    function formatDateMed(date) {
        return new Date(date).toLocaleDateString(Qt.locale(), "MMM d, yyyy");
    }

    function formatDateTime(date) {
        return new Date(date).toLocaleString(Qt.locale(), Locale.ShortFormat);
    }

    readonly property var _byteUnits: ["B", "KB", "MB", "GB", "TB"]

    function humanizeFileSize(sizeBytes, space) {
        const i = sizeBytes === 0 ? 0 : Math.floor(Math.log(sizeBytes) / Math.log(1000));
        return (sizeBytes / Math.pow(1000, i)).toFixed(1) + (space ? " " : "") + _byteUnits[i];
    }

    function escapeHtml(s) {
        return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
    }

    function removeProperty(obj, prop) {
        const rest = Object.assign({}, obj);
        delete rest[prop];
        return rest;
    }

    function percentage(progress) {
        if (!progress || !progress.total)
            return 0;
        return progress.completed / progress.total * 100;
    }

    // Thunderstore URLs

    function getIconUrl(qualifiedModName) {
        return `https://gcdn.thunderstore.io/live/repository/icons/${qualifiedModName}.png`;
    }
    function getQualifiedModName(owner, name, version) {
        return `${owner}-${name}-${version}`;
    }
    function concatenateMod(owner, name, version) {
        return `${owner}${name}${version}`;
    }
    function getModVersionUrl(gameId, owner, name, version) {
        return getModAuthorUrl(gameId, owner) + `${name}/versions#:~:text=${version}`;
    }
    function getModUrl(gameId, owner, name) {
        return getModAuthorUrl(gameId, owner) + `${name}/`;
    }
    function getModAuthorUrl(gameId, owner) {
        return `https://thunderstore.io/c/${gameId}/p/${owner}/`;
    }

    function gameCoverUrl(thunderstoreId) {
        return `qrc:/assets/img/game_covers/${thunderstoreId}.webp`;
    }
    function gameHeroUrl(gameId) {
        return `qrc:/assets/img/game_heros/${gameId}.webp`;
    }

    /// The version a mod is displayed with: the installed version of a package, or the latest
    /// version of a listing.
    function displayVersion(mod) {
        return mod.version !== undefined ? mod.version : mod.versions[0];
    }

    function modIdEquals(a, b) {
        return !!a && !!b && a.owner === b.owner && a.name === b.name;
    }

    function openUrl(url) {
        if (!Qt.openUrlExternally(url))
            console.error(`Failed to open link: ${url}`);
    }
}
