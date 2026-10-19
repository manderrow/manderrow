import QtQuick
import QtQuick.Shapes
import Manderrow

// A Font Awesome icon, sized like solid-fa: 1em tall, with the icon's own aspect ratio.
Item {
    id: root

    /// The icon name, such as "xmark", or "regular:star" for the regular style.
    property string name
    property color color: Theme.neutral05
    /// The font size the icon is drawn at.
    property real size: Theme.rem

    readonly property var _icon: Icons.icons[name]

    implicitHeight: size
    implicitWidth: _icon ? size * _icon.w / _icon.h : 0

    Shape {
        anchors.fill: parent
        visible: !!root._icon
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            fillColor: root.color
            strokeWidth: -1
            strokeColor: "transparent"
            scale: root._icon ? Qt.size(root.width / root._icon.w, root.height / root._icon.h) : Qt.size(1, 1)

            PathSvg {
                path: root._icon ? root._icon.d : ""
            }
        }
    }
}
