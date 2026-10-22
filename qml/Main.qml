import QtQuick
import QtQuick.Controls.Basic
import Manderrow

ApplicationWindow {
    id: appWindow

    width: 1024
    height: 576
    minimumWidth: 500
    minimumHeight: 400
    // shown once its frame is set up and its geometry restored
    visible: false
    title: "Manderrow"
    // translucent, the window's regions each draw their own background
    color: Ui.translucent ? "transparent" : Theme.neutral85
    // macOS keeps its native window controls over the app's title bar, like Tauri's overlay
    // title bar style. Windows keeps the rest of its native frame, whose title bar Native hides,
    // as frameless windows there can't be resized from outside, snapped, or animated. Elsewhere
    // the app draws the whole frame.
    flags: Qt.platform.os === "osx" ? Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint : Qt.platform.os === "windows" ? Qt.Window : Qt.Window | Qt.FramelessWindowHint
    /// Whether the window has a native frame without its title bar, which resizes the window.
    property bool _nativeFrame: false

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
        if (Qt.platform.os === "windows") {
            _nativeFrame = Native.hideTitleBar(appWindow);
            if (!_nativeFrame)
                flags = Qt.Window | Qt.FramelessWindowHint;
        }
        Theme.windowWidth = width;
        _restoreState();
        if (!visible)
            show();
        // once the window exists on screen
        Qt.callLater(() => {
            Native.setBlurBehind(appWindow, appWindow._blurBehind);
            Native.setDarkFrame(appWindow, Theme.dark);
        });

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

    // The appearance settings' glass: the window's background shows the desktop, blurred,
    // through it. Until the settings load, their defaults apply.
    function _setting(key, fallback) {
        const setting = Globals.settings ? Globals.settings[key] : undefined;
        return setting !== undefined ? setting.value : fallback;
    }
    Binding {
        target: Ui
        property: "surfaceOpacity"
        value: appWindow._setting("windowTranslucent", true) ? Math.max(0, Math.min(100, appWindow._setting("windowOpacity", 70))) / 100 : 1
    }
    readonly property bool _blurBehind: Ui.translucent && _setting("windowBlur", true)
    on_BlurBehindChanged: Native.setBlurBehind(appWindow, _blurBehind)
    onVisibleChanged: Native.setBlurBehind(appWindow, _blurBehind)
    // the native frame's colors, on Windows
    Connections {
        target: Theme
        function onDarkChanged() {
            Native.setDarkFrame(appWindow, Theme.dark);
        }
    }

    // The title bar is outside the overlay of popups, which is above it: pressing it neither
    // closes popups nor is blocked by a modal one, so that the window can still be moved and
    // controlled while a dialog is open. Dialogs and their backdrops are placed below it.
    Binding {
        target: appWindow.Overlay.overlay
        property: "containmentMask"
        value: Ui.belowTitleBar
    }

    // browser-like focus, Tab, and keyboard scrolling
    FocusNavigator {
        id: navigator
        window: appWindow
        home: root
        Component.onCompleted: Ui.focusNavigator = navigator
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
        // holds the focus when nothing else does, so that the keyboard can still scroll and move
        // the focus on from where the pointer was pressed
        focus: true
        Keys.onPressed: event => navigator.handleKey(event)

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
            visible: Qt.platform.os !== "osx" && !appWindow._nativeFrame && appWindow.visibility === Window.Windowed
        }
    }

    Loader {
        active: Globals.error !== null
        sourceComponent: ErrorDialog {
            visible: true
            err: Globals.error
            reset: () => {
                const error = Backend.relaunch();
                if (error !== "")
                    throw new Error(error);
                Qt.quit();
            }
        }
    }

    PopupLoader {
        id: globalError

        property var err: null

        sourceComponent: ErrorDialog {
            err: globalError.err
            reset: () => {}
        }
    }
}
