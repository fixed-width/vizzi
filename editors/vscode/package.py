"""Build a local VSIX with Microsoft's packager and the project's licenses."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import zipfile

VSCE_VERSION = "4.1.0"
SOURCE = Path(__file__).resolve().parent
REPOSITORY = SOURCE.parents[1]
PAYLOAD = ("package.json", "extension.cjs", "bridge.cjs", "providers.cjs", "launcher.cjs", "workspace-view.cjs")
TARGETS = ("linux-x64", "linux-arm64", "darwin-x64", "darwin-arm64")


def native_target():
    system = platform.system().lower()
    architecture = {"x86_64": "x64", "amd64": "x64", "aarch64": "arm64", "arm64": "arm64"}.get(platform.machine().lower())
    target = f"{system}-{architecture}"
    if target not in TARGETS:
        raise ValueError(f"Unsupported VSIX platform: {target}")
    return target


def main():
    manifest = json.loads((SOURCE / "package.json").read_text())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, default=native_target())
    parser.add_argument("--binary", type=Path, default=REPOSITORY / "target/release/vizzi")
    parser.add_argument("--out", type=Path)
    arguments = parser.parse_args()
    binary = arguments.binary.resolve()
    if not binary.is_file():
        raise SystemExit("Build the native executable first: cargo build --locked --release -p vizzi")
    output = (arguments.out or REPOSITORY / "dist" / f"vizzi-vscode-{manifest['version']}-{arguments.target}.vsix").resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    licenses = sorted(REPOSITORY.glob("LICENSE*"))
    if not licenses:
        raise SystemExit("No project license files found")
    with tempfile.TemporaryDirectory(prefix="vizzi-vsix-") as directory:
        stage = Path(directory)
        for name in PAYLOAD:
            shutil.copyfile(SOURCE / name, stage / name)
        (stage / "bin").mkdir()
        shutil.copyfile(binary, stage / "bin/vizzi")
        (stage / "bin/vizzi").chmod(0o755)
        shutil.copyfile(SOURCE / "INSTALL.md", stage / "README.md")
        for license_file in licenses:
            shutil.copyfile(license_file, stage / license_file.name)
        if not (stage / "LICENSE").exists():
            shutil.copyfile(REPOSITORY / "LICENSE-APACHE", stage / "LICENSE")
        notice = REPOSITORY / "crates/vizzi/web/vendor/bubblesets-LICENSE.txt"
        (stage / "THIRD_PARTY_NOTICES").mkdir()
        shutil.copyfile(notice, stage / "THIRD_PARTY_NOTICES/bubblesets-LICENSE.txt")
        subprocess.run([
            "npx", "--yes", f"--package=@vscode/vsce@{VSCE_VERSION}", "vsce", "package",
            "--target", arguments.target, "--no-dependencies", "--allow-missing-repository", "--out", str(output),
        ], cwd=stage, check=True)
    with zipfile.ZipFile(output) as archive:
        if archive.read("extension/bin/vizzi") != binary.read_bytes():
            raise SystemExit("Packaged executable mismatch")
        if archive.read("extension/readme.md") != (SOURCE / "INSTALL.md").read_bytes():
            raise SystemExit("Packaged documentation mismatch")
        for name in PAYLOAD:
            if archive.read(f"extension/{name}") != (SOURCE / name).read_bytes():
                raise SystemExit(f"Packaged source mismatch: {name}")
        for license_file in licenses:
            if archive.read(f"extension/{license_file.name}") != license_file.read_bytes():
                raise SystemExit(f"Packaged license mismatch: {license_file.name}")
    checksum = hashlib.sha256(output.read_bytes()).hexdigest()
    Path(str(output) + ".sha256").write_text(f"{checksum}  {output.name}\n")
    print(output)


if __name__ == "__main__":
    main()
