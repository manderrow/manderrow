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

    /// The width of the widest word of some text in a FontMetrics' font: the narrowest the text
    /// can wrap to, its min-content width on the web.
    function longestWordWidth(metrics, text) {
        return Math.ceil(Math.max(0, ...text.split(/\s+/).map(word => metrics.advanceWidth(word))));
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

    /// The source for an image on the web, which loads it with the backend's HTTP client, as
    /// Qt's own TLS support varies by platform. Other sources are returned as they are.
    ///
    /// Given the display's pixel ratio, a vector image is drawn sharply at it rather than scaled
    /// up, though it is still sized as it would be without it. Rich text sizes images by their
    /// pixels, so only give it to rich text for an image that has a width or height of its own.
    ///
    /// Given `maxWidth`, in device pixels, a wider image is decoded at that width, though it is
    /// still sized as it would be without it, for an image that is shown no wider. Decoded, an
    /// image takes 4 bytes for each pixel.
    function webImage(url, dpr, maxWidth) {
        if (!String(url).startsWith("https://"))
            return url;
        let options = "";
        if (dpr > 1)
            options += `${dpr}x/`;
        if (maxWidth > 0)
            options += `${Math.ceil(maxWidth)}w/`;
        return "image://web/" + options + url;
    }
    /// The width, in device pixels, to decode images that are shown no wider than `width` at, for
    /// webImage's `maxWidth`. It is rounded up, so that it only changes now and then as `width`
    /// does, which loads the images again.
    function imageDecodeWidth(width, dpr) {
        return Math.ceil(width * dpr / 128) * 128;
    }
    /// The minimum height of shields.io's badges, which is their known height in the style most
    /// use, so that the space for them is kept while they load, as the web UI's style did.
    readonly property real badgeHeight: 28
    function isBadge(url) {
        return String(url).includes("img.shields.io");
    }
    /// Loads the web images of rich text as webImage does, and sizes them as the web UI's style
    /// did: no wider than the text, which is `img { max-width: 100% }`, and badges at their
    /// height. `maxWidth` sets their maximum width in pixels instead, for where rich text takes a
    /// percentage of the wrong width, as in a table's cells.
    ///
    /// `decodeWidth` is webImage's `maxWidth` for the images, which must be no less than the
    /// text's width, or `maxWidth`, in device pixels: rich text sizes an image by its pixels, though
    /// no wider than its maximum width.
    function webImagesInHtml(html, dpr, maxWidth, decodeWidth) {
        const maxWidthStyle = maxWidth !== undefined ? `${Math.max(0, Math.floor(maxWidth))}px` : "100%";
        return html.replace(/<img\b[^>]*>/g, tag => {
            // the sanitizer has removed any style of its own
            tag = tag.replace(/^<img\b/, `<img style="max-width: ${maxWidthStyle}"`);
            const src = /\bsrc="([^"]*)"/.exec(tag);
            if (!src)
                return tag;
            let sized = /\b(?:width|height)="/.test(tag);
            if (!sized && isBadge(src[1])) {
                tag = tag.replace(/^<img\b/, `<img height="${badgeHeight}"`);
                sized = true;
            }
            return tag.replace(src[0], () => `src="${webImage(src[1], sized ? dpr : 1, decodeWidth)}"`);
        });
    }

    // Thunderstore URLs

    /// The source of a mod's icon.
    function getIconUrl(qualifiedModName) {
        return webImage(`https://gcdn.thunderstore.io/live/repository/icons/${qualifiedModName}.png`);
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
