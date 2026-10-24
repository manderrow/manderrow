import QtQuick
import Manderrow

// An image decoded at the display's pixel density, so it isn't upscaled on high-DPI screens.
// By default it covers its area, like `object-fit: cover`.
Image {
    readonly property real _dpr: Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1
    // Qt itself requests images from providers, and draws vector images, at the source size times
    // the pixel ratio
    readonly property bool _scaledByQt: /^image:|(svg|svgz|pdf)$/.test(String(source).split(/[?#]/)[0])
    readonly property real _scale: _scaledByQt ? 1 : _dpr

    fillMode: Image.PreserveAspectCrop
    sourceSize: Qt.size(Math.ceil(width * _scale), Math.ceil(height * _scale))
    asynchronous: true
    smooth: true
}
