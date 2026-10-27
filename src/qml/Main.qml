pragma ComponentBehavior: Bound
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
    // macOS and Windows keep the rest of their native frames, whose title bars Native hides, as
    // frameless windows there can't be resized from outside, snapped, or animated. On macOS, the
    // app's title bar takes the place of the native one, under which the window's content
    // reaches, and the native window buttons are placed in it, like Tauri's overlay title bar
    // style. Elsewhere the app draws the whole frame.
    flags: Qt.platform.os === "osx" ? Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint : Qt.platform.os === "windows" ? Qt.Window : Qt.Window | Qt.FramelessWindowHint
    // The content would otherwise be padded to the window's safe area, which on macOS is below the
    // native title bar, leaving the app's title bar below it instead of in its place.
    topPadding: 0
    leftPadding: 0
    rightPadding: 0
    bottomPadding: 0
    /// Whether the window has a native frame without its title bar, which resizes the window.
    property bool _nativeFrame: false

    font.family: Theme.font
    font.pixelSize: Theme.rem

    // the window's pixel ratio, for effects that work in device pixels. Its screen's can differ:
    // with fractional scaling on Wayland, a screen's is the scale rounded up to a whole number.
    function _updateDpr() {
        Ui.dpr = Native.devicePixelRatio(appWindow);
    }
    Connections {
        target: Native
        function onDevicePixelRatioChanged() {
            appWindow._updateDpr();
        }
    }

    onWidthChanged: {
        Theme.windowWidth = width;
        _trackGeometry();
    }

    // The geometry to save, tracked while the window is neither maximized nor minimized.
    function _trackGeometry() {
        if (visibility !== Window.Windowed)
            return;
        const s = App.windowState;
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
        if (App.devOption("screenshot") !== "")
            return;
        const s = App.windowState;
        s.maximized = visibility === Window.Maximized;
        s.fullscreen = visibility === Window.FullScreen;
        s.save();
    }

    function _restoreState() {
        const s = App.windowState;
        if (!s.saved) {
            x = (Screen.width - width) / 2;
            y = (Screen.height - height) / 2;
            return;
        }
        width = s.width;
        height = s.height;
        // restore the position only if it is still on a screen
        const px = s.maximized ? s.prevX : s.x;
        const py = s.maximized ? s.prevY : s.y;
        const onScreen = Application.screens.some(screen => px < screen.virtualX + screen.width && px + s.width > screen.virtualX && py < screen.virtualY + screen.height && py + s.height > screen.virtualY);
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
    /// What kept the app from starting: its settings or profiles couldn't be read.
    readonly property var startupError: {
        if (Settings.errorMessages.length > 0)
            return Async.nativeError(Settings.errorMessages, Settings.backtrace);
        if (Profiles.errorMessages.length > 0)
            return Async.nativeError(Profiles.errorMessages, Profiles.backtrace);
        return null;
    }

    // The singletons that the backend sends events to are created before it can send them any,
    // rather than as they're first used, which would drop those sent before then.
    readonly property list<QtObject> _eventSinks: [Console, Tasks]

    function reportError(e) {
        if (e && e.isAborted)
            return;
        console.error(e, e && e.stack);
        globalError.err = e;
        globalError.open();
    }

    Component.onCompleted: {
        Ui.window = appWindow;
        _updateDpr();
        Ui.appContent = content;
        if (Qt.platform.os === "windows" || Qt.platform.os === "osx")
            _nativeFrame = Native.hideTitleBar(appWindow);
        if (Qt.platform.os === "windows" && !_nativeFrame)
            flags = Qt.Window | Qt.FramelessWindowHint;
        Theme.windowWidth = width;
        _restoreState();
        if (!visible)
            show();
        // once the window exists on screen
        Qt.callLater(() => {
            Native.setBlurBehind(appWindow, appWindow._blurBehind);
            Native.setDarkFrame(appWindow, Theme.dark);
        });

        App.benchExitSplash();
        I18n.detectLocale(App.preferredLocales());

        const size = App.devOption("size");
        if (size !== "") {
            const [w, h] = size.split("x").map(Number);
            showNormal();
            width = w;
            height = h;
        }
        const route = App.devOption("route");
        if (route !== "")
            Router.navigate(route, { replace: true });
        const screenshot = App.devOption("screenshot");
        if (screenshot !== "")
            screenshotTimer.start();
        if (App.devOption("script") !== "")
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
        interval: Number(App.devOption("script_delay") || 1500)
        onTriggered: {
            try {
                // qmllint disable literal-constructor
                const run = new Function("find", "App", "Async", "Console", "Games", "I18n", "ModIndex", "Native", "Profiles", "Router", "Settings", "Strings", "Tasks", "Theme", "Ui", App.devOption("script"));
                // qmllint enable literal-constructor
                run(appWindow.find, App, Async, Console, Games, I18n, ModIndex, Native, Profiles, Router, Settings, Strings, Tasks, Theme, Ui);
            } catch (e) {
                console.error("Dev script failed:", e, e.stack);
            }
        }
    }

    // Development hook: MANDERROW_DEV_SCREENSHOT=<path> saves a screenshot of the window after
    // MANDERROW_DEV_SCREENSHOT_DELAY milliseconds (3000 by default), then quits.
    Timer {
        id: screenshotTimer
        interval: Number(App.devOption("screenshot_delay") || 3000)
        onTriggered: {
            Native.grabWindow(appWindow, App.devOption("screenshot"));
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
    // through it.
    Binding {
        target: Ui
        property: "surfaceOpacity"
        value: Settings.windowTranslucent ? Math.max(0, Math.min(100, Settings.windowOpacity)) / 100 : 1
    }
    readonly property bool _blurBehind: Ui.translucent && Settings.windowBlur
    on_BlurBehindChanged: Native.setBlurBehind(appWindow, _blurBehind)
    onVisibleChanged: Native.setBlurBehind(appWindow, _blurBehind)
    // the native frame's colors, on Windows and macOS
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

    // snaps boxes onto device pixels before each frame is drawn, after animations and layouts have
    // moved them
    onAfterAnimating: {
        for (const snap of Ui.pixelSnaps)
            snap.update();
    }

    // browser-like focus, Tab, and keyboard scrolling
    FocusNavigator {
        id: navigator
        window: appWindow
        home: root
        Component.onCompleted: Ui.focusNavigator = navigator
    }

    Connections {
        target: Profiles
        function onLoadedChanged() {
            if (Profiles.loaded)
                App.benchExitInteractive();
        }
    }

    Connections {
        target: App
        function onActivationRequested() {
            if (appWindow.visibility === Window.Minimized)
                appWindow.showNormal();
            appWindow.raise();
            appWindow.requestActivate();
        }
    }

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
            readonly property bool _holdSplash: App.devOption("splash") !== ""

            Loader {
                anchors.fill: parent
                active: !Profiles.loaded || content._holdSplash
                sourceComponent: Splashscreen {}
            }

            Loader {
                anchors.fill: parent
                active: Profiles.loaded && !content._holdSplash
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
                ProfilePage {}
            }
            Component {
                id: settingsView
                SettingsPage {}
            }
            Component {
                id: notFoundView
                NotFound {}
            }

            // middle-click autoscrolling, over the pages
            MiddleClickScroll {}
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
        active: appWindow.startupError !== null
        sourceComponent: ErrorDialog {
            visible: true
            err: appWindow.startupError
            reset: () => {
                const error = App.relaunch();
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
