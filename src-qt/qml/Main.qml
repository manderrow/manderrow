import QtQuick
import QtQuick.Controls.Basic
import Manderrow

ApplicationWindow {
    id: appWindow

    width: 1024
    height: 576
    minimumWidth: 500
    minimumHeight: 400
    visible: true
    title: "Manderrow"
    color: Theme.neutral85
    // macOS keeps its native window controls over the app's title bar, like Tauri's overlay
    // title bar style. Elsewhere the app draws the whole frame.
    flags: Qt.platform.os === "osx" ? Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint : Qt.Window | Qt.FramelessWindowHint

    font.family: Theme.font
    font.pixelSize: Theme.rem

    // the screen's pixel ratio, for effects that work in device pixels
    Binding {
        target: Ui
        property: "dpr"
        value: appWindow.screen ? appWindow.screen.devicePixelRatio : 1
    }

    onWidthChanged: {
        Theme.windowWidth = width;
        _trackGeometry();
    }

    // The geometry to save, tracked while the window is neither maximized nor minimized.
    property var _state: ({ width: 1024, height: 576, x: 0, y: 0, prevX: 0, prevY: 0, maximized: false, fullscreen: false })

    function _trackGeometry() {
        if (visibility !== Window.Windowed)
            return;
        const s = _state;
        if (s.x !== x || s.y !== y) {
            s.prevX = s.x;
            s.prevY = s.y;
            s.x = x;
            s.y = y;
        }
        if (width > 0 && height > 0) {
            s.width = width;
            s.height = height;
        }
    }

    onXChanged: _trackGeometry()
    onYChanged: _trackGeometry()
    onHeightChanged: _trackGeometry()

    onClosing: {
        // automated screenshot runs shouldn't overwrite the user's window state
        if (Backend.devOption("screenshot") !== "")
            return;
        _state.maximized = visibility === Window.Maximized;
        _state.fullscreen = visibility === Window.FullScreen;
        Backend.saveWindowState(JSON.stringify(_state));
    }

    function _restoreState() {
        const saved = Backend.windowState();
        if (saved === "") {
            x = (Screen.width - width) / 2;
            y = (Screen.height - height) / 2;
            return;
        }
        const s = JSON.parse(saved);
        _state = s;
        width = s.width;
        height = s.height;
        // restore the position only if it is still on a screen
        const px = s.maximized ? s.prevX : s.x;
        const py = s.maximized ? s.prevY : s.y;
        const onScreen = Qt.application.screens.some(screen => px < screen.virtualX + screen.width && px + s.width > screen.virtualX && py < screen.virtualY + screen.height && py + s.height > screen.virtualY);
        if (onScreen) {
            x = px;
            y = py;
        } else {
            x = (Screen.width - width) / 2;
            y = (Screen.height - height) / 2;
        }
        if (s.fullscreen)
            showFullScreen();
        else if (s.maximized)
            showMaximized();
    }

    /// Shows an error that a view couldn't handle itself, like the web UI's ErrorBoundary.
    function reportError(e) {
        if (e && e.isAborted)
            return;
        console.error(e, e && e.stack);
        globalError.err = e;
        globalError.open();
    }

    Component.onCompleted: {
        Ui.window = appWindow;
        Ui.appContent = content;
        Theme.windowWidth = width;
        _restoreState();

        Api.benchExitSplash();
        Api.getPreferredLocales().then(locales => I18n.detectLocale(locales));
        ConsoleStore.init();
        Globals.load();

        const size = Backend.devOption("size");
        if (size !== "") {
            const [w, h] = size.split("x").map(Number);
            showNormal();
            width = w;
            height = h;
        }
        const route = Backend.devOption("route");
        if (route !== "")
            Router.navigate(route, { replace: true });
        const screenshot = Backend.devOption("screenshot");
        if (screenshot !== "")
            screenshotTimer.start();
        if (Backend.devOption("script") !== "")
            scriptTimer.start();
    }

    /// Finds an item or popup by its objectName, for development scripts.
    function find(objectName, object) {
        object = object || appWindow.contentItem;
        if (object.objectName === objectName)
            return object;
        // an item's data includes popups declared in it, which aren't among its children
        const nested = object.data !== undefined ? object.data : object.contentItem ? [object.contentItem] : [];
        for (let i = 0; i < nested.length; i++) {
            const found = nested[i] ? find(objectName, nested[i]) : null;
            if (found)
                return found;
        }
        return null;
    }

    // Development hook: MANDERROW_DEV_SCRIPT=<js> runs after MANDERROW_DEV_SCRIPT_DELAY
    // milliseconds (1500 by default), with `find(objectName)` and the core singletons available.
    Timer {
        id: scriptTimer
        interval: Number(Backend.devOption("script_delay") || 1500)
        onTriggered: {
            try {
                const run = new Function("find", "Api", "ConsoleStore", "Globals", "Native", "Router", "Tasks", "Ui", Backend.devOption("script"));
                run(appWindow.find, Api, ConsoleStore, Globals, Native, Router, Tasks, Ui);
            } catch (e) {
                console.error("Dev script failed:", e, e.stack);
            }
        }
    }

    // Development hook: MANDERROW_DEV_SCREENSHOT=<path> saves a screenshot of the window after
    // MANDERROW_DEV_SCREENSHOT_DELAY milliseconds (3000 by default), then quits.
    Timer {
        id: screenshotTimer
        interval: Number(Backend.devOption("screenshot_delay") || 3000)
        onTriggered: {
            Native.grabWindow(appWindow, Backend.devOption("screenshot"));
            Qt.quit();
        }
    }

    FontMetrics {
        id: chMetrics
        font.family: Theme.font
        font.pixelSize: Theme.rem
        Component.onCompleted: Ui.ch = advanceWidth("0")
        onFontChanged: Ui.ch = advanceWidth("0")
    }

    // As on the web, pressing anywhere outside the focused text input or text blurs it, which
    // commits editors and clears selections. Qt only moves focus to items that take it.
    Connections {
        target: Native
        function onPointerPressed(position) {
            const item = appWindow.activeFocusItem;
            if (!item || item.cursorPosition === undefined)
                return;
            const local = item.mapFromItem(null, position.x, position.y);
            if (!item.contains(local))
                item.focus = false;
        }
    }

    Connections {
        target: Globals
        function onLoadedChanged() {
            if (Globals.loaded)
                Api.benchExitInteractive();
        }
    }

    readonly property var _activateSubscription: Api.listen("single_instance", () => {
        if (appWindow.visibility === Window.Minimized)
            appWindow.showNormal();
        appWindow.raise();
        appWindow.requestActivate();
    })

    Rectangle {
        id: root
        anchors.fill: parent
        color: appWindow.color

        Item {
            id: content
            anchors.fill: parent
            anchors.topMargin: Theme.titlebarHeight

            // the `splash` dev option keeps the splashscreen up, for testing
            readonly property bool _holdSplash: Backend.devOption("splash") !== ""

            Loader {
                anchors.fill: parent
                active: !Globals.loaded || content._holdSplash
                sourceComponent: Splashscreen {}
            }

            Loader {
                anchors.fill: parent
                active: Globals.loaded && !content._holdSplash
                sourceComponent: {
                    switch (Router.route) {
                    case "profile":
                        return profileView;
                    case "settings":
                        return settingsView;
                    default:
                        return notFoundView;
                    }
                }
            }

            Component {
                id: profileView
                Profile {}
            }
            Component {
                id: settingsView
                SettingsPage {}
            }
            Component {
                id: notFoundView
                NotFound {}
            }
        }

        TitleBar {
            id: titleBar
            window: appWindow
            width: parent.width
            profileName: Ui.currentProfileName
        }

        ResizeHandles {
            window: appWindow
            visible: Qt.platform.os !== "osx" && appWindow.visibility === Window.Windowed
        }
    }

    ErrorDialog {
        id: startupError
        visible: Globals.error !== null
        err: Globals.error
        reset: () => {
            const error = Backend.relaunch();
            if (error !== "")
                throw new Error(error);
            Qt.quit();
        }
    }

    ErrorDialog {
        id: globalError
        reset: () => {}
    }
}
