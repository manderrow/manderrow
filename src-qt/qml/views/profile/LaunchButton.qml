import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import Manderrow

// The "Launch profile" button, with the gradient border, sliding arrow, and glint of the web UI's
// `button[data-launch]`.
AbstractButton {
    id: launch

    /// The mini sidebar shows only the icon, without the button's decoration.
    property bool mini: false

    readonly property bool hot: enabled && (hovered || visualFocus)
    readonly property real borderWidth: mini ? 0 : 2
    // the start and end of `linear-gradient(20deg, ...)` across this button
    readonly property real _len: width * Math.sin(20 * Math.PI / 180) + height * Math.cos(20 * Math.PI / 180)
    readonly property real _dx: Math.sin(20 * Math.PI / 180) * _len / 2
    readonly property real _dy: Math.cos(20 * Math.PI / 180) * _len / 2

    hoverEnabled: true
    opacity: enabled ? 1 : 0.5
    padding: 0.5 * Theme.rem
    font.family: Theme.font
    font.pixelSize: mini ? 1.2 * Theme.rem : Theme.rem

    HoverHandler {
        cursorShape: launch.enabled ? Qt.PointingHandCursor : Qt.ForbiddenCursor
    }

    background: Item {
        visible: !launch.mini
        clip: true

        // the border: radial-gradient(circle at center, primary-20, primary-60, primary-30),
        // which doubles in size on hover
        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeWidth: -1
                strokeColor: "transparent"
                fillGradient: RadialGradient {
                    property real size: launch.hot ? 2 : 1
                    Behavior on size {
                        NumberAnimation {
                            duration: 1000
                        }
                    }
                    centerX: launch.width / 2
                    centerY: launch.height / 2
                    centerRadius: Math.hypot(launch.width, launch.height) / 2 * size
                    focalX: centerX
                    focalY: centerY
                    GradientStop {
                        position: 0
                        color: Theme.primary20
                    }
                    GradientStop {
                        position: 0.5
                        color: launch.down ? Theme.primary55 : Theme.primary60
                    }
                    GradientStop {
                        position: 1
                        color: launch.down ? Theme.primary25 : Theme.primary30
                    }
                }
                PathRectangle {
                    width: launch.width
                    height: launch.height
                    radius: launch.height / 2
                }
            }
        }

        // the fill: linear-gradient(20deg, primary-30, primary-50)
        Shape {
            anchors.fill: parent
            anchors.margins: launch.borderWidth
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeWidth: -1
                strokeColor: "transparent"
                fillGradient: LinearGradient {
                    x1: launch.width / 2 - launch._dx
                    y1: launch.height / 2 + launch._dy
                    x2: launch.width / 2 + launch._dx
                    y2: launch.height / 2 - launch._dy
                    GradientStop {
                        position: 0
                        color: launch.down ? Theme.primary25 : Theme.primary30
                    }
                    GradientStop {
                        position: 1
                        color: launch.down ? Theme.primary45 : Theme.primary50
                    }
                }
                PathRectangle {
                    width: launch.width - 2 * launch.borderWidth
                    height: launch.height - 2 * launch.borderWidth
                    radius: height / 2
                }
            }
        }

        // the glint that sweeps across on hover
        Shape {
            id: glint
            width: launch.width + 2 * launch.borderWidth
            height: width
            y: (launch.height - height) / 2
            x: launch.hot ? 0.05 * width - width / 2 + launch.width / 2 : -1.05 * width + launch.width / 2
            rotation: 30
            opacity: launch.hot ? 1 : 0

            Behavior on x {
                enabled: launch.hot
                NumberAnimation {
                    duration: 1300
                    easing.type: Easing.InOutQuad
                }
            }
            Behavior on opacity {
                enabled: launch.hot
                NumberAnimation {
                    duration: 750
                }
            }

            ShapePath {
                id: glintPath
                readonly property color glintColor: launch.down ? Qt.rgba(1, 1, 1, 0.15) : Qt.rgba(1, 1, 1, 0.3)
                strokeWidth: -1
                strokeColor: "transparent"
                fillGradient: ConicalGradient {
                    centerX: glint.width / 2
                    centerY: glint.height / 2
                    angle: 90
                    GradientStop {
                        position: 0
                        color: glintPath.glintColor
                    }
                    GradientStop {
                        position: 20 / 360
                        color: "transparent"
                    }
                    GradientStop {
                        position: 160 / 360
                        color: "transparent"
                    }
                    GradientStop {
                        position: 0.5
                        color: glintPath.glintColor
                    }
                    GradientStop {
                        position: 200 / 360
                        color: "transparent"
                    }
                    GradientStop {
                        position: 340 / 360
                        color: "transparent"
                    }
                    GradientStop {
                        position: 1
                        color: glintPath.glintColor
                    }
                }
                PathRectangle {
                    width: glint.width
                    height: glint.height
                }
            }
        }
    }

    contentItem: Item {
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight

        Row {
            id: row
            anchors.centerIn: parent
            spacing: 0.35 * Theme.rem

            FaIcon {
                name: "circlePlay"
                size: launch.font.pixelSize
                anchors.verticalCenter: parent.verticalCenter
                // nudges left while pressed
                transform: Translate {
                    x: launch.down && !launch.mini ? -0.5 * launch.font.pixelSize : 0
                    Behavior on x {
                        NumberAnimation {
                            duration: 200
                        }
                    }
                }
            }

            Text {
                visible: !launch.mini
                text: I18n.t("profile.sidebar.launch_modded_btn")
                color: Theme.neutral05
                font: launch.font
                anchors.verticalCenter: parent.verticalCenter
            }

            // the arrow slides in on hover
            Item {
                visible: !launch.mini
                width: launch.hot ? arrow.width : 0
                height: arrow.height
                clip: true
                anchors.verticalCenter: parent.verticalCenter

                Behavior on width {
                    NumberAnimation {
                        duration: 300
                    }
                }

                FaIcon {
                    id: arrow
                    name: "arrowRightLong"
                    size: launch.font.pixelSize
                }
            }
        }
    }
}
