"""Build a local VSIX with Microsoft's packager and the project's licenses."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile

VSCE_VERSION = "4.1.0"
SOURCE = Path(__file__).resolve().parent
REPOSITORY = SOURCE.parents[1]
PAYLOAD = ("package.json", "extension.cjs", "bridge.cjs", "providers.cjs")


def main():
    manifest = json.loads((SOURCE / "package.json").read_text())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=REPOSITORY / "dist" / f"vizzi-vscode-{manifest['version']}.vsix")
    arguments = parser.parse_args()
    output = arguments.out.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    licenses = sorted(REPOSITORY.glob("LICENSE*"))
    if not licenses:
        raise SystemExit("No project license files found")
    with tempfile.TemporaryDirectory(prefix="vizzi-vsix-") as directory:
        stage = Path(directory)
        for name in PAYLOAD:
            shutil.copyfile(SOURCE / name, stage / name)
        shutil.copyfile(SOURCE / "INSTALL.md", stage / "README.md")
        for license_file in licenses:
            shutil.copyfile(license_file, stage / license_file.name)
        if not (stage / "LICENSE").exists():
            shutil.copyfile(REPOSITORY / "LICENSE-APACHE", stage / "LICENSE")
        subprocess.run([
            "npx", "--yes", f"--package=@vscode/vsce@{VSCE_VERSION}", "vsce", "package",
            "--no-dependencies", "--allow-missing-repository", "--out", str(output),
        ], cwd=stage, check=True)
    with zipfile.ZipFile(output) as archive:
        for name in PAYLOAD:
            if archive.read(f"extension/{name}") != (SOURCE / name).read_bytes():
                raise SystemExit(f"Packaged source mismatch: {name}")
        for license_file in licenses:
            if archive.read(f"extension/{license_file.name}") != license_file.read_bytes():
                raise SystemExit(f"Packaged license mismatch: {license_file.name}")
    print(output)


if __name__ == "__main__":
    main()
