import QtQuick
import QtQuick.Shapes
import Manderrow

// The game selection title: text filled with linear-gradient(180deg, primary-60 20%, primary-40),
// between two fading rules. The text is drawn as vector paths, so it stays smooth at any scale.
Item {
    id: title

    property string text

    readonly property real fontSize: 4 * Theme.rem

    implicitWidth: textMetrics.advanceWidth
    implicitHeight: metrics.height

    FontMetrics {
        id: metrics
        font.family: Theme.font
        font.pixelSize: title.fontSize
        font.weight: 650
        font.variableAxes: ({ "wght": 650 })
    }

    TextMetrics {
        id: textMetrics
        font: metrics.font
        text: title.text
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeWidth: -1
            strokeColor: "transparent"
            // variable font glyphs are built from overlapping contours
            fillRule: ShapePath.WindingFill
            fillGradient: LinearGradient {
                x1: 0
                y1: 0
                x2: 0
                y2: title.height
                GradientStop {
                    position: 0.2
                    color: Theme.primary60
                }
                GradientStop {
                    position: 1
                    color: Theme.primary40
                }
            }

            PathText {
                id: label
                // PathText puts the top left of the glyphs' bounds here, so offset that from the
                // baseline to line the text up as a Text would
                x: textMetrics.tightBoundingRect.x
                y: metrics.ascent + textMetrics.tightBoundingRect.y
                text: title.text
                font: metrics.font
            }
        }
    }

    component Rule: Rectangle {
        property bool leading
        width: title.width * 0.2
        height: 2
        anchors.verticalCenter: parent.verticalCenter
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop {
                position: 0
                color: leading ? "transparent" : Theme.neutral50
            }
            GradientStop {
                position: 1
                color: leading ? Theme.neutral50 : "transparent"
            }
        }
    }

    Rule {
        leading: true
        x: -width - 0.5 * title.fontSize
    }
    Rule {
        leading: false
        x: title.width + 0.5 * title.fontSize
    }
}
