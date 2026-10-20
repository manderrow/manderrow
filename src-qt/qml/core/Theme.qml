pragma Singleton
import QtQuick
import Manderrow

// The design tokens of the app, transcribed from the web UI's Theme.css and App.css.
QtObject {
    id: theme

    /// Follows the system color scheme, like `prefers-color-scheme` did. The `theme` dev option
    /// overrides it.
    readonly property string _forced: Backend.devOption("theme")
    readonly property bool dark: _forced !== "" ? _forced === "dark" : Qt.styleHints.colorScheme === Qt.ColorScheme.Dark

    /// The width of the window, which drives the responsive breakpoints.
    property real windowWidth: 1024

    /// The root font size. The web UI shrank it to 12px on narrow windows.
    readonly property real rem: windowWidth <= 700 ? 12 : 16

    readonly property string font: "Inter"
    readonly property string fontMonospace: "Source Code Pro"

    // CSS timing functions, for `easing.bezierCurve` with `easing.type: Easing.Bezier`
    readonly property var ease: [0.25, 0.1, 0.25, 1, 1, 1]
    readonly property var easeIn: [0.42, 0, 1, 1, 1, 1]
    readonly property var easeOut: [0, 0, 0.58, 1, 1, 1]
    readonly property var easeInOut: [0.42, 0, 0.58, 1, 1, 1]

    readonly property real titlebarHeight: 28
    readonly property real titlebarLogoSize: 16

    function hsl(h, s, l, a) {
        return Qt.hsla(h / 360, s / 100, l / 100, a === undefined ? 1 : a);
    }

    /// Returns `color` with its alpha replaced.
    function alpha(color, a) {
        return Qt.rgba(color.r, color.g, color.b, a);
    }

    // MultiEffect blurMax for a CSS `blur(sigma)`, measured by blurring an edge and comparing its
    // 10-90% width to a Gaussian's. MultiEffect works in texture pixels, hence the pixel ratio.
    readonly property var _blurCurve: [[0, 0], [1, 5], [2, 10], [3, 15], [4, 20], [6, 27], [8, 32], [15, 48], [21, 64]]

    function blurMax(sigma) {
        const s = sigma * Ui.dpr;
        const curve = _blurCurve;
        for (let i = 1; i < curve.length; i++) {
            if (s <= curve[i][0]) {
                const [x0, y0] = curve[i - 1];
                const [x1, y1] = curve[i];
                return y0 + (y1 - y0) * (s - x0) / (x1 - x0);
            }
        }
        return 64;
    }

    /// The color to animate from `color` towards `other`. Animating from "transparent" (transparent
    /// black) would pass through dark tints, unlike CSS, which interpolates premultiplied colors.
    function fadeFrom(color, other) {
        return color.a === 0 ? Qt.rgba(other.r, other.g, other.b, 0) : color;
    }

    /// Returns `color` with its HSL lightness shifted by `delta` percentage points.
    function lighten(color, delta) {
        return Qt.hsla(color.hslHue, color.hslSaturation, Math.max(0, Math.min(1, color.hslLightness + delta / 100)), color.a);
    }

    readonly property color comp: "#ec9d0c"

    readonly property color neutral05: dark ? hsl(211, 20, 95) : hsl(214, 28, 5)
    readonly property color neutral10: dark ? hsl(212, 18, 90) : hsl(214, 28, 10)
    readonly property color neutral15: dark ? hsl(212, 18, 85) : hsl(214, 28, 15)
    readonly property color neutral20: dark ? hsl(212, 15, 80) : hsl(214, 28, 20)
    readonly property color neutral25: dark ? hsl(212, 15, 75) : hsl(218, 20, 25)
    readonly property color neutral30: dark ? hsl(210, 12, 70) : hsl(218, 20, 30)
    readonly property color neutral45: hsl(210, 15, 45)
    readonly property color neutral50: hsl(210, 15, 50)
    readonly property color neutral55: hsl(210, 10, 55)
    readonly property color neutral60: dark ? hsl(210, 15, 50) : hsl(210, 10, 60)
    readonly property color neutral65: dark ? hsl(210, 15, 45) : hsl(210, 12, 65)
    readonly property color neutral70: dark ? hsl(218, 20, 30) : hsl(210, 12, 70)
    readonly property color neutral75: dark ? hsl(218, 20, 25) : hsl(212, 15, 75)
    readonly property color neutral80: dark ? hsl(214, 28, 20) : hsl(212, 15, 80)
    readonly property color neutral85: dark ? hsl(214, 28, 15) : hsl(212, 18, 85)
    readonly property color neutral90: dark ? hsl(214, 28, 10) : hsl(212, 18, 90)
    readonly property color neutral95: dark ? hsl(214, 28, 5) : hsl(211, 20, 95)
    readonly property color primary05: hsl(219, 90, 4)
    readonly property color primary10: hsl(219, 90, 9)
    readonly property color primary15: hsl(219, 90, 14)
    readonly property color primary20: hsl(219, 90, 19)
    readonly property color primary25: hsl(219, 90, 24)
    readonly property color primary30: hsl(219, 90, 29)
    readonly property color primary35: hsl(219, 90, 34)
    readonly property color primary40: hsl(219, 90, 39)
    readonly property color primary45: hsl(219, 90, 44)
    readonly property color primary50: hsl(219, 90, 49)
    readonly property color primary55: hsl(219, 90, 54)
    readonly property color primary60: hsl(219, 90, 59)
    readonly property color primary65: hsl(219, 90, 64)
    readonly property color primary70: hsl(219, 90, 69)
    readonly property color primary75: hsl(219, 90, 74)
    readonly property color primary80: hsl(219, 90, 79)
    readonly property color primary85: hsl(219, 90, 84)
    readonly property color primary90: hsl(219, 90, 89)
    readonly property color primary95: hsl(219, 90, 94)
    readonly property color accent1: hsl(309, 63, 10)
    readonly property color accent2: hsl(309, 63, 20)
    readonly property color accent3: hsl(309, 63, 30)
    readonly property color accent4: hsl(309, 63, 40)
    readonly property color accent5: hsl(309, 63, 48)
    readonly property color accent6: hsl(309, 63, 60)
    readonly property color accent7: hsl(309, 63, 75)
    readonly property color accent8: hsl(309, 63, 80)
    readonly property color accent9: hsl(309, 63, 90)
    readonly property color danger1: hsl(1, 85, 10)
    readonly property color danger2: hsl(1, 85, 20)
    readonly property color danger3: hsl(1, 85, 30)
    readonly property color danger4: hsl(1, 85, 36)
    readonly property color danger5: hsl(1, 85, 50)
    readonly property color danger6: hsl(1, 85, 60)
    readonly property color danger7: hsl(1, 85, 70)
    readonly property color danger8: hsl(1, 85, 80)
    readonly property color danger9: hsl(1, 85, 90)
    readonly property color warn1: hsl(34, 100, 10)
    readonly property color warn2: hsl(34, 100, 20)
    readonly property color warn3: hsl(34, 100, 30)
    readonly property color warn4: hsl(34, 100, 40)
    readonly property color warn5: hsl(34, 100, 50)
    readonly property color warn6: hsl(34, 100, 59)
    readonly property color warn7: hsl(34, 100, 70)
    readonly property color warn8: hsl(34, 100, 80)
    readonly property color warn9: hsl(34, 100, 90)
    readonly property color ok1: hsl(146, 100, 10)
    readonly property color ok2: hsl(146, 100, 20)
    readonly property color ok3: hsl(146, 100, 30)
    readonly property color ok4: hsl(146, 100, 40)
    readonly property color ok5: hsl(146, 100, 50)
    readonly property color ok6: hsl(146, 100, 59)
    readonly property color ok7: hsl(146, 100, 70)
    readonly property color ok8: hsl(146, 100, 80)
    readonly property color ok9: hsl(146, 100, 90)
    readonly property color neutral40: hsl(210, 10, 60)
    readonly property color neutral35: hsl(210, 12, 65)
}
