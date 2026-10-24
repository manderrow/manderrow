import QtQuick
import Manderrow

// A true Gaussian blur of an item, like CSS's blur(), in two passes. MultiEffect's blur is cheaper
// but softer and boxier, which reads as weaker.
Item {
    id: root

    /// The item to blur, which this should be placed over. It can be hidden.
    required property Item source
    /// The part of `source` to blur, in its coordinates. The default, an empty rectangle, is all of
    /// it.
    property rect sourceRect
    /// The blur's standard deviation, as in `blur()`.
    property real radius: 0
    /// Clips the result to rounded corners.
    property real cornerRadius: 0
    /// Hides `source` itself while this shows, for a blur that covers it all.
    property bool hidesSource: false

    readonly property real _dpr: Ui.dpr

    ShaderEffectSource {
        id: capture
        anchors.fill: parent
        sourceItem: root.source
        sourceRect: root.sourceRect
        hideSource: root.hidesSource && root.visible
        visible: false
    }

    component Pass: ShaderEffect {
        required property Item source
        required property point texelStep
        readonly property real deviation: root.radius * root._dpr
        readonly property size size: Qt.size(width * root._dpr, height * root._dpr)
        property real cornerRadius: 0

        anchors.fill: parent
        fragmentShader: "qrc:/assets/shaders/gaussian.frag.qsb"
    }

    Pass {
        id: horizontal
        source: capture
        texelStep: Qt.point(1 / Math.max(1, width * root._dpr), 0)
        visible: false
        layer.enabled: true
        // the samples rely on bilinear filtering
        layer.smooth: true
    }

    Pass {
        source: horizontal
        texelStep: Qt.point(0, 1 / Math.max(1, height * root._dpr))
        cornerRadius: root.cornerRadius * root._dpr
    }
}
