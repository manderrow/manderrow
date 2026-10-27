import QtQuick
import Manderrow

// Scrolls a Flickable with the mouse wheel like the web UI did under WebKitGTK: each notch scrolls
// viewport^(2/3) pixels (GTK's step), smoothly. Touchpads scroll as far as they did there, and keep
// scrolling for a moment after the fingers lift, slowing down, as WebKit's kinetic scrolling does.
// Place it inside the Flickable; it takes over the Flickable's own wheel handling.
//
// The Flickable also becomes a panel that FocusNavigator scrolls with the keyboard, as browsers
// scroll the focused or last clicked scroll container.
WheelHandler {
    id: scroll

    /// The Flickable to scroll: by default, the nearest one this handler is inside of. (A handler
    /// declared inside a Flickable is attached to its content item.)
    property Flickable flickable: {
        let item = parent;
        while (item && !(item as Flickable))
            item = item.parent;
        return item as Flickable;
    }
    readonly property bool vertical: orientation === Qt.Vertical
    /// Whether this is scrolling the Flickable, which the Flickable's `moving` doesn't say, as that
    /// is only of its own movement, for telling the user's scrolling from the view's own changes.
    readonly property bool scrolling: animation.running || kinetic.running || _jumping
    property bool _jumping: false
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
        kinetic.stop();
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
            _jumping = true;
            flickable[property] = destination;
            _jumping = false;
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

    /// Scrolls for a wheel event. A horizontal scroller also follows vertical wheels, as browsers
    /// do with Shift held, when given one by a handler that takes those.
    function scrollFor(event) {
        const sideways = !vertical && event.angleDelta.x === 0 && event.pixelDelta.x === 0;
        const pixels = vertical || sideways ? event.pixelDelta.y : event.pixelDelta.x;
        const angle = vertical || sideways ? event.angleDelta.y : event.angleDelta.x;
        const touchpad = _gtkTouchpad ? _fromTouchpad(event, pixels, angle) : pixels !== 0 && event.device.type === PointerDevice.TouchPad;
        console.debug(log, "wheel event from", event.device.name, "type", event.device.type, "phase", event.phase, "angle", angle, "pixels", pixels, "touchpad", touchpad);
        if (touchpad && _gtkTouchpad) {
            if (event.phase === Qt.ScrollEnd) {
                _fling();
            } else if (angle !== 0) {
                // WebKitGTK scrolls by GDK's deltas times 40 pixels (pixelsPerLineStep), and GDK's
                // touchpad deltas are Wayland's axis values / 10, or X11's valuator values / their
                // increment. Qt's angle deltas are those times 120, rather than the pixel deltas,
                // which are Wayland's axis values themselves and which X11 doesn't have.
                const delta = -angle / 3;
                scrollBy(delta, false);
                _record(delta);
            }
        } else if (pixels !== 0 && touchpad) {
            scrollBy(-pixels, false);
        } else if (angle !== 0) {
            const viewport = vertical ? flickable.height : flickable.width;
            scrollBy(-angle / 120 * Math.pow(Math.max(viewport, 1), 2 / 3), true);
        }
    }

    // Kinetic scrolling, as in WebKit's ScrollAnimationKinetic: when the fingers lift, the content
    // keeps moving at the touchpad's velocity over the last 150 ms, which decays by e^(-4t). A fling
    // in the direction of one still going adds to its velocity. macOS does this itself, sending
    // momentum events, and Windows does for precision touchpads.

    /// Whether touchpads scroll as WebKitGTK did, on Linux and the BSDs.
    readonly property bool _gtkTouchpad: Qt.platform.os !== "osx" && Qt.platform.os !== "windows"
    /// When the last of a touchpad's scrolls came, in ms.
    property real _touchpadAt: 0

    /// Whether a wheel event on Linux came from a touchpad, which Qt doesn't say on Wayland, and
    /// says of some mice on X11. Anything else is a wheel, so that a mouse never loses its animation.
    function _fromTouchpad(event, pixels, angle) {
        const now = Date.now();
        // a scroll that comes within one of the touchpad's frames of its last one
        const ongoing = now - _touchpadAt < 100;
        const touchpad =
            // Wayland gives touchpads (and other continuous scrollers) pixel deltas, but never wheels,
            // as X11 does touchpads whose driver isn't libinput
            pixels !== 0
            // Wayland's touchpad scrolls begin and end, with phases between, which wheels also have
            // after Qt misses a touchpad scroll's end (if the pointer had left the window). A
            // touchpad's smallest scrolls round to no pixels, but come amid its others.
            || event.phase === Qt.ScrollBegin || event.phase === Qt.ScrollEnd
            || (event.phase === Qt.ScrollUpdate && ongoing)
            // X11's touchpads with libinput, which scroll by fractions of a notch, unlike the mice
            // that Qt takes for touchpads, such as those whose driver is evdev
            || (event.device.type === PointerDevice.TouchPad && (angle % 120 !== 0 || ongoing));
        if (touchpad)
            _touchpadAt = now;
        return touchpad;
    }

    readonly property LoggingCategory _log: LoggingCategory {
        id: log
        name: "manderrow.scroll"
        defaultLogLevel: LoggingCategory.Warning
    }
    readonly property real _friction: 4
    /// The touchpad's recent scrolls along this axis, as [time in ms, delta] pairs.
    property var _history: []
    property real _flingStart: 0
    property real _flingFrom: 0
    property real _flingVelocity: 0
    /// Where the fling last put the content, to notice anything else scrolling it.
    property real _flingAt: 0
    property string _flingProperty: "contentY"

    function _record(delta) {
        const now = Date.now();
        const history = _history.filter(([time]) => now - time <= 150);
        history.push([now, delta]);
        _history = history;
    }

    function _fling() {
        const now = Date.now();
        const history = _history.filter(([time]) => now - time <= 150);
        _history = [];
        // the end of the gesture counts toward its duration, as WebKit adds it to the history
        const span = history.length > 0 ? (now - history[0][0]) / 1000 : 0;
        if (span <= 0)
            return;
        let velocity = history.reduce((sum, [, delta]) => sum + delta, 0) / span;
        // what remains of the last fling's velocity, if it hadn't been stopped
        const previous = _flingVelocity * Math.exp(-_friction * (now - _flingStart) / 1000);
        if (previous !== 0 && Math.sign(previous) === Math.sign(velocity) && Math.abs(velocity) >= Math.abs(previous * 0.33)) {
            const multiplier = (velocity - previous * 0.33) / (previous - previous * 0.33);
            velocity += previous * Math.min(multiplier, 6);
        }
        _flingProperty = vertical ? "contentY" : "contentX";
        _flingFrom = flickable[_flingProperty];
        _flingAt = _flingFrom;
        _flingVelocity = velocity;
        _flingStart = now;
        kinetic.restart();
    }

    readonly property FrameAnimation _kinetic: FrameAnimation {
        id: kinetic

        onTriggered: {
            const f = scroll.flickable;
            const property = scroll._flingProperty;
            // something else scrolled the content
            if (!f || Math.abs(f[property] - scroll._flingAt) > 0.5) {
                stop();
                return;
            }
            const decay = Math.exp(-scroll._friction * elapsedTime);
            const [min, max] = scroll.bounds(property === "contentY");
            const position = scroll._flingFrom + scroll._flingVelocity / scroll._friction * (1 - decay);
            const clamped = Math.max(min, Math.min(max, position));
            f[property] = clamped;
            scroll._flingAt = f[property];
            if (clamped !== position || Math.abs(scroll._flingVelocity * decay) < 1)
                stop();
        }
    }

    onWheel: event => scrollFor(event)

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
