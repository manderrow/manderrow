#!/usr/bin/env python3
"""Bundles the macOS build as an app in a disk image, with the Qt it was built with.

macdeployqt copies the Qt frameworks, plugins and QML modules that the app uses into the app and
points the executable at them, rather than at the Qt it was built with. As that changes the
binaries, the app is then signed again, ad hoc, since arm64 Macs do not run unsigned code.

The app is put in a disk image, as GitHub's artifacts would drop the executable permissions and the
frameworks' symbolic links.

Usage: [QMAKE=<qmake>] bundle-macos.py <manderrow> <version> <output dmg>
"""

import os
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

BUNDLE_IDENTIFIER = "io.github.manderrow"


def qmake_query(key: str) -> Path:
    qmake = os.environ.get("QMAKE", "qmake")
    output = subprocess.run([qmake, "-query", key], capture_output=True, text=True, check=True)
    return Path(output.stdout.strip())


def main():
    exe, version, out = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])

    out.unlink(missing_ok=True)

    with tempfile.TemporaryDirectory() as tmp:
        staging = Path(tmp) / "Manderrow"
        app = staging / "Manderrow.app"
        contents = app / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        (contents / "Resources").mkdir()
        shutil.copy2(exe, contents / "MacOS" / "manderrow")
        # The app finds the agent beside the executable.
        agent = contents / "MacOS" / "libmanderrow_agent"
        shutil.copy2(ROOT / "agent" / "zig-out" / "libmanderrow_agent", agent)
        shutil.copy2(ROOT / "icons" / "icon.icns", contents / "Resources" / "manderrow.icns")

        with open(contents / "Info.plist", "wb") as file:
            plistlib.dump(
                {
                    "CFBundleDevelopmentRegion": "en",
                    "CFBundleDisplayName": "Manderrow",
                    "CFBundleExecutable": "manderrow",
                    "CFBundleIconFile": "manderrow.icns",
                    "CFBundleIdentifier": BUNDLE_IDENTIFIER,
                    "CFBundleInfoDictionaryVersion": "6.0",
                    "CFBundleName": "Manderrow",
                    "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version,
                    "CFBundleVersion": version,
                    "LSApplicationCategoryType": "public.app-category.games",
                    # The same as MACOSX_DEPLOYMENT_TARGET in the build workflow
                    "LSMinimumSystemVersion": "13.0",
                    "NSHighResolutionCapable": True,
                    "NSPrincipalClass": "NSApplication",
                },
                file,
            )

        # This also writes the qt.conf that points Qt at the bundled plugins and QML modules.
        subprocess.run(
            [
                qmake_query("QT_INSTALL_BINS") / "macdeployqt",
                app,
                f"-qmldir={ROOT / 'src' / 'qml'}",
                "-verbose=1",
            ],
            check=True,
        )

        # --deep signs the frameworks and plugins, but not other code in Contents/MacOS.
        subprocess.run(["codesign", "--force", "--sign", "-", agent], check=True)
        subprocess.run(["codesign", "--force", "--deep", "--sign", "-", app], check=True)
        subprocess.run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", app], check=True)

        for file in sorted(app.rglob("*")):
            if file.is_file() and not file.is_symlink():
                print(file.relative_to(staging))

        # So that the app can be dragged into Applications from the disk image
        (staging / "Applications").symlink_to("/Applications")
        # hdiutil sometimes fails on GitHub's runners with "Resource busy", so it is retried.
        for attempt in range(5):
            result = subprocess.run(
                [
                    "hdiutil", "create",
                    "-volname", "Manderrow",
                    "-srcfolder", staging,
                    "-format", "UDZO",
                    "-ov",
                    out,
                ]
            )
            if result.returncode == 0:
                break
            time.sleep(5)
        else:
            sys.exit(f"hdiutil failed to create {out}")


if __name__ == "__main__":
    main()
