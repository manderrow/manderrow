import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import Manderrow

// The custom window title bar, ported from TitleBar.tsx.
Rectangle {
    id: bar

    required property var window
    /// The name of the current profile, shown in the middle of the bar.
    property string profileName: ""

    readonly property bool maximized: window.visibility === Window.Maximized || window.visibility === Window.FullScreen

    height: Theme.titlebarHeight
    color: window.active ? Theme.neutral90 : Theme.neutral85

    function toggleMaximized() {
        if (Qt.platform.os === "osx")
            window.visibility === Window.FullScreen ? window.showNormal() : window.showFullScreen();
        else
            maximized ? window.showNormal() : window.showMaximized();
    }

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: Theme.neutral80
    }

    // dragging the bar moves the window, and double clicking it maximizes the window
    TapHandler {
        acceptedButtons: Qt.LeftButton
        gesturePolicy: TapHandler.DragThreshold
        onDoubleTapped: bar.toggleMaximized()
    }
    DragHandler {
        target: null
        grabPermissions: PointerHandler.CanTakeOverFromAnything
        onActiveChanged: if (active)
            bar.window.startSystemMove()
    }

    RowLayout {
        anchors.left: parent.left
        anchors.leftMargin: 0.4 * Theme.rem
        anchors.verticalCenter: parent.verticalCenter
        spacing: 0.4 * Theme.rem

        SharpImage {
            source: "qrc:/assets/logo.svg"
            fillMode: Image.PreserveAspectFit
            Layout.preferredWidth: Theme.titlebarLogoSize
            Layout.preferredHeight: Theme.titlebarLogoSize
        }

        Text {
            text: "Manderrow"
            color: Theme.neutral20
            font.family: Theme.font
            font.pixelSize: 0.85 * Theme.rem
        }
    }

    Text {
        anchors.centerIn: parent
        width: Math.min(implicitWidth, parent.width / 2)
        text: bar.profileName
        elide: Text.ElideRight
        horizontalAlignment: Text.AlignHCenter
        color: Theme.neutral30
        font.family: Theme.font
        font.pixelSize: Theme.rem
    }

    component ControlButton: Item {
        id: control

        property string path
        property string tooltip
        property color hoverColor: Theme.neutral70
        signal clicked

        width: 24 + Theme.rem
        height: parent.height

        Rectangle {
            anchors.fill: parent
            color: hover.hovered ? control.hoverColor : Theme.alpha(control.hoverColor, 0)

            Behavior on color {
                ColorAnimation {
                    duration: 200
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.ease
                }
            }
        }

        // Material Design Icons, as in the web UI
        Shape {
            anchors.centerIn: parent
            width: 24
            height: 24
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                fillColor: Theme.neutral05
                strokeWidth: -1
                strokeColor: "transparent"
                PathSvg {
                    path: control.path
                }
            }
        }

        HoverHandler {
            id: hover
        }
        TapHandler {
            onTapped: control.clicked()
        }
        Tip {
            text: control.tooltip
            visible: hover.hovered
        }
    }

    Row {
        anchors.right: parent.right
        height: parent.height

        ControlButton {
            tooltip: I18n.t("titlebar.minimize_btn")
            path: "M19 13H5v-2h14z"
            onClicked: bar.window.showMinimized()
        }
        ControlButton {
            tooltip: bar.maximized ? I18n.t("titlebar.restore_btn") : I18n.t("titlebar.maximize_btn")
            path: bar.maximized ? "M4 8h4V4h12v12h-4v4H4zm12 0v6h2V6h-8v2zM6 12v6h8v-6z" : "M4 4h16v16H4zm2 4v10h12V8z"
            onClicked: bar.toggleMaximized()
        }
        ControlButton {
            tooltip: I18n.t("global.phrases.close")
            path: "M13.46 12L19 17.54V19h-1.46L12 13.46L6.46 19H5v-1.46L10.54 12L5 6.46V5h1.46L12 10.54L17.54 5H19v1.46z"
            hoverColor: Theme.danger5
            onClicked: bar.window.close()
        }
    }
}
