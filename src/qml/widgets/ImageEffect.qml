import QtQuick
import Manderrow

// Draws an Image's texture over this item like `object-fit: cover`, within rounded corners and
// optionally with CSS's blur() and brightness() filters. The image can be hidden, and several
// effects can share it, so the image is decoded and uploaded once.
ShaderEffect {
    id: effect

    required property Image image
    property real cornerRadius: 0
    /// The blur's standard deviation, as in `blur()`. The image needs `mipmap: true`.
    property real blurRadius: 0
    property real brightness: 1
    /// Whether the blur fades to transparent at the edges, as `filter: blur()` does, rather than
    /// extending the edge pixels, as `backdrop-filter: blur()` does.
    property bool blurFadesEdges: true

    readonly property real _dpr: Ui.dpr
    // the image's aspect ratio relative to this item's
    readonly property real _relativeAspect: image.implicitHeight > 0 && height > 0 ? (image.implicitWidth / image.implicitHeight) / (width / height) : 1

    visible: image.status === Image.Ready
    fragmentShader: "qrc:/assets/shaders/image.frag.qsb"

    readonly property Image source: image
    readonly property size size: Qt.size(width * _dpr, height * _dpr)
    readonly property real radius: cornerRadius * _dpr
    readonly property point uvScale: _relativeAspect > 1 ? Qt.point(1 / _relativeAspect, 1) : Qt.point(1, _relativeAspect)
    readonly property point uvOffset: Qt.point((1 - uvScale.x) / 2, (1 - uvScale.y) / 2)
    readonly property real blur: blurRadius * _dpr
    // sample the mip level whose texels are about a third of the blur's deviation
    readonly property real blurBias: blur > 0 ? Math.max(0, Math.log2(blur / 3)) : 0
    readonly property real fadeEdges: blurFadesEdges ? 1 : 0
}
