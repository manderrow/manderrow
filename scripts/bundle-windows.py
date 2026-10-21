#!/usr/bin/env python3
"""Bundles the Windows build with everything it loads at runtime, as windeployqt does.

Unlike windeployqt, this also runs on Linux, so it deploys builds cross-compiled from there the same
way as native ones. It copies the Qt plugins and QML modules that the app uses, then every library
that those and the executable import from Qt or from Visual Studio's redistributable MSVC runtime.

Usage: [QMAKE=<Qt for Windows' qmake>] bundle-windows.py <manderrow.exe> <output directory>
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The plugins that Qt loads at runtime, by directory, or None for all of a directory's plugins
PLUGINS = {
    "platforms": ["qwindows"],
    # The formats of mod icons and of images in mods' descriptions
    "imageformats": ["qgif", "qjpeg", "qsvg", "qwebp"],
    "iconengines": None,
    "networkinformation": None,
    # OpenSSL is not bundled, so only the backends that use the system's TLS
    "tls": ["qschannelbackend", "qcertonlybackend"],
}

# Libraries that Qt loads dynamically rather than importing
DYNAMIC_LIBRARIES = ["d3dcompiler_47.dll", "opengl32sw.dll"]

VS_CHANNEL = "https://aka.ms/vs/17/release/channel"


def qmake_query(key: str) -> Path:
    qmake = os.environ.get("QMAKE", "qmake")
    output = subprocess.run([qmake, "-query", key], capture_output=True, text=True, check=True)
    return Path(output.stdout.strip())


def is_debug(path: Path) -> bool:
    """Whether the library is the debug build of another beside it, which Qt names with a d."""
    return path.stem.endswith("d") and path.with_name(path.stem[:-1] + path.suffix).exists()


def copy_release_files(src: Path, dst: Path, names: list[str] | None = None):
    dst.mkdir(parents=True, exist_ok=True)
    for file in sorted(src.iterdir()):
        if not file.is_file() or file.suffix == ".pdb" or is_debug(file):
            continue
        if names is not None and file.stem not in names:
            continue
        shutil.copy2(file, dst)


def fetch_msvc_runtime(dst: Path) -> Path:
    """Downloads the latest redistributable MSVC runtime, from the Visual Studio manifest that xwin
    downloads the CRT's libraries from."""

    def fetch_json(url: str):
        with urllib.request.urlopen(url) as response:
            return json.load(response)

    channel = fetch_json(VS_CHANNEL)
    manifest = next(
        item for item in channel["channelItems"] if item["id"] == "Microsoft.VisualStudio.Manifests.VisualStudio"
    )
    packages = fetch_json(manifest["payloads"][0]["url"])["packages"]
    redist = max(
        (p for p in packages if re.fullmatch(r"Microsoft\.VC\.[\d.]+\.CRT\.Redist\.X64\.base", p["id"], re.I)),
        key=lambda p: tuple(int(part) for part in p["version"].split(".")),
    )
    vsix = dst / "redist.vsix"
    urllib.request.urlretrieve(redist["payloads"][0]["url"], vsix)
    with zipfile.ZipFile(vsix) as archive:
        archive.extractall(dst)
    return next(dst.glob("Contents/VC/Redist/MSVC/*/x64/Microsoft.VC143.CRT"))


def imports(binary: Path) -> list[str]:
    output = subprocess.run(
        ["llvm-readobj", "--coff-imports", binary], capture_output=True, text=True, check=True
    ).stdout
    return re.findall(r"^\s*Name: (\S+\.dll)\s*$", output, re.M | re.I)


def main():
    exe, out = Path(sys.argv[1]), Path(sys.argv[2])
    qt_bins = qmake_query("QT_INSTALL_BINS")
    qt_plugins = qmake_query("QT_INSTALL_PLUGINS")
    qt_qml = qmake_query("QT_INSTALL_QML")
    # Found with which, for its .exe on Windows
    qmlimportscanner = shutil.which("qmlimportscanner", path=str(qmake_query("QT_HOST_LIBEXECS")))
    if qmlimportscanner is None:
        sys.exit("qmlimportscanner not found")

    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    shutil.copy2(exe, out)
    shutil.copy2(ROOT / "agent" / "zig-out" / "libmanderrow_agent", out)

    for directory, names in PLUGINS.items():
        if (qt_plugins / directory).is_dir():
            copy_release_files(qt_plugins / directory, out / "plugins" / directory, names)

    # Each module the app imports is listed separately, including those nested in another's
    # directory, so only each module's own files are copied.
    modules = json.loads(
        subprocess.run(
            [qmlimportscanner, "-rootPath", ROOT / "src-qt" / "qml", "-importPath", qt_qml],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    for module in modules:
        path = Path(module.get("path", ""))
        if module.get("type") == "module" and path.is_relative_to(qt_qml):
            copy_release_files(path, out / "qml" / path.relative_to(qt_qml))

    (out / "qt.conf").write_text("[Paths]\nPrefix=.\nPlugins=plugins\nQmlImports=qml\n")

    for name in DYNAMIC_LIBRARIES:
        if (qt_bins / name).exists():
            shutil.copy2(qt_bins / name, out)

    with tempfile.TemporaryDirectory() as tmp:
        msvc_runtime = fetch_msvc_runtime(Path(tmp))
        # Anything else that is imported, such as kernel32.dll or the UCRT, is part of Windows.
        available = {
            file.name.lower(): file
            for directory in [qt_bins, msvc_runtime]
            for file in directory.glob("*.dll")
            if not is_debug(file)
        }
        queue = [file for file in out.rglob("*") if file.suffix.lower() in (".exe", ".dll")]
        queue.append(out / "libmanderrow_agent")
        bundled = {file.name.lower() for file in queue}
        while queue:
            for name in imports(queue.pop()):
                library = available.get(name.lower())
                if library is not None and name.lower() not in bundled:
                    bundled.add(name.lower())
                    shutil.copy2(library, out)
                    queue.append(out / library.name)

    for file in sorted(out.rglob("*")):
        if file.is_file():
            print(file.relative_to(out))


if __name__ == "__main__":
    main()
