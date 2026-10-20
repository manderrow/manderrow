# Manderrow

[![build](https://github.com/Jack-the-Pro101/manderrow/actions/workflows/build.yml/badge.svg)](https://github.com/Jack-the-Pro101/manderrow/actions/workflows/build.yml)

- [About](#about)
- [Features and Improvements](#features-and-improvements)
- [Building](#building)
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
- [Zig](https://ziglang.org/) 0.15, which builds the injection agent
- Qt 6.10 or newer, with the Qt Quick, Qt Quick Controls, and Qt SVG development packages. Set `QMAKE` to the path of `qmake` if it isn't on your `PATH`.
  - Fedora: `sudo dnf install qt6-qtbase-devel qt6-qtbase-private-devel qt6-qtdeclarative-devel qt6-qtsvg-devel`
  - Debian/Ubuntu: `sudo apt install qt6-base-dev qt6-base-private-dev qt6-declarative-dev qt6-svg-dev`

[mise](https://mise.jdx.dev/) installs the Rust and Zig versions with `mise install`. Then:

```sh
just run    # build and run a debug build
just build  # build a release build
just test   # run the tests
```

The app lives in `src-qt`: the backend is in `src-qt/src`, the bridge to the UI in `src-qt/src/qt`, and the UI in `src-qt/qml`.

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
  cargo run --manifest-path src-qt/Cargo.toml
```

## Contributing

This project is under rapid active development, and is not yet in release. If you wish you help, there are todo lists in the issues tab, or if you are interested in getting in contact, you can friend me on Discord `@emperor_of_bluegaria`.
