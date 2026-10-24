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
MouseArea {
    id: area

    /// The item whose TextEdits are selected.
    required property Item content
    /// Put between the text of each block when they are copied.
    property string separator: "\n\n"
    /// The color of the links in the text, which hovering one turns into `hoveredLinkColor`.
    property color linkColor: Theme.primary60
    property color hoveredLinkColor: Theme.primary65

    /// The TextEdits being selected, in reading order.
    property list<TextEdit> _items
    /// The ends of the selection, as `{ index, position }` in `_items`.
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
    }

    function _collectIfChanged() {
        if (content.width !== _collectedSize.width || content.height !== _collectedSize.height)
            _collect();
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

    /// The position in the text nearest a point in scene coordinates. Between blocks, it is the end
    /// of the one above.
    function _locate(scenePoint) {
        for (let i = 0; i < _items.length; i++) {
            const item = _items[i];
            const p = item.mapFromItem(null, scenePoint.x, scenePoint.y);
            if (p.y < 0)
                return i === 0 ? { index: 0, position: 0 } : { index: i - 1, position: _items[i - 1].length };
            if (p.y < item.height)
                return { index: i, position: item.positionAt(p.x, p.y) };
        }
        const last = _items.length - 1;
        return { index: last, position: _items[last].length };
    }

    function _apply() {
        const backwards = _anchor && (_focus.index < _anchor.index || (_focus.index === _anchor.index && _focus.position < _anchor.position));
        const start = backwards ? _focus : _anchor;
        const end = backwards ? _anchor : _focus;
        _items.forEach((item, i) => {
            if (!start || i < start.index || i > end.index)
                item.deselect();
            else
                item.select(i === start.index ? start.position : 0, i === end.index ? end.position : item.length);
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
        if (_items.length === 0)
            return;
        _anchor = { index: 0, position: 0 };
        _focus = { index: _items.length - 1, position: _items[_items.length - 1].length };
        _apply();
    }

    function selectedText() {
        // a TextEdit's own selectedText runs the cells of rich text's tables together, and keeps
        // its line separators and non-breaking spaces
        return _items.filter(item => item.selectedText !== "").map(item => Native.selectedText(item.textDocument, item.selectionStart, item.selectionEnd)).filter(text => text !== "").join(separator);
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
            _anchor = { index: at.index, position: item.selectionStart };
            _focus = { index: at.index, position: item.selectionEnd };
        } else if (_clicks === 3) {
            _anchor = { index: at.index, position: 0 };
            _focus = { index: at.index, position: item.length };
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
            Util.openUrl(_link);
        _link = "";
    }

    // scrolls while the pointer is dragged past the top or bottom of the view
    Timer {
        readonly property real overshoot: {
            const scroller = area._scroller;
            if (!area.pressed || !area._dragged || !scroller)
                return 0;
            const y = scroller.flickable.mapFromItem(null, area._scenePoint).y;
            return y < 0 ? y : Math.max(0, y - scroller.flickable.height);
        }

        running: overshoot !== 0
        repeat: true
        interval: 16
        onTriggered: {
            area._scroller.scrollBy(Math.sign(overshoot) * Math.min(Math.abs(overshoot), 100) / 4, false);
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
