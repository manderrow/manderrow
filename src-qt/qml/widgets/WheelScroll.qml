import QtQuick
import Manderrow

// Scrolls a Flickable with the mouse wheel like the web UI did under WebKitGTK: each notch scrolls
// viewport^(2/3) pixels (GTK's step), smoothly. Touchpads scroll by their pixel deltas. Place it
// inside the Flickable; it takes over the Flickable's own wheel handling.
//
// The Flickable also becomes a panel that FocusNavigator scrolls with the keyboard, as browsers
// scroll the focused or last clicked scroll container.
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

    /// The range that the content can be scrolled through along an axis, as [min, max].
    function bounds(vertical) {
        const f = flickable;
        if (vertical)
            return [f.originY - f.topMargin, Math.max(f.originY - f.topMargin, f.originY + f.contentHeight + f.bottomMargin - f.height)];
        return [f.originX - f.leftMargin, Math.max(f.originX - f.leftMargin, f.originX + f.contentWidth + f.rightMargin - f.width)];
    }

    /// Whether the content can scroll by `delta` along an axis, or at all if `delta` is 0.
    function canScroll(delta, vertical) {
        const [min, max] = bounds(vertical);
        const position = vertical ? flickable.contentY : flickable.contentX;
        return delta > 0 ? position < max - 0.5 : delta < 0 ? position > min + 0.5 : max - min > 0.5;
    }

    /// Scrolls to a position along an axis, within the bounds.
    function scrollTo(position, animated, vertical = scroll.vertical) {
        const [min, max] = bounds(vertical);
        const property = vertical ? "contentY" : "contentX";
        const destination = Math.max(min, Math.min(max, position));
        if (animated) {
            _destination = destination;
            animation.property = property;
            animation.to = destination;
            animation.restart();
        } else {
            if (animation.running && animation.property === property)
                animation.stop();
            flickable[property] = destination;
        }
    }

    function scrollBy(delta, animated, vertical = scroll.vertical) {
        const property = vertical ? "contentY" : "contentX";
        const start = animation.running && animation.property === property ? _destination : flickable[property];
        scrollTo(start + delta, animated, vertical);
    }

    /// The distance an arrow key scrolls, as in WebKit.
    readonly property real lineStep: 40
    /// The distance Page Up/Down and Space scroll along an axis, as in WebKit: most of the
    /// viewport, keeping some of it in view.
    function pageStep(vertical) {
        const viewport = vertical ? flickable.height : flickable.width;
        return Math.max(viewport * 0.875, viewport - 40, 1);
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

    Component.onCompleted: Ui.scrollers.push(scroll)
    Component.onDestruction: {
        const i = Ui.scrollers.indexOf(scroll);
        if (i >= 0)
            Ui.scrollers.splice(i, 1);
    }

    readonly property NumberAnimation _animation: NumberAnimation {
        id: animation
        target: scroll.flickable
        duration: 150
        easing.type: Easing.OutCubic
    }
}
