#!/usr/bin/env python3
"""Bundles the Linux build as an AppImage, with the Qt it was built with.

This copies only the Qt plugins and QML modules that the app uses, as bundle-windows.py does, then
has linuxdeploy bundle the libraries that those and the executable import and create the AppImage.
linuxdeploy's own Qt plugin would instead copy every plugin of each kind, every Qt Quick Controls
style, and Qt's translations.

Usage: [QMAKE=<qmake>] bundle-linux.py <manderrow> <output AppImage>

linuxdeploy must be on the PATH.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The plugins that Qt loads at runtime, by directory, or None for all of a directory's plugins
PLUGINS = {
    "platforms": ["libqxcb", "libqwayland"],
    "xcbglintegrations": None,
    # The window is frameless, so Wayland's client-side decorations are not needed.
    "wayland-shell-integration": ["libxdg-shell"],
    "wayland-graphics-integration-client": ["libqt-plugin-wayland-egl"],
    "platforminputcontexts": None,
    "platformthemes": ["libqxdgdesktopportal"],
    # The formats of mod icons and of images in mods' descriptions
    "imageformats": ["libqgif", "libqjpeg", "libqsvg", "libqwebp"],
    "iconengines": None,
}


def qmake_query(key: str) -> Path:
    qmake = os.environ.get("QMAKE", "qmake")
    output = subprocess.run([qmake, "-query", key], capture_output=True, text=True, check=True)
    return Path(output.stdout.strip())


def copy_files(src: Path, dst: Path, names: list[str] | None = None):
    dst.mkdir(parents=True, exist_ok=True)
    for file in sorted(src.iterdir()):
        if not file.is_file():
            continue
        if names is not None and file.name.split(".")[0] not in names:
            continue
        shutil.copy2(file, dst)


def main():
    exe, out = Path(sys.argv[1]), Path(sys.argv[2])
    qt_plugins = qmake_query("QT_INSTALL_PLUGINS")
    qt_qml = qmake_query("QT_INSTALL_QML")
    qmlimportscanner = qmake_query("QT_HOST_LIBEXECS") / "qmlimportscanner"

    # An AppImage left from before would hide linuxdeploy failing to create one.
    out.unlink(missing_ok=True)

    with tempfile.TemporaryDirectory() as tmp:
        appdir = Path(tmp) / "AppDir"
        usr = appdir / "usr"
        (usr / "bin").mkdir(parents=True)
        shutil.copy2(exe, usr / "bin" / "manderrow")
        # The app finds the agent in ../lib/manderrow, relative to the executable.
        (usr / "lib" / "manderrow").mkdir(parents=True)
        shutil.copy2(ROOT / "agent" / "zig-out" / "libmanderrow_agent", usr / "lib" / "manderrow")

        for directory, names in PLUGINS.items():
            if (qt_plugins / directory).is_dir():
                copy_files(qt_plugins / directory, usr / "plugins" / directory, names)

        # Each module the app imports is listed separately, including those nested in another's
        # directory, so only each module's own files are copied.
        modules = json.loads(
            subprocess.run(
                [qmlimportscanner, "-rootPath", ROOT / "src" / "qml", "-importPath", qt_qml],
                capture_output=True,
                text=True,
                check=True,
            ).stdout
        )
        for module in modules:
            path = Path(module.get("path", ""))
            if module.get("type") == "module" and path.is_relative_to(qt_qml):
                copy_files(path, usr / "qml" / path.relative_to(qt_qml))

        (usr / "bin" / "qt.conf").write_text("[Paths]\nPrefix=..\nPlugins=plugins\nQmlImports=qml\n")

        # linuxdeploy bundles the libraries that the executable, the plugins and the QML modules
        # import, except those that every system has, such as glibc's and the graphics drivers'.
        # By itself it only looks at the files in the AppDir's bin and lib directories, and it
        # does not search the directories it is given recursively. Its AppImages extract
        # themselves to run, as they would otherwise need FUSE.
        qt_libraries = sorted([*(usr / "plugins").rglob("*.so"), *(usr / "qml").rglob("*.so")])
        subprocess.run(
            [
                "linuxdeploy",
                "--appdir", appdir,
                *(f"--deploy-deps-only={library}" for library in qt_libraries),
                "--desktop-file", ROOT / "src" / "platforms" / "linux" / "manderrow.desktop",
                "--icon-file", ROOT / "icons" / "128x128@2x.png",
                "--icon-filename", "manderrow",
                "--output", "appimage",
            ],
            env={
                **os.environ,
                "APPIMAGE_EXTRACT_AND_RUN": "1",
                "LDAI_OUTPUT": str(out.resolve()),
                # The libraries of the Qt that the app was built with, rather than the system's
                "LD_LIBRARY_PATH": os.pathsep.join(
                    filter(None, [str(qmake_query("QT_INSTALL_LIBS")), os.environ.get("LD_LIBRARY_PATH")])
                ),
            },
            check=True,
        )

        for file in sorted(appdir.rglob("*")):
            if file.is_file():
                print(file.relative_to(appdir))

    # linuxdeploy succeeds even when it fails to create the AppImage.
    if not out.is_file():
        sys.exit(f"linuxdeploy did not create {out}")


if __name__ == "__main__":
    main()
