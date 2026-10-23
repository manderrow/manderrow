import QtQuick
import Manderrow

// An image with antialiased rounded corners, like `border-radius` with `overflow: hidden`. It
// covers its area, like `object-fit: cover`, at the display's pixel density.
Item {
    id: root

    property alias source: image.source
    property real radius: 0
    /// Generates mipmaps, which ImageEffects that blur this image need.
    property alias mipmap: image.mipmap
    readonly property alias status: image.status
    /// The image itself, for ImageEffects that draw it differently.
    readonly property alias image: image

    SharpImage {
        id: image
        anchors.fill: parent
        visible: false
    }

    ImageEffect {
        anchors.fill: parent
        image: image
        cornerRadius: root.radius
    }
}
