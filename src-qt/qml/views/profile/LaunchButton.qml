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
    // the border adds to the size, as it does on the web
    padding: 0.5 * Theme.rem + borderWidth
    font.family: Theme.font
    font.pixelSize: mini ? 1.2 * Theme.rem : Theme.rem

    HoverHandler {
        cursorShape: launch.enabled ? Qt.PointingHandCursor : Qt.ForbiddenCursor
    }

    // How far the glint has swept across, and its opacity. The transition is only declared on
    // :hover, so leaving snaps them back.
    property real _glint: 0
    property real _glintOpacity: 0

    onHotChanged: {
        if (hot) {
            glintIn.restart();
        } else {
            glintIn.stop();
            _glint = 0;
            _glintOpacity = 0;
        }
    }

    ParallelAnimation {
        id: glintIn
        SmoothNumberAnimation {
            target: launch
            property: "_glint"
            from: 0
            to: 1
            duration: 1300
        }
        SmoothNumberAnimation {
            target: launch
            property: "_glintOpacity"
            from: 0
            to: 1
            duration: 750
        }
    }

    background: Item {

        // the border: radial-gradient(circle at center, primary-20, primary-60, primary-30),
        // which doubles in size on hover
        Shape {
            visible: !launch.mini
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeWidth: -1
                strokeColor: "transparent"
                fillGradient: RadialGradient {
                    property real size: launch.hot ? 2 : 1
                    Behavior on size {
                        SmoothNumberAnimation {
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
            visible: !launch.mini
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

        // the glint that sweeps across on hover. It is the button's inner shape filled with the
        // conic gradient, which clips it to the rounded corners like `overflow: hidden`.
        Shape {
            anchors.fill: parent
            anchors.margins: launch.borderWidth
            preferredRendererType: Shape.CurveRenderer
            opacity: launch._glintOpacity

            ShapePath {
                id: glintPath

                // The ::before square is the button's width plus the border, centred on the
                // button and translated from -105% to 5% of its size.
                readonly property real square: launch.width + 2 * launch.borderWidth
                readonly property real centerX: launch.width / 2 + (1.1 * launch._glint - 0.55) * square
                readonly property color glintColor: launch.down ? Qt.rgba(1, 1, 1, 0.15) : Qt.rgba(1, 1, 1, 0.3)

                strokeWidth: -1
                strokeColor: "transparent"
                fillGradient: ConicalGradient {
                    // in the inner shape's coordinates
                    centerX: glintPath.centerX - launch.borderWidth
                    centerY: launch.height / 2 - launch.borderWidth
                    // CSS's conic gradient starts at the top; Qt's starts at 3 o'clock and runs
                    // the other way. The ::before is rotated by 30deg.
                    angle: 90 - 30
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
                    width: launch.width - 2 * launch.borderWidth
                    height: launch.height - 2 * launch.borderWidth
                    radius: height / 2
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
            // the contents shift as the arrow slides in, which snapping to whole pixels makes jerky
            anchors.alignWhenCentered: false
            spacing: 0.35 * Theme.rem

            FaIcon {
                name: "circlePlay"
                size: launch.font.pixelSize
                anchors.verticalCenter: parent.verticalCenter
                // nudges left while pressed
                transform: Translate {
                    x: launch.down && !launch.mini ? -0.5 * launch.font.pixelSize : 0
                    Behavior on x {
                        SmoothNumberAnimation {
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

            // the arrow slides in on hover, as its max-width transitions from 0 to 3ch
            Item {
                visible: !launch.mini
                property real maxWidth: launch.hot ? 3 * Ui.ch : 0
                width: Math.min(arrow.width, maxWidth)
                height: arrow.height
                clip: true
                anchors.verticalCenter: parent.verticalCenter

                Behavior on maxWidth {
                    SmoothNumberAnimation {
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
