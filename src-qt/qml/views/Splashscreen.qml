import QtQuick
import QtQuick.Effects
import QtQuick.Shapes
import Manderrow

// Shown while the app loads, ported from Splashscreen.tsx.
Item {
    id: splash

    clip: true
    opacity: 0
    Component.onCompleted: opacity = 1

    Behavior on opacity {
        NumberAnimation {
            duration: 250
            easing.type: Easing.Bezier
            easing.bezierCurve: Theme.easeIn
        }
    }

    // radial-gradient(circle at top center, hsl(210 30% 24%), --clr-neutral-90)
    Shape {
        anchors.fill: parent

        ShapePath {
            strokeWidth: -1
            strokeColor: "transparent"
            fillGradient: RadialGradient {
                centerX: splash.width / 2
                centerY: 0
                centerRadius: Math.hypot(splash.width / 2, splash.height)
                focalX: centerX
                focalY: 0
                GradientStop {
                    position: 0
                    color: Theme.hsl(210, 30, 24)
                }
                GradientStop {
                    position: 1
                    color: Theme.neutral90
                }
            }
            PathRectangle {
                width: splash.width
                height: splash.height
            }
        }
    }

    // the faint, blurred logo in the top right
    Item {
        id: logoArea
        height: parent.height * 0.75
        width: height
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.rightMargin: width * 0.05
        anchors.topMargin: height * 0.05

        SharpImage {
            id: logo
            anchors.fill: parent
            source: "qrc:/assets/logo.svg"
            fillMode: Image.PreserveAspectFit
            visible: false
            layer.enabled: true
        }

        // mask-image: linear-gradient(45deg, transparent 0%, rgb(0 0 0 / 0.02) 45%, rgb(0 0 0 / 0.3))
        // MultiEffect maps mask alpha through smoothstep(0, 1, a) with the threshold and spread
        // below, so the stops are pre-inverted: smoothstep(0.36) = 0.3, smoothstep(0.085) = 0.02.
        Item {
            id: mask
            anchors.fill: parent
            visible: false
            layer.enabled: true

            Rectangle {
                anchors.centerIn: parent
                width: parent.width * Math.SQRT2
                height: width
                rotation: 45
                gradient: Gradient {
                    GradientStop {
                        position: 0
                        color: Qt.rgba(0, 0, 0, 0.36)
                    }
                    GradientStop {
                        position: 0.55
                        color: Qt.rgba(0, 0, 0, 0.085)
                    }
                    GradientStop {
                        position: 1
                        color: "transparent"
                    }
                }
            }
        }

        MultiEffect {
            anchors.fill: parent
            source: logo
            blurEnabled: true
            blur: 1
            // filter: blur(4px)
            blurMax: Theme.blurMax(4)
            maskEnabled: true
            maskSource: mask
            maskThresholdMin: 0.5
            maskSpreadAtMin: 1
        }
    }

    Column {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: -(0.5 * 1.5 * Theme.rem + Theme.titlebarHeight) / 2
        spacing: 0

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Manderrow"
            color: Theme.neutral05
            font.family: Theme.font
            font.pixelSize: 2.5 * 1.5 * Theme.rem
            font.weight: Font.DemiBold
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            topPadding: 2 * 0.9 * 1.5 * Theme.rem
            text: I18n.t("splashscreen.launching_msg")
            color: Theme.hsl(212, 18, 70)
            font.family: Theme.font
            font.pixelSize: 0.9 * 1.5 * Theme.rem
        }

        Item {
            width: 1
            height: 0.75 * 1.5 * Theme.rem
        }

        // the animated loading bar
        Rectangle {
            id: loader
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(16 * 1.5 * Theme.rem, splash.width - Theme.rem)
            height: 0.6 * 1.5 * Theme.rem
            radius: height / 2
            border.color: Theme.hsl(210, 11, 35)
            clip: true
            color: "transparent"

            Item {
                anchors.fill: parent
                anchors.margins: 1
                clip: true

                Row {
                    id: stripes
                    property real offset
                    x: -width / 2 + offset

                    NumberAnimation on offset {
                        from: 0
                        to: stripes.width / 2
                        duration: 850
                        loops: Animation.Infinite
                    }

                    Repeater {
                        model: 2
                        Rectangle {
                            width: loader.width * 2
                            height: loader.height
                            gradient: Gradient {
                                orientation: Gradient.Horizontal
                                GradientStop {
                                    position: 0
                                    color: Theme.primary50
                                }
                                GradientStop {
                                    position: 0.5
                                    color: Theme.accent5
                                }
                                GradientStop {
                                    position: 1
                                    color: Theme.primary50
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
