import QtQuick
import Manderrow

// The window's keyboard navigation, which works like Discord's: the window is made of panels,
// Tab moves between them and the controls in them, and the arrow keys move within them.
//
// - Tab and Shift+Tab move through the window's controls in order, except that each FocusGroup,
//   a list or grid of items, is one stop: Tab enters it at its current item, moves through that
//   item's own controls, then leaves it. Qt's own Tab handling is replaced, since it moves on
//   from whichever item around the focused one takes the focus, not from the focused item.
// - In a FocusGroup, the arrow keys move between the items by where they are on screen, so lists
//   and grids work alike. Page Up/Down move by a page, and Home/End to the first and last item.
// - Elsewhere, the arrow keys, Page Up/Down, Home/End, and Space scroll the panel that holds the
//   focus or was last pressed in: the innermost view with a WheelScroll, or the panel around it
//   once it can't scroll any further that way.
// - Pressing outside the focused item takes the focus away from it, as clicking elsewhere does on
//   the web. Tab then moves on from where the pointer was pressed, and the arrow keys move on
//   from an item pressed in a FocusGroup.
// - Focus moved by the keyboard scrolls the newly focused item into view.
// - `moveFocusWithin` moves the focus through all of a small group's controls, as the arrow keys
//   do in menus and tab lists.
// - `Ui.focusVisible` tracks whether focus should be shown, as by `:focus-visible`. FocusRing
//   uses it.
//
// Keys other than Tab reach this through `handleKey`, which the window's content and its popups
// call for keys that their items leave unhandled.
QtObject {
    id: navigator

    required property Window window
    /// Holds the focus when nothing else does, so that keys still reach `handleKey`. It should
    /// pass its keys to `handleKey`.
    required property Item home

    /// Where Tab moves on from while `home` has the focus: the item last pressed.
    property Item startItem: null
    /// The WheelScroll of the panel that the keyboard scrolls.
    property WheelScroll scroller: null

    Component.onCompleted: Native.setTabHandler(forward => navigator.moveTab(forward))

    /// The window's layer that an item is in: its content, or the overlay of popups.
    function _layerOf(item) {
        let layer = item;
        while (layer.parent && layer.parent.parent)
            layer = layer.parent;
        return layer;
    }

    /// The innermost scrolling panel around an item.
    function scrollerOf(item) {
        let best = null;
        for (const s of Ui.scrollers) {
            if (s.flickable && s.flickable.visible && Ui.isWithin(item, s.flickable) && (!best || Ui.isWithin(s.flickable, best.flickable)))
                best = s;
        }
        return best;
    }

    function _parentScroller(scroller) {
        return scroller.flickable.parent ? scrollerOf(scroller.flickable.parent) : null;
    }

    /// The innermost FocusGroup holding an item.
    function groupOf(item) {
        let best = null;
        for (const g of Ui.focusGroups) {
            if (g.view && g.view.visible && g.holds(item) && (!best || Ui.isWithin(g.view, best.view)))
                best = g;
        }
        return best;
    }

    /// Scrolls the panel holding an item, and the panels around it, to bring it into view.
    function scrollIntoView(item) {
        for (let s = scrollerOf(item); s; s = _parentScroller(s)) {
            const f = s.flickable;
            const r = item.mapToItem(f.contentItem, 0, 0, item.width, item.height);
            for (const vertical of [true, false]) {
                const start = vertical ? r.y : r.x;
                const length = vertical ? r.height : r.width;
                const position = vertical ? f.contentY : f.contentX;
                const viewport = vertical ? f.height : f.width;
                if (start < position)
                    s.scrollTo(start, false, vertical);
                else if (start + length > position + viewport)
                    s.scrollTo(Math.min(start, start + length - viewport), false, vertical);
            }
        }
    }

    function _focus(item, forward) {
        item.forceActiveFocus(forward ? Qt.TabFocusReason : Qt.BacktabFocusReason);
        scrollIntoView(item);
    }

    /// Moves the focus on with Tab, or back with Shift+Tab. Returns whether it moved.
    function moveTab(forward) {
        const focused = window.activeFocusItem;
        // editable text takes Tab itself
        const edit = focused as TextEdit;
        if (edit && !edit.readOnly)
            return false;
        const from = !focused || home.activeFocus ? (startItem || home) : focused;

        // after pressing an item in a group, Tab focuses it
        const pressedGroup = from !== focused ? groupOf(from) : null;
        const pressed = pressedGroup ? pressedGroup.itemOf(from) : null;
        if (pressed) {
            _focus(pressed, forward);
            return true;
        }

        const fromGroup = groupOf(from);
        const fromItem = fromGroup ? fromGroup.itemOf(from) : null;
        let next = from;
        // the focus chain is a cycle through the window, or through a popup
        for (let i = 0; i < 10000; i++) {
            next = next.nextItemInFocusChain(forward);
            if (!next || next === from)
                return false;
            const group = groupOf(next);
            if (!group)
                break;
            if (group === fromGroup && fromItem) {
                // the current item's own controls, but none of the group's other items
                if (group.itemOf(next) === fromItem)
                    break;
                continue;
            }
            // entering the group
            next = group.entry() || next;
            break;
        }
        _focus(next, forward);
        return true;
    }

    /// The item of `items` nearest to `from` in a direction, `[dx, dy]`, by where they are on
    /// screen, or null if none is that way.
    function _nearest(from, items, direction) {
        const f = from.mapToItem(null, 0, 0, from.width, from.height);
        const vertical = direction[1] !== 0;
        const sign = vertical ? direction[1] : direction[0];
        let best = null;
        let bestScore = Infinity;
        for (const item of items) {
            if (item === from)
                continue;
            const r = item.mapToItem(null, 0, 0, item.width, item.height);
            // how far it is that way, from edge to edge, and how far off to the side
            const gap = vertical ? (sign > 0 ? r.y - (f.y + f.height) : f.y - (r.y + r.height)) : (sign > 0 ? r.x - (f.x + f.width) : f.x - (r.x + r.width));
            if (gap < -1)
                continue;
            const side = vertical ? Math.abs((r.x + r.width / 2) - (f.x + f.width / 2)) : Math.abs((r.y + r.height / 2) - (f.y + f.height / 2));
            const score = Math.max(gap, 0) + 2 * side;
            if (score < bestScore) {
                best = item;
                bestScore = score;
            }
        }
        return best;
    }

    /// Moves the focus between the items of a group with a key. Returns whether it moved.
    function moveInGroup(group, key) {
        const view = group.view;
        const layout = () => {
            if (view.forceLayout)
                view.forceLayout();
        };
        let from = group.itemOf(window.activeFocusItem);
        if (!from && home.activeFocus && startItem)
            from = group.itemOf(startItem);
        let target = null;

        if (key === Qt.Key_Home || key === Qt.Key_End) {
            // the first or last item may not exist yet in a view that creates items as they
            // scroll into it
            if (key === Qt.Key_Home && view.positionViewAtBeginning)
                view.positionViewAtBeginning();
            else if (key === Qt.Key_End && view.positionViewAtEnd)
                view.positionViewAtEnd();
            layout();
            const items = group.items();
            target = key === Qt.Key_Home ? items[0] : items[items.length - 1];
        } else if (!from) {
            target = group.entry();
        } else if (key === Qt.Key_PageUp || key === Qt.Key_PageDown) {
            // the item that scrolls into the place of this one
            const before = from.mapToItem(null, from.width / 2, from.height / 2);
            const s = scrollerOf(from);
            const sign = key === Qt.Key_PageDown ? 1 : -1;
            if (s && s.canScroll(sign, true)) {
                s.scrollBy(sign * s.pageStep(true), false, true);
                layout();
            }
            let bestDistance = Infinity;
            for (const item of group.items()) {
                const c = item.mapToItem(null, item.width / 2, item.height / 2);
                const distance = Math.abs(c.y - before.y) + 2 * Math.abs(c.x - before.x);
                if (distance < bestDistance && (sign > 0 ? c.y >= before.y - 1 : c.y <= before.y + 1) && item !== from) {
                    target = item;
                    bestDistance = distance;
                }
            }
            if (!target) {
                const items = group.items();
                target = sign > 0 ? items[items.length - 1] : items[0];
            }
        } else {
            const direction = key === Qt.Key_Up ? [0, -1] : key === Qt.Key_Down ? [0, 1] : key === Qt.Key_Left ? [-1, 0] : [1, 0];
            target = _nearest(from, group.items(), direction);
            if (!target) {
                // the next item may not exist until it scrolls into the view
                const vertical = direction[1] !== 0;
                const sign = vertical ? direction[1] : direction[0];
                const s = scrollerOf(from);
                if (s && Ui.isWithin(s.flickable, view) && s.canScroll(sign, vertical)) {
                    s.scrollBy(sign * (vertical ? from.height : from.width), false, vertical);
                    layout();
                    target = _nearest(from, group.items(), direction);
                }
            }
        }
        if (!target || target === from)
            return false;
        _focus(target, true);
        return true;
    }

    /// Moves the focus to the next or previous item in the tab order within `group`, wrapping
    /// around, or into the group if the focus is outside it. Returns whether the focus moved.
    function moveFocusWithin(group, forward) {
        const focused = window.activeFocusItem;
        const from = focused && Ui.isWithin(focused, group) ? focused : group;
        let next = from;
        // the focus chain is a cycle through the whole window, which re-enters the group
        for (let i = 0; i < 10000; i++) {
            next = next.nextItemInFocusChain(forward);
            if (!next || next === from)
                return false;
            if (Ui.isWithin(next, group)) {
                next.forceActiveFocus(forward ? Qt.TabFocusReason : Qt.BacktabFocusReason);
                return true;
            }
        }
        return false;
    }

    /// Handles a key that the focused item left unhandled. Returns whether it was handled.
    function handleKey(event) {
        event.accepted = false;
        if (event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))
            return false;
        const shift = (event.modifiers & Qt.ShiftModifier) !== 0;

        const movesInGroup = [Qt.Key_Up, Qt.Key_Down, Qt.Key_Left, Qt.Key_Right, Qt.Key_PageUp, Qt.Key_PageDown, Qt.Key_Home, Qt.Key_End].includes(event.key);
        const group = movesInGroup ? groupOf(home.activeFocus && startItem ? startItem : window.activeFocusItem) : null;
        if (group && moveInGroup(group, event.key)) {
            event.accepted = true;
            return true;
        }

        let delta = 0;
        let vertical = true;
        let to = undefined;
        switch (event.key) {
        case Qt.Key_Up:
            delta = -1;
            break;
        case Qt.Key_Down:
            delta = 1;
            break;
        case Qt.Key_Left:
            delta = -1;
            vertical = false;
            break;
        case Qt.Key_Right:
            delta = 1;
            vertical = false;
            break;
        case Qt.Key_PageUp:
            delta = -2;
            break;
        case Qt.Key_PageDown:
            delta = 2;
            break;
        case Qt.Key_Space:
            delta = shift ? -2 : 2;
            break;
        case Qt.Key_Home:
            delta = -1;
            to = "start";
            break;
        case Qt.Key_End:
            delta = 1;
            to = "end";
            break;
        default:
            return false;
        }

        let s = scroller && scroller.flickable && scroller.flickable.visible ? scroller : null;
        while (s && !s.canScroll(delta, vertical))
            s = _parentScroller(s);
        if (!s)
            return false;
        if (to !== undefined) {
            const [min, max] = s.bounds(vertical);
            s.scrollTo(to === "start" ? min : max, true, vertical);
        } else {
            const step = Math.abs(delta) === 2 ? s.pageStep(vertical) : s.lineStep;
            s.scrollBy(Math.sign(delta) * step, true, vertical);
        }
        event.accepted = true;
        return true;
    }

    readonly property Connections _input: Connections {
        target: Native

        function onPointerPressed(position) {
            Ui.focusVisible = false;
            const pressed = Native.itemAt(navigator.window, position.x, position.y);
            navigator.startItem = pressed;
            navigator.scroller = pressed ? navigator.scrollerOf(pressed) : null;
            const group = pressed ? navigator.groupOf(pressed) : null;
            const item = group ? group.itemOf(pressed) : null;
            if (item)
                group.current = item;

            // pressing elsewhere takes the focus away, which also commits text being edited
            const focused = navigator.window.activeFocusItem;
            if (focused && focused !== navigator.home && !(pressed && Ui.isWithin(pressed, focused))) {
                focused.focus = false;
                const now = navigator.window.activeFocusItem;
                if (!now || now === navigator.window.contentItem)
                    navigator.home.forceActiveFocus();
            }
        }

        function onKeyPressed(key) {
            if (key !== Qt.Key_Shift && key !== Qt.Key_Control && key !== Qt.Key_Alt && key !== Qt.Key_Meta)
                Ui.focusVisible = true;
        }
    }

    readonly property Connections _focusChanges: Connections {
        target: navigator.window

        function onActiveFocusItemChanged() {
            const item = navigator.window.activeFocusItem;
            if (!item || item === navigator.home || item === navigator.window.contentItem)
                return;
            navigator.startItem = item;
            const group = navigator.groupOf(item);
            const groupItem = group ? group.itemOf(item) : null;
            if (groupItem)
                group.current = groupItem;
            // an item outside any panel leaves the keyboard scrolling the last one, unless the focus
            // moved between the window's content and a popup
            const s = navigator.scrollerOf(item);
            if (s || !navigator.scroller || navigator._layerOf(navigator.scroller.flickable) !== navigator._layerOf(item))
                navigator.scroller = s;
            if (Ui.focusVisible)
                navigator.scrollIntoView(item);
        }
    }
}
