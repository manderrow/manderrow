import QtQuick
import Manderrow

// Lets the user resize the frameless window from its edges and corners.
Item {
    id: handles

    required property var window
    property real thickness: 5

    anchors.fill: parent
    visible: window.visibility === Window.Windowed

    component Handle: MouseArea {
        property int edges

        acceptedButtons: Qt.LeftButton
        onPressed: handles.window.startSystemResize(edges)
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
