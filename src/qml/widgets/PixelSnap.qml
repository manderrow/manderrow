import QtQuick
import Manderrow

// Moves its parent's edges onto whole device pixels where the parent is drawn, as browsers snap
// boxes when they paint them. Otherwise a rounded (so antialiased) rectangle at a fractional
// position smears its straight edges, and its border, across two rows of pixels.
//
// It transforms the parent rather than moving it, so that the parent can still be anchored or
// managed by a layout or control, and the snapping can't change the layout. The transform scales
// the parent by up to a device pixel, so only snap items without text inside: text is drawn
// blurrier under a scale. Main.qml snaps every PixelSnap before each frame is drawn, once
// animations and layouts have moved them.
Item {
    id: snap

    readonly property Item target: parent
    readonly property Scale _scale: Scale {}
    readonly property Translate _translate: Translate {}

    visible: false

    function update() {
        const t = target;
        if (!t || !t.visible || !t.parent)
            return;
        // the corners in device pixels
        const d = Ui.dpr;
        const a = t.parent.mapToItem(null, t.x, t.y);
        const b = t.parent.mapToItem(null, t.x + t.width, t.y + t.height);
        const left = a.x * d, top = a.y * d, right = b.x * d, bottom = b.y * d;
        if (right <= left || bottom <= top)
            return;
        const snappedLeft = Math.round(left), snappedTop = Math.round(top);
        // device pixels per unit of the parent's coordinates, which ancestors may scale
        const kx = (right - left) / t.width, ky = (bottom - top) / t.height;
        _scale.xScale = (Math.round(right) - snappedLeft) / (right - left);
        _scale.yScale = (Math.round(bottom) - snappedTop) / (bottom - top);
        _translate.x = (snappedLeft - left) / kx;
        _translate.y = (snappedTop - top) / ky;
    }

    Component.onCompleted: {
        target.transform = [_scale, _translate];
        Ui.pixelSnaps.push(snap);
    }
    Component.onDestruction: {
        const i = Ui.pixelSnaps.indexOf(snap);
        if (i >= 0)
            Ui.pixelSnaps.splice(i, 1);
    }
}
