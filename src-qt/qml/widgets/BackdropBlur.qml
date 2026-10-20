import QtQuick
import QtQuick.Effects
import Manderrow

// CSS's `backdrop-filter: blur()`: shows what is drawn behind this item, blurred, under an optional
// tint, as in `background-color`. Qt Quick can't read back what is behind an item, so this blurs a
// copy of `source` instead, which should be the content that this is placed over.
Item {
    id: backdrop

    enum Quality {
        /// MultiEffect's blur, which is cheap enough to redraw while the content scrolls, but is
        /// softer and boxier than CSS's.
        Fast,
        /// A true Gaussian blur, as CSS draws.
        Gaussian
    }

    /// The content behind this item.
    property Item source: Ui.appContent
    /// The part of `source` that this covers, in its coordinates. The default, an empty rectangle,
    /// is all of it.
    property rect sourceRect
    /// The blur's standard deviation, as in `blur()`.
    property real radius: 0
    property int quality: BackdropBlur.Fast
    property real cornerRadius: 0
    property color tint: "transparent"
    /// Drawn under the blurred copy. The content is transparent where the window's background
    /// shows through it, so without this the sharp content beneath would show through the copy.
    property color backgroundColor: Ui.window ? Ui.window.color : "transparent"

    Rectangle {
        anchors.fill: parent
        radius: backdrop.cornerRadius
        color: backdrop.backgroundColor
    }

    Loader {
        anchors.fill: parent
        active: backdrop.source !== null
        sourceComponent: backdrop.quality === BackdropBlur.Gaussian ? gaussian : fast
    }

    Component {
        id: gaussian
        GaussianBlur {
            source: backdrop.source
            sourceRect: backdrop.sourceRect
            radius: backdrop.radius
            cornerRadius: backdrop.cornerRadius
        }
    }

    Component {
        id: fast
        MultiEffect {
            source: ShaderEffectSource {
                sourceItem: backdrop.source
                sourceRect: backdrop.sourceRect
            }
            blurEnabled: true
            blur: 1
            blurMax: Theme.blurMax(backdrop.radius)
            autoPaddingEnabled: false
            maskEnabled: backdrop.cornerRadius > 0
            maskSource: maskEnabled ? mask : null
            // smooth the mask's antialiased edge instead of thresholding it
            maskThresholdMin: 0.5
            maskSpreadAtMin: 1

            Rectangle {
                id: mask
                anchors.fill: parent
                radius: backdrop.cornerRadius
                visible: false
                layer.enabled: true
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: backdrop.cornerRadius
        color: backdrop.tint
    }
}
