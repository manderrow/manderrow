import QtQuick
import Manderrow

// Scrolls a Flickable with the mouse wheel like the web UI did under WebKitGTK: each notch scrolls
// viewport^(2/3) pixels (GTK's step), smoothly. Touchpads scroll by their pixel deltas. Place it
// inside the Flickable; it takes over the Flickable's own wheel handling.
WheelHandler {
    id: scroll

    /// The Flickable to scroll: by default, the nearest one this handler is inside of. (A handler
    /// declared inside a Flickable is attached to its content item.)
    property Flickable flickable: {
        let item = parent;
        while (item && item.contentY === undefined)
            item = item.parent;
        return item;
    }
    readonly property bool vertical: orientation === Qt.Vertical
    property real _destination: 0

    target: null
    orientation: Qt.Vertical
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad

    function _bounds() {
        const f = flickable;
        if (vertical)
            return [f.originY - f.topMargin, Math.max(f.originY - f.topMargin, f.originY + f.contentHeight + f.bottomMargin - f.height)];
        return [f.originX - f.leftMargin, Math.max(f.originX - f.leftMargin, f.originX + f.contentWidth + f.rightMargin - f.width)];
    }

    function scrollBy(delta, animated) {
        const [min, max] = _bounds();
        const property = vertical ? "contentY" : "contentX";
        const start = animation.running ? _destination : flickable[property];
        _destination = Math.max(min, Math.min(max, start + delta));
        if (animated) {
            animation.property = property;
            animation.to = _destination;
            animation.restart();
        } else {
            animation.stop();
            flickable[property] = _destination;
        }
    }

    onWheel: event => {
        const pixels = vertical ? event.pixelDelta.y : event.pixelDelta.x;
        // a horizontal scroller also follows vertical wheels, as browsers do
        const angle = vertical ? event.angleDelta.y : (event.angleDelta.x !== 0 ? event.angleDelta.x : event.angleDelta.y);
        if (pixels !== 0 && event.device.type === PointerDevice.TouchPad) {
            scrollBy(-pixels, false);
        } else if (angle !== 0) {
            const viewport = vertical ? flickable.height : flickable.width;
            scrollBy(-angle / 120 * Math.pow(Math.max(viewport, 1), 2 / 3), true);
        }
    }

    // the wheel is handled here, and the mouse doesn't drag the content, as on the web
    onFlickableChanged: if (flickable)
        flickable.interactive = false


    readonly property NumberAnimation _animation: NumberAnimation {
        id: animation
        target: scroll.flickable
        duration: 150
        easing.type: Easing.OutCubic
    }
}
