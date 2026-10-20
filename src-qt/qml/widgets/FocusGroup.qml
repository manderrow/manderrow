import QtQuick
import Manderrow

// Makes the items of a view one stop in the Tab order, as Discord's lists are: Tab enters the
// group at its current item, then moves through that item's own controls and on out of the
// group, and the arrow keys move between the items. This keeps long and endless lists from
// taking a Tab per item. FocusNavigator does the moving.
//
// The items are the outermost items in the view's content that take the focus from Tab, such as
// its delegates, or the cards in each row of a grid. A ListView's header and footer aren't part
// of the group: their controls are ordinary stops.
QtObject {
    id: group

    /// The view, usually a ListView, whose content holds the items.
    required property Item view
    /// The item that the group was last on, where Tab enters it.
    property Item current: null

    /// The group's items that exist, in their order in the content.
    function items() {
        const found = [];
        const collect = item => {
            if (!item.visible || !item.enabled)
                return;
            if (item.activeFocusOnTab) {
                found.push(item);
                return;
            }
            for (const child of item.children)
                collect(child);
        };
        const content = view.contentItem || view;
        for (const child of content.children) {
            if (!_isHeaderOrFooter(child))
                collect(child);
        }
        return found;
    }

    /// The group's item that holds `item`, or null if it isn't in one.
    function itemOf(item) {
        const content = view.contentItem || view;
        let outermost = null;
        for (let i = item; i && i !== content; i = i.parent) {
            if (i === view)
                return null;
            if (i.activeFocusOnTab && i.visible)
                outermost = i;
        }
        return Ui.isWithin(item, content) && !_isHeaderOrFooter(item) ? outermost : null;
    }

    function _isHeaderOrFooter(item) {
        return (!!view.headerItem && Ui.isWithin(item, view.headerItem)) || (!!view.footerItem && Ui.isWithin(item, view.footerItem));
    }

    /// Whether `item` is the group's view or inside it, outside its header and footer.
    function holds(item) {
        return Ui.isWithin(item, view) && !_isHeaderOrFooter(item);
    }

    /// Where Tab enters the group: its current item if it still shows, or else the first item
    /// in view.
    function entry() {
        if (current && current.visible && holds(current))
            return current;
        const all = items();
        const top = view.mapToItem(null, 0, 0).y;
        return all.find(item => item.mapToItem(null, 0, item.height).y > top) || all[0] || null;
    }

    Component.onCompleted: Ui.focusGroups.push(group)
    Component.onDestruction: {
        const i = Ui.focusGroups.indexOf(group);
        if (i >= 0)
            Ui.focusGroups.splice(i, 1);
    }
}
