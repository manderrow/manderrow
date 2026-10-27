pragma ComponentBehavior: Bound
import QtQuick

// Lets the user resize the frameless window from its edges and corners.
Item {
    id: handles

    required property var window
    property real thickness: 6
    /// The outermost pixels of the edges, which resize the window even where an item claims the
    /// edge, as the padding of a scroll bar there does, so that every edge can be grabbed. The
    /// corners always resize it.
    property real reserved: 3

    anchors.fill: parent
    visible: window.visibility === Window.Windowed

    /// Whether an item under a point of the window, other than these handles, takes input at the
    /// window's edges itself, by having `claimsWindowEdge` set, as scroll bars there do.
    function claimedAt(item, x, y) {
        if (item === handles || !item.visible)
            return false;
        const local = item.mapFromItem(null, x, y);
        if (item.clip && !item.contains(local))
            return false;
        if (item.claimsWindowEdge === true && item.contains(local))
            return true;
        for (let i = 0; i < item.children.length; i++) {
            if (claimedAt(item.children[i], x, y))
                return true;
        }
        return false;
    }

    component Handle: MouseArea {
        id: handle

        property int edges
        readonly property bool corner: (edges & (Qt.LeftEdge | Qt.RightEdge)) !== 0 && (edges & (Qt.TopEdge | Qt.BottomEdge)) !== 0

        acceptedButtons: Qt.LeftButton
        onPressed: handles.window.startSystemResize(edges)

        /// How far a point of the window is from the edge that the handle resizes.
        function distanceToEdge(x, y) {
            if (edges & Qt.LeftEdge)
                return x;
            if (edges & Qt.RightEdge)
                return handles.width - x;
            if (edges & Qt.TopEdge)
                return y;
            return handles.height - y;
        }

        // Where an item claims the edge, the handle isn't there at all, so that item gets the
        // pointer's presses, hovering, and cursor.
        containmentMask: QtObject {
            function contains(point: point): bool {
                if (point.x < 0 || point.y < 0 || point.x >= handle.width || point.y >= handle.height)
                    return false;
                if (handle.corner)
                    return true;
                const local = handle.mapToItem(handles, point.x, point.y);
                if (handle.distanceToEdge(local.x, local.y) < handles.reserved)
                    return true;
                const scene = handle.mapToItem(null, point.x, point.y);
                return !handles.claimedAt(handles.window.contentItem, scene.x, scene.y);
            }
        }
    }

    Handle {
        edges: Qt.LeftEdge
        cursorShape: Qt.SizeHorCursor
        width: handles.thickness
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
    }
    Handle {
        edges: Qt.RightEdge
        cursorShape: Qt.SizeHorCursor
        width: handles.thickness
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.right: parent.right
    }
    Handle {
        edges: Qt.TopEdge
        cursorShape: Qt.SizeVerCursor
        height: handles.thickness
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
    }
    Handle {
        edges: Qt.BottomEdge
        cursorShape: Qt.SizeVerCursor
        height: handles.thickness
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
    }
    Handle {
        edges: Qt.TopEdge | Qt.LeftEdge
        cursorShape: Qt.SizeFDiagCursor
        width: 2 * handles.thickness
        height: width
    }
    Handle {
        edges: Qt.TopEdge | Qt.RightEdge
        cursorShape: Qt.SizeBDiagCursor
        width: 2 * handles.thickness
        height: width
        anchors.right: parent.right
    }
    Handle {
        edges: Qt.BottomEdge | Qt.LeftEdge
        cursorShape: Qt.SizeBDiagCursor
        width: 2 * handles.thickness
        height: width
        anchors.bottom: parent.bottom
    }
    Handle {
        edges: Qt.BottomEdge | Qt.RightEdge
        cursorShape: Qt.SizeFDiagCursor
        width: 2 * handles.thickness
        height: width
        anchors.right: parent.right
        anchors.bottom: parent.bottom
    }
}
