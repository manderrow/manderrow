import QtQuick
import Manderrow

// Selects text across the TextEdits in an item, as a browser selects across a page's paragraphs:
// dragging from one to another selects everything between them, and Ctrl+C copies it. Ctrl+A
// selects all of it, a double click a word, and a triple click a whole block. Links in the text
// open when clicked, and are underlined and lightened while hovered, as on the web. Dragging past
// the edge of the view around it scrolls it.
//
// It selects the read-only TextEdits whose `selectByMouse` is off, which leaves their presses, and
// their cursor, to it. Declare it after the content, over it: it is only over the text, so that
// elsewhere, as on images and buttons, the pointer is for what is under it.
//
// The content can be a view that creates its blocks only while they are in view, as a ListView
// does its delegates, with `placeOf`, `textBetween`, and `lastPlace`, so that the selection spans
// blocks that have gone out of view, or haven't been created yet.
MouseArea {
    id: area

    /// The item whose TextEdits are selected.
    required property Item content
    /// Put between the text of each block when they are copied.
    property string separator: "\n\n"
    /// For a view that creates its blocks only while they are in view: a function that gives the
    /// place of a block in the content, which numbers the blocks in reading order, so that the
    /// selection doesn't depend on which of them there are.
    property var placeOf: null
    /// With `placeOf`, a function that gives the text from one end of the selection to the other,
    /// as `{ place, position }`, start first, for copying blocks that aren't there. The end's
    /// position can be past the end of its block's text.
    property var textBetween: null
    /// With `placeOf`, the place of the last block, which Ctrl+A selects to the end of.
    property int lastPlace: -1
    /// The color of the links in the text, which hovering one turns into `hoveredLinkColor`.
    property color linkColor: Theme.primary60
    property color hoveredLinkColor: Theme.primary65
    /// A function that opens a link clicked in the text, which otherwise opens externally.
    property var openLink: url => Util.openUrl(url)

    /// The TextEdits being selected, in reading order.
    property list<TextEdit> _items
    /// The ends of the selection, as `{ place, position }`. A block's place is its index in
    /// `_items`, unless `placeOf` gives it.
    property var _anchor: null
    property var _focus: null
    property point _pressPoint
    property real _lastPress: 0
    property int _clicks: 0
    property bool _dragged: false
    property string _link: ""
    /// The link under the pointer while hovering.
    property string _hoveredLink: ""
    /// The TextEdit whose link is styled as hovered.
    property TextEdit _hoveredEdit: null
    /// The size of the content when the TextEdits were collected, which changes as its blocks are
    /// created and its images load.
    property size _collectedSize
    /// Whether the view's blocks have been created or destroyed since they were collected.
    property bool _stale: false
    /// Where the pointer is while dragging, in scene coordinates, so that the selection follows it
    /// as the view scrolls.
    property point _scenePoint
    property WheelScroll _scroller: null

    anchors.fill: parent
    preventStealing: true
    hoverEnabled: true
    // a MouseArea always has a cursor, which is why it is only over the text
    cursorShape: !pressed && _hoveredLink !== "" ? Qt.PointingHandCursor : Qt.IBeamCursor
    containmentMask: QtObject {
        function contains(point: point): bool {
            if (area.pressed)
                return true;
            area._collectIfChanged();
            return area._itemAt(area.mapToItem(null, point.x, point.y)) !== null;
        }
    }

    function _collect() {
        const found = [];
        const walk = item => {
            if (!item.visible)
                return;
            const edit = item as TextEdit;
            if (edit && edit.readOnly && !edit.selectByMouse)
                found.push({ edit, top: edit.mapToItem(area, 0, 0) });
            for (const child of item.children)
                walk(child);
        };
        walk(content);
        found.sort((a, b) => a.top.y - b.top.y || a.top.x - b.top.x);
        _items = found.map(entry => entry.edit);
        _collectedSize = Qt.size(content.width, content.height);
        _stale = false;
    }

    function _collectIfChanged() {
        if (_stale || content.width !== _collectedSize.width || content.height !== _collectedSize.height)
            _collect();
    }

    function _placeOf(i) {
        return placeOf ? placeOf(_items[i]) : i;
    }

    // the blocks that a view creates as they come into view are selected as they are created
    Connections {
        target: area.placeOf ? area.content : null
        function onChildrenChanged() {
            area._stale = true;
            Qt.callLater(area._refresh);
        }
    }

    function _refresh() {
        _collect();
        _apply();
    }

    /// The TextEdit under a point in scene coordinates.
    function _itemAt(scenePoint) {
        for (const item of _items) {
            // blocks can be destroyed when their content changes
            if (!item)
                continue;
            const p = item.mapFromItem(null, scenePoint.x, scenePoint.y);
            if (item.contains(p))
                return item;
        }
        return null;
    }

    /// The position in the text nearest a point in scene coordinates: in the block beside it, of
    /// those level with it, or between rows of blocks, at the end of the one before.
    function _locate(scenePoint) {
        let before = -1;
        let nearest = -1;
        let nearestDistance = Infinity;
        let nearestPoint = null;
        for (let i = 0; i < _items.length; i++) {
            const item = _items[i];
            if (!item)
                continue;
            const p = item.mapFromItem(null, scenePoint.x, scenePoint.y);
            // as are the blocks after it, which are sorted by their tops
            if (p.y < 0)
                break;
            if (p.y >= item.height) {
                before = i;
                continue;
            }
            const distance = p.x < 0 ? -p.x : Math.max(0, p.x - item.width);
            if (distance < nearestDistance) {
                nearest = i;
                nearestDistance = distance;
                nearestPoint = p;
            }
        }
        if (nearest >= 0)
            return { place: _placeOf(nearest), position: _items[nearest].positionAt(nearestPoint.x, nearestPoint.y) };
        if (before >= 0)
            return { place: _placeOf(before), position: _items[before].length };
        const first = _items.findIndex(item => item);
        return first >= 0 ? { place: _placeOf(first), position: 0 } : _focus;
    }

    /// The ends of the selection, start first.
    function _ends() {
        const backwards = _anchor && (_focus.place < _anchor.place || (_focus.place === _anchor.place && _focus.position < _anchor.position));
        return backwards ? [_focus, _anchor] : [_anchor, _focus];
    }

    function _apply() {
        const [start, end] = _ends();
        _items.forEach((item, i) => {
            if (!item)
                return;
            const place = _placeOf(i);
            if (!start || place < start.place || place > end.place)
                item.deselect();
            else
                item.select(place === start.place ? Math.min(start.position, item.length) : 0, place === end.place ? Math.min(end.position, item.length) : item.length);
        });
    }

    /// Styles the link under a point in scene coordinates as hovered, and no other.
    function _hover(scenePoint) {
        const item = _itemAt(scenePoint);
        const local = item ? item.mapFromItem(null, scenePoint.x, scenePoint.y) : null;
        const link = item ? item.linkAt(local.x, local.y) : "";
        if (_hoveredEdit && _hoveredEdit !== item)
            Native.hoverLink(_hoveredEdit, -1, "", linkColor, hoveredLinkColor);
        if (link !== "" || (item && item === _hoveredEdit))
            Native.hoverLink(item, item.positionAt(local.x, local.y), link, linkColor, hoveredLinkColor);
        _hoveredEdit = link !== "" ? item : null;
        _hoveredLink = link;
    }

    function clear() {
        _anchor = null;
        _focus = null;
        _apply();
    }

    function selectAll() {
        _collect();
        if (placeOf) {
            if (lastPlace < 0)
                return;
            _anchor = { place: 0, position: 0 };
            _focus = { place: lastPlace, position: Infinity };
        } else {
            if (_items.length === 0)
                return;
            _anchor = { place: 0, position: 0 };
            _focus = { place: _items.length - 1, position: _items[_items.length - 1].length };
        }
        _apply();
    }

    function selectedText() {
        if (placeOf)
            return _anchor ? textBetween(..._ends()) : "";
        // a TextEdit's own selectedText runs the cells of rich text's tables together, and keeps
        // its line separators and non-breaking spaces
        return _items.filter(item => item && item.selectedText !== "").map(item => Native.selectedText(item.textDocument, item.selectionStart, item.selectionEnd)).filter(text => text !== "").join(separator);
    }

    onPressed: mouse => {
        const scenePoint = mapToItem(null, mouse.x, mouse.y);
        const extending = (mouse.modifiers & Qt.ShiftModifier) && _anchor !== null;
        if (!extending)
            _collect();
        const item = _itemAt(scenePoint);
        if (!item && !extending) {
            // not text, so it is for what is under this
            mouse.accepted = false;
            return;
        }
        forceActiveFocus(Qt.MouseFocusReason);

        const now = Date.now();
        const repeated = now - _lastPress < Qt.styleHints.mouseDoubleClickInterval && Math.hypot(mouse.x - _pressPoint.x, mouse.y - _pressPoint.y) < Qt.styleHints.startDragDistance;
        _clicks = repeated ? _clicks % 3 + 1 : 1;
        _lastPress = now;
        _pressPoint = Qt.point(mouse.x, mouse.y);
        _scenePoint = scenePoint;
        _dragged = false;
        _scroller = Ui.focusNavigator ? Ui.focusNavigator.scrollerOf(area) : null;

        const local = item ? item.mapFromItem(null, scenePoint.x, scenePoint.y) : null;
        _link = item && _clicks === 1 && !extending ? item.linkAt(local.x, local.y) : "";
        const at = _locate(scenePoint);
        if (extending) {
            _focus = at;
        } else if (_clicks === 2) {
            item.cursorPosition = at.position;
            item.selectWord();
            _anchor = { place: at.place, position: item.selectionStart };
            _focus = { place: at.place, position: item.selectionEnd };
        } else if (_clicks === 3) {
            _anchor = { place: at.place, position: 0 };
            _focus = { place: at.place, position: item.length };
        } else {
            _anchor = at;
            _focus = at;
        }
        _apply();
    }

    onPositionChanged: mouse => {
        _hover(mapToItem(null, mouse.x, mouse.y));
        if (!pressed)
            return;
        if (!_dragged && Math.hypot(mouse.x - _pressPoint.x, mouse.y - _pressPoint.y) < Qt.styleHints.startDragDistance)
            return;
        _dragged = true;
        _scenePoint = mapToItem(null, mouse.x, mouse.y);
        // a double or triple click goes on by characters
        _focus = _locate(_scenePoint);
        _apply();
    }

    onExited: _hover(Qt.point(-Infinity, -Infinity))
    onReleased: {
        if (!_dragged && _link !== "")
            openLink(_link);
        _link = "";
    }

    // scrolls while the pointer is dragged past the edges of the view, as WebKit does: every 50 ms,
    // by as far as the pointer is past them, so that the farther it is, the faster it goes
    FrameAnimation {
        /// How far the pointer is past the view's edges.
        readonly property point overshoot: {
            const scroller = area._scroller;
            if (!area.pressed || !area._dragged || !scroller)
                return Qt.point(0, 0);
            const f = scroller.flickable;
            const p = f.mapFromItem(null, area._scenePoint);
            return Qt.point(p.x < 0 ? p.x : Math.max(0, p.x - f.width), p.y < 0 ? p.y : Math.max(0, p.y - f.height));
        }

        running: overshoot.x !== 0 || overshoot.y !== 0
        onTriggered: {
            const scale = frameTime * 1000 / 50;
            if (overshoot.y !== 0)
                area._scroller.scrollBy(overshoot.y * scale, false, true);
            if (overshoot.x !== 0)
                area._scroller.scrollBy(overshoot.x * scale, false, false);
            area._focus = area._locate(area._scenePoint);
            area._apply();
        }
    }

    Keys.onPressed: event => {
        if (event.matches(StandardKey.Copy) && _anchor !== null) {
            Native.copyText(selectedText());
        } else if (event.matches(StandardKey.SelectAll)) {
            selectAll();
        } else {
            event.accepted = false;
        }
    }

    // pressing anywhere else deselects, as on the web
    readonly property Item _activeFocusItem: Window.activeFocusItem
    on_ActiveFocusItemChanged: if (_activeFocusItem && _activeFocusItem !== area && _anchor !== null)
        clear()
}
