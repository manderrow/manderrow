import QtQuick
import Manderrow

// An image decoded at the display's pixel density, so it isn't upscaled on high-DPI screens.
// By default it covers its area, like `object-fit: cover`.
Image {
    readonly property real _dpr: Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1

    fillMode: Image.PreserveAspectCrop
    sourceSize: Qt.size(Math.ceil(width * _dpr), Math.ceil(height * _dpr))
    asynchronous: true
    smooth: true
}
