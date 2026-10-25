import QtQuick
import QtQuick.Shapes
import Manderrow

// Browsers' autoscrolling: pressing the middle button over something that scrolls marks the
// point, and the content then scrolls toward wherever the pointer is, the faster the farther it is
// from the point. Clicking the button leaves it scrolling until the next press of any button, or a
// key, as in Firefox and Chrome. Holding it down and moving the pointer scrolls until it is
// released.
//
// Place it over the content, filling it. It only takes the middle button, and only over a view
// that a WheelScroll scrolls and that has something to scroll.
MouseArea {
    id: autoscroll

    /// The scroller being scrolled, while autoscrolling.
    property WheelScroll scroller: null
    readonly property bool active: scroller !== null
    /// The axes along which the content scrolls.
    property bool vertical: false
    property bool horizontal: false
    /// Where the button was pressed, and where the pointer is, in this item's coordinates.
    property point origin
    property point pointer
    /// Whether the press that began the scrolling is still held, and whether the pointer has left
    /// the point since, in which case releasing it ends the scrolling.
    property bool _held: false
    property bool _moved: false

    /// How far from the point the pointer can be without scrolling, as in Firefox.
    readonly property real deadZone: 12

    anchors.fill: parent
    acceptedButtons: active ? Qt.AllButtons : Qt.MiddleButton
    hoverEnabled: active
    // A MouseArea has the arrow cursor unless it is unset, which would hide the cursors of
    // everything under this
    cursorShape: !active ? undefined : vertical && horizontal ? Qt.SizeAllCursor : vertical ? Qt.SizeVerCursor : Qt.SizeHorCursor

    /// The innermost of the views under a point in scene coordinates that has something to scroll.
    function scrollerAt(scenePoint) {
        let best = null;
        for (const s of Ui.scrollers) {
            const f = s.flickable;
            if (!f || !f.visible || !(s.canScroll(0, true) || s.canScroll(0, false)))
                continue;
            if (best && (f === best.flickable || !Ui.isWithin(f, best.flickable)))
                continue;
            // and not cut off by anything around it
            let shown = true;
            for (let item = f; item && shown; item = item.parent) {
                if (item === f || item.clip)
                    shown = item.contains(item.mapFromItem(null, scenePoint.x, scenePoint.y));
            }
            if (shown)
                best = s;
        }
        return best;
    }

    function stop() {
        scroller = null;
        _held = false;
    }

    /// The distance the content scrolls in 20 ms, for the pointer's distance from the point along
    /// an axis, as Firefox's autoscroll accelerates.
    function _speed(distance) {
        const steps = distance / deadZone;
        return steps > 1 ? steps * Math.sqrt(steps) - 1 : steps < -1 ? steps * Math.sqrt(-steps) + 1 : 0;
    }

    onPressed: mouse => {
        if (active) {
            // ends it, without reaching what is under the pointer, as in browsers
            stop();
            return;
        }
        const s = scrollerAt(mapToItem(null, mouse.x, mouse.y));
        if (!s) {
            mouse.accepted = false;
            return;
        }
        origin = Qt.point(mouse.x, mouse.y);
        pointer = origin;
        vertical = s.canScroll(0, true);
        horizontal = s.canScroll(0, false);
        _held = true;
        _moved = false;
        scroller = s;
    }
    onPositionChanged: mouse => {
        pointer = Qt.point(mouse.x, mouse.y);
        if (Math.hypot(pointer.x - origin.x, pointer.y - origin.y) > deadZone)
            _moved = true;
    }
    onReleased: {
        if (_held && _moved)
            stop();
        _held = false;
    }
    onCanceled: stop()

    Connections {
        target: Native
        enabled: autoscroll.active
        function onKeyPressed() {
            autoscroll.stop();
        }
    }
    Connections {
        target: Qt.application
        function onStateChanged() {
            if (Qt.application.state !== Qt.ApplicationActive)
                autoscroll.stop();
        }
    }

    FrameAnimation {
        running: autoscroll.active
        onTriggered: {
            const s = autoscroll.scroller;
            const scale = frameTime * 1000 / 20;
            const dy = autoscroll.vertical ? autoscroll._speed(autoscroll.pointer.y - autoscroll.origin.y) : 0;
            const dx = autoscroll.horizontal ? autoscroll._speed(autoscroll.pointer.x - autoscroll.origin.x) : 0;
            if (dy !== 0)
                s.scrollBy(dy * scale, false, true);
            if (dx !== 0)
                s.scrollBy(dx * scale, false, false);
        }
    }


    // the mark at the point, with arrows along the axes the content scrolls
    Rectangle {
        visible: autoscroll.active
        x: Math.round(autoscroll.origin.x - width / 2)
        y: Math.round(autoscroll.origin.y - height / 2)
        width: 28
        height: 28
        radius: width / 2
        color: Theme.neutral90
        border.width: Ui.hairline
        border.color: Theme.neutral50

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeWidth: -1
                fillColor: Theme.neutral10
                PathSvg {
                    path: {
                        let path = "M 12.5 14 a 1.5 1.5 0 1 0 3 0 a 1.5 1.5 0 1 0 -3 0 Z ";
                        if (autoscroll.vertical)
                            path += "M 14 4 L 18 9 L 10 9 Z M 14 24 L 18 19 L 10 19 Z ";
                        if (autoscroll.horizontal)
                            path += "M 4 14 L 9 10 L 9 18 Z M 24 14 L 19 10 L 19 18 Z ";
                        return path;
                    }
                }
            }
        }
    }
}
