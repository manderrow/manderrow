# Manderrow

[![build](https://github.com/Jack-the-Pro101/manderrow/actions/workflows/build.yml/badge.svg)](https://github.com/Jack-the-Pro101/manderrow/actions/workflows/build.yml)

- [About](#about)
- [Features and Improvements](#features-and-improvements)
- [Building](#building)
- [Developing](#developing)
- [Contributing](#contributing)

## About

Manderrow is a mod manager with the goal of becoming the successor to [r2modman](https://github.com/ebkr/r2modmanPlus). Because of this, it will support all of the existing features of r2modman, in addition to more. Also, Manderrow aims to address the major issues of r2modman, outlined in the next section.

## Features and Improvements

### Improvements:

| r2modman                                                                                                                                                      | Manderrow                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Performance of loading mod list is extremely slow (20-30s)                                                                                                    | Optimized JSON parsing results in 3-8x faster loading speeds                                   |
| Memory usage while loading mod list is extremely high (peaks around 1GB)                                                                                      | Optimized JSON parsing results in around half the memory usage                                 |
| No multi-language support after 5 years, despite having an i18n library installed                                                                             | Multiple languages (English, Spanish, French, Mandarin) will be supported on the first release |
| Unintuitive UI choices (e.g. profile switching and sharing is in settings menu)                                                                               | Intuitive UI                                                                                   |
| Auto-updater not functional (this issue is acknowledged in the [r2modman docs](https://r2modman.net/how-to-update-r2modman#h.3znysh7) yet has not been fixed) | Functional auto-updater                                                                        |
| Uses deprecated and old technologies: Vue 2, moment JS, Electron v24                                                                                          | Uses modern and up-to-date technologies: Rust and a native Qt 6 Quick UI                       |

### New features:

- Customizable themes
- Better UX: more cancel control, background-processed UI instead of fully blocking UI, and modernized interface.
- (might add eventually) Ability to support new games without having to update the app if the game uses existing launchers.

## Building

Manderrow is a Rust application with a [Qt Quick](https://doc.qt.io/qt-6/qtquick-index.html) user interface, bridged with [CXX-Qt](https://github.com/KDAB/cxx-qt). You'll need:

- The nightly Rust toolchain pinned in `rust-toolchain.toml`
- [Zig](https://ziglang.org/) 0.17, which builds the injection agent
- Qt 6.10 or newer, with the Qt Quick, Qt Quick Controls, and Qt SVG development packages. Set `QMAKE` to the path of `qmake` if it isn't on your `PATH`.
  - Fedora: `sudo dnf install qt6-qtbase-devel qt6-qtbase-private-devel qt6-qtdeclarative-devel qt6-qtsvg-devel`
  - Debian/Ubuntu: `sudo apt install qt6-base-dev qt6-base-private-dev qt6-declarative-dev qt6-svg-dev`

[mise](https://mise.jdx.dev/) installs the Rust and Zig versions with `mise install`. Then:

```sh
just run    # build and run a debug build
just build  # build a release build
just test   # run the tests
```

The backend is in `src`, the bridge to the UI in `src/qt`, and the UI in `src/qml`.

The UI's icons are [Lucide](https://lucide.dev/) icons, generated into a font, `assets/fonts/ManderrowIcons.ttf`, and its index, `src/qml/core/Icons.qml`, by `just icons` with [Deno](https://deno.com/). To add one, add its name to `scripts/icons/main.ts` and run it again.

Debug builds accept some options for developing and testing the UI, as `MANDERROW_DEV_<NAME>` environment variables:

| Option       | Effect                                                                                        |
| ------------ | --------------------------------------------------------------------------------------------- |
| `ROUTE`      | Opens a route at startup, like `/profile/lethal-company` or `/settings`                       |
| `THEME`      | Forces the `dark` or `light` theme                                                            |
| `SIZE`       | Sets the window size, like `1280x720`                                                         |
| `SPLASH`     | Keeps the splashscreen up                                                                     |
| `SCREENSHOT` | Saves a screenshot to this path after `SCREENSHOT_DELAY` milliseconds, then quits             |
| `SCRIPT`     | Runs JavaScript after `SCRIPT_DELAY` milliseconds, with `find(objectName)` and the singletons |
| `QML_DEBUG`  | Lets QML tools attach, like `qmlprofiler -o trace.qtd target/debug/manderrow`                 |

For example, to screenshot the settings page without a display:

```sh
MANDERROW_DEV_ROUTE=/settings MANDERROW_DEV_SCREENSHOT=settings.png \
  QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl \
  cargo run
```

QML's `console.log` is a debug message, which Qt as some distributions package it, such as Fedora's, hides: `QT_LOGGING_RULES=qml.debug=true` shows it. Qt also logs to the system journal rather than to the terminal when its output is redirected, unless `QT_FORCE_STDERR_LOGGING=1` is set.

## Developing

### Editor

[VS Code](https://code.visualstudio.com/) with the recommended extensions works once `just gen` has run: the workspace settings point the QML language server, qmlls, at the QML module in `target/cxxqt/qml_modules`, which `just gen` generates, as the build does, but in a second rather than the minute that the build spends compiling C++. The QML then has completion, warnings as you type, and go to definition, including for the types defined in Rust, whose go to definition opens their declarations in `src/qt`. qmlls reads what's generated only as it starts, so after changing the types defined in Rust or the translations, run `just gen`, then **Qt: Restart QML language server**. Use the qmlls of the Qt you build with, rather than the standalone one the Qt extension offers to download, which can't find Qt's own modules: unless your Qt came from Qt's installer, which the extension finds, set `qt-qml.qmlls.customExePath` in your user settings to the `qmlls` in the directory that `qmake -query QT_INSTALL_BINS` prints. In another editor, run qmlls with `-I target/cxxqt/qml_modules --no-cmake-calls`. qmlls only reads that module's mapping of the build's copies of the QML files back to `src/qml` from a directory given with `-I`, so without it, go to definition opens the copies.

If your shell runs `mise activate`, VS Code started from your desktop has your global tools rather than this project's, as it reads its environment from your shell in your home directory, so rust-analyzer's build of the agent fails with another Zig. Give it mise's shims instead, which run the versions of the project they run in. In fish:

```fish
if set -q VSCODE_RESOLVING_ENVIRONMENT
    mise activate fish --shims | source
else
    mise activate fish | source
end
```

### Linting

```sh
just lint             # lint all of the QML, failing on any warning
just lint src/qml/views/gameselect/GameSelect.qml
```

`just lint` runs `just gen` first, and qmllint reads the QML from `src/qml`, so it never needs a build. The QML is kept free of warnings, which keeps it checkable:

- Properties and function parameters are typed, like `property Game game` and `function select(game: Game)`, wherever QML has a type for them, rather than `var`. A misspelled member is then a warning, and the editor can complete them.
- Files with delegates or other nested components set `pragma ComponentBehavior: Bound`, so that those can refer to ids outside them, and delegates take their model data as `required` properties.
- Where qmllint can't follow the code, such as a property read from an item that a Loader loaded by URL, a `// qmllint disable <category>` comment says so, with why.

### Translations

The translations are in `src/locales`, and `en-CA.json` has every key. `just gen` and the build generate `src/qml/core/Strings.qml` from it, which is checked in: a singleton with a typed property for each translation, so that `Strings.game_select.title` is the translation of `game_select.title`, and the editor completes the keys. A translation with `{{ placeholders }}` is a function of them, like `Strings.status_bar.downloads_tracker(count)`. A key that is a keyword or the name of a JavaScript global, like `delete`, gets a `_` after it, and one that starts with a digit gets one before it. `I18n.t(key, args)` is for keys that are only known at runtime.

### The bridge to the UI

The UI reads data from and calls into the backend through QObjects that `src/qt` defines in Rust with CXX-Qt. QML sees them as ordinary types, whose properties QML binds to and qmllint checks, and whose go to definition opens their declarations in `src/qt`:

- Each part of the backend has a singleton: `Profiles`, `Settings`, `ModIndex`, `Tasks`, `Console`, `Games`, and `App` for the rest, like the window's state and the translations. They give QML objects, like each `Profile`, `Mod`, or `Task`.
- The objects that a singleton keeps, like the profiles and the mods installed in them, are its children, and are updated in place when they change, so that what QML refers to stays valid and can be compared by identity. Those that are removed are kept, as QML may still be showing them in an array, which can't tell it they're gone first.
- Values are properties, `CONSTANT` where they never change. A list of objects is a `QObjectList` (`src/qt/object_list.rs`), which QML knows as a `list<QtObject>`, and a map is a `QVariantMap` (`src/qt/variant_map.rs`). A long list that QML shows, like a game's log, is a `QAbstractListModel` (`ConsoleLog` in `src/qt/console.rs`).
- Enums are `#[qenum]`s of their objects, like `Task.Running`. Their names must be unique across the bridges, as must the `rust_name`s of the `make_unique`s that make objects.
- A method that does something asynchronously returns a `Call` (`src/qt/call.rs`), which `call::spawn` makes of a future, or `call::spawn_with_task` of one that creates a task, whose progress becomes the call's. QML turns it into a promise with `Async.promise(call)`, awaits it with `yield` in `Async.co`, or gives it to an `AsyncButton`, which shows the progress of its `taskId`'s `Task`. It rejects with the backend's error, which has every message in its cause chain and a backtrace. What it resolves to is what the future returns, as its `call::Output`: nothing, a value, or objects, like a page of the mod index (`Vec<ModInfo>` is a list of `Mod`s), which become QML's once it takes them, and live as long as QML refers to them.
- The backend sends what happens, like a task's progress or a game's log, as typed `Events` of its `AppHandle`, which the singletons subscribe to. An `Outbox` (`src/qt/outbox.rs`) delivers them on the Qt thread in order, with a flood of them delivered at once.
- It's safe Rust, but for creating the objects through CXX-Qt, handing out pointers to them, and changing a singleton's children, which `Child` (`src/qt/child.rs`) does.

A setting is a field of `Settings` in `src/settings/mod.rs`, with its section, default, and translations, under `settings` in `src/locales`. The settings page lists it, set with what its type's `SettingType` decides, like a toggle for a `bool`, without changes to the bridge. The `Settings` singleton has properties only for the settings that the rest of the UI reads.

A new bridge file is added to the list of bridges in `crates/qml-gen/src/lib.rs`, which both the build and `just gen` read, and to `src/qt/mod.rs`. A singleton that receives events is created as the app starts, in `Main.qml`, so that none are sent before it subscribes.

## Contributing

This project is under rapid active development, and is not yet in release. If you wish you help, there are todo lists in the issues tab, or if you are interested in getting in contact, you can friend me on Discord `@emperor_of_bluegaria`.
