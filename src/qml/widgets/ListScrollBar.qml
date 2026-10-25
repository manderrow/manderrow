import QtQuick
import Manderrow

// A vertical scroll bar for a ListView whose delegates differ widely in height, which ListView
// scrolls well but measures poorly: it estimates the height of the delegates it hasn't laid out
// from the average of those it has, and moves its origin as it lays out more of them, which a
// scroll bar attached to it follows. Next to a delegate far taller than the rest, the bar stops
// moving while the view scrolls, and jumps back and forth under the pointer while dragged, as
// what it scrolls to changes how far that is.
//
// This one measures the content itself: the delegates that have been laid out by their height,
// and the others by `estimate`. It places the view by row, at a fraction of the row's height, so
// that a new measurement only moves the bar a little, and never the view.
//
// Give it the view as its parent, rather than attaching it. The view's delegates must have an
// `index` property, as they do when they declare it as required.
AppScrollBar {
    id: bar

    required property ListView view
    /// A function that estimates the height of the delegate for a row.
    property var estimate: row => 0

    anchors.top: view.top
    anchors.right: view.right
    anchors.bottom: view.bottom
    orientation: Qt.Vertical
    size: Math.min(1, view.height / Math.max(_total, 1))

    /// The rows' heights, as measured, or estimated.
    property var _heights: []
    /// A Fenwick tree of `_heights`, for summing those before a row.
    property var _tree: [0]
    /// The height of the content, with the view's margins.
    property real _total: 0
    property bool _syncing: false

    /// Estimates the heights of all the rows again, as when the width of the view changes them.
    function reestimate() {
        const count = view.count;
        const heights = new Array(count);
        for (let i = 0; i < count; i++)
            heights[i] = estimate(i);
        _heights = heights;
        const tree = new Array(count + 1).fill(0);
        for (let i = 1; i <= count; i++) {
            tree[i] += heights[i - 1];
            const parent = i + (i & -i);
            if (parent <= count)
                tree[parent] += tree[i];
        }
        _tree = tree;
        _measure();
        _sync();
    }

    function _append(first, last) {
        for (let row = first; row <= last; row++) {
            const height = estimate(row);
            // the sum of the rows that its node covers, with it
            const i = row + 1;
            _tree.push(height + _sum(row) - _sum(i - (i & -i)));
            _heights.push(height);
        }
        _measure();
        _sync();
    }

    /// The sum of the heights of the rows before `row`.
    function _sum(row) {
        let sum = 0;
        for (let i = row; i > 0; i -= i & -i)
            sum += _tree[i];
        return sum;
    }

    function _setHeight(row, height) {
        const delta = height - _heights[row];
        if (delta === 0)
            return;
        _heights[row] = height;
        for (let i = row + 1; i < _tree.length; i += i & -i)
            _tree[i] += delta;
    }

    /// The row that a distance down the rows is in, the last if it is past them.
    function _rowAt(distance) {
        let row = 0;
        let step = 1;
        while (step * 2 < _tree.length)
            step *= 2;
        for (; step > 0; step >>= 1) {
            if (row + step < _tree.length && _tree[row + step] <= distance) {
                row += step;
                distance -= _tree[row];
            }
        }
        return Math.min(row, _heights.length - 1);
    }

    /// Measures the delegates that the view has laid out.
    function _measure() {
        for (const item of view.contentItem.children) {
            const row = item.index;
            if (typeof row === "number" && row >= 0 && row < _heights.length && item.height > 0)
                _setHeight(row, item.height);
        }
        _total = view.topMargin + _sum(_heights.length) + view.bottomMargin;
    }

    /// Where the view is scrolled to, as the height of the content above the top of the view, or
    /// null if that isn't known.
    function _offset() {
        const count = _heights.length;
        if (count === 0)
            return 0;
        const y = view.contentY;
        const row = view.indexAt(0, y);
        if (row >= 0 && row < count) {
            const item = view.itemAtIndex(row);
            return view.topMargin + _sum(row) + (y - item.y) / item.height * _heights[row];
        }
        const first = view.itemAtIndex(0);
        if (first && y < first.y)
            return Math.max(0, view.topMargin - (first.y - y));
        const last = view.itemAtIndex(count - 1);
        if (last && y >= last.y + last.height)
            return view.topMargin + _sum(count) + (y - last.y - last.height);
        return null;
    }

    function _sync() {
        if (pressed || _syncing)
            return;
        const offset = _offset();
        if (offset === null)
            return;
        const scrollable = _total - view.height;
        _syncing = true;
        position = scrollable > 0 ? Math.max(0, Math.min(1, offset / scrollable)) * (1 - size) : 0;
        _syncing = false;
    }

    /// Scrolls the view to the bar's position.
    function _scroll() {
        const scrollable = _total - view.height;
        const fraction = size < 1 ? position / (1 - size) : 0;
        const offset = fraction * scrollable;
        const count = _heights.length;
        if (count === 0 || scrollable <= 0)
            return;
        _syncing = true;
        if (offset >= scrollable - 0.5) {
            // as ListView's own positioning leaves out the bottom margin
            view.positionViewAtEnd();
            view.contentY = Math.max(view.originY - view.topMargin, view.originY + view.contentHeight + view.bottomMargin - view.height);
        } else if (offset <= view.topMargin) {
            view.positionViewAtBeginning();
            const first = view.itemAtIndex(0);
            if (first)
                view.contentY = first.y - view.topMargin + offset;
        } else {
            const distance = offset - view.topMargin;
            const row = _rowAt(distance);
            view.positionViewAtIndex(row, ListView.Beginning);
            const item = view.itemAtIndex(row);
            if (item) {
                const within = Math.max(0, Math.min(1, (distance - _sum(row)) / _heights[row]));
                view.contentY = item.y + within * item.height;
            }
        }
        _syncing = false;
        _measure();
    }

    onPositionChanged: if (!_syncing)
        _scroll()
    onPressedChanged: if (!pressed)
        _sync()

    Connections {
        target: bar.view
        function onContentYChanged() {
            bar._measure();
            bar._sync();
        }
        function onHeightChanged() {
            bar._sync();
        }
        // as the view lays out more of its delegates
        function onContentHeightChanged() {
            Qt.callLater(bar._remeasure);
        }
    }

    function _remeasure() {
        _measure();
        _sync();
    }

    Connections {
        target: bar.view.model
        ignoreUnknownSignals: true
        function onRowsInserted(parent, first, last) {
            if (first === bar._heights.length)
                bar._append(first, last);
            else
                Qt.callLater(bar.reestimate);
        }
        function onRowsRemoved() {
            Qt.callLater(bar.reestimate);
        }
        function onRowsMoved() {
            Qt.callLater(bar.reestimate);
        }
        function onModelReset() {
            Qt.callLater(bar.reestimate);
        }
        function onLayoutChanged() {
            Qt.callLater(bar.reestimate);
        }
    }

    Component.onCompleted: reestimate()
}
