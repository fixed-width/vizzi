"""Version-bound release archives and checksum manifests; Python standard library only."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile
import tomllib
import zipfile
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[2]
TARGETS = (
    'x86_64-unknown-linux-musl',
    'aarch64-unknown-linux-musl',
    'x86_64-apple-darwin',
    'aarch64-apple-darwin',
)
VSIX_TARGETS = dict(zip(('linux-x64', 'linux-arm64', 'darwin-x64', 'darwin-arm64'), TARGETS))


def extension_version(root):
    return json.loads((root / 'editors/vscode/package.json').read_text())['version']


def validate(tag, root=ROOT):
    version = tomllib.loads((root / 'crates/vizzi/Cargo.toml').read_text())['package']['version']
    if not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?', tag) or tag != 'v' + version:
        raise ValueError(f'Release tag must be v{version}; got {tag!r}')
    return version


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def project_licenses(root):
    licenses = sorted(path for path in root.glob('LICENSE*') if path.is_file())
    if not licenses:
        raise ValueError('Release input must include a project license')
    return licenses


def package(tag, target, binary, output, root=ROOT):
    validate(tag, root)
    if target not in TARGETS:
        raise ValueError(f'Unsupported release target: {target}')
    if not binary.is_file() or binary.is_symlink():
        raise ValueError('Release input must be a regular executable file')
    licenses = project_licenses(root)
    output.mkdir(parents=True, exist_ok=True)
    prefix = f'vizzi-{tag}-{target}'
    archive = output / (prefix + '.tar.gz')
    with tarfile.open(archive, 'x:gz') as contents:
        executable = contents.gettarinfo(str(binary), arcname=prefix + '/vizzi')
        executable.mode = 0o755
        executable.uid = executable.gid = 0
        executable.uname = executable.gname = ''
        with binary.open('rb') as source:
            contents.addfile(executable, source)
        documents = {
            'BUILD.json': json.dumps({'tag': tag, 'target': target}, indent=2) + '\n',
            'README.txt': (
                'Vizzi - visual codebase inspection\n\n'
                'Run ./vizzi from a project directory, or use ./vizzi --root /path/to/project.\n'
                'Use ./vizzi --help for options. No compiler is needed to run this executable.\n'
                'Language servers are optional and installed separately.\n\n'
                f'Documentation: https://github.com/fixed-width/vizzi/tree/{tag}/docs\n'
            ),
        }
        for name, text in documents.items():
            data = text.encode()
            info = tarfile.TarInfo(prefix + '/' + name)
            info.mode = 0o644
            info.size = len(data)
            contents.addfile(info, io.BytesIO(data))
        for path in [*licenses, root / 'NOTICE']:
            if path.is_file():
                contents.add(path, arcname=prefix + '/' + path.name, recursive=False)
        notice = root / 'crates/vizzi/web/vendor/bubblesets-LICENSE.txt'
        if notice.is_file():
            contents.add(notice, arcname=prefix + '/THIRD_PARTY_NOTICES/bubblesets-LICENSE.txt')
    checksum = Path(str(archive) + '.sha256')
    with checksum.open('x') as destination:
        destination.write(f'{digest(archive)}  {archive.name}\n')
    return archive


def bundle(tag, directory, root=ROOT):
    validate(tag, root)
    licenses = project_licenses(root)
    archives = {f'vizzi-{tag}-{target}.tar.gz' for target in TARGETS}
    version = extension_version(root)
    extensions = {f'vizzi-vscode-{version}-{platform}.vsix': (platform, target)
                  for platform, target in VSIX_TARGETS.items()}
    names = archives | extensions.keys()
    expected = names | {name + '.sha256' for name in names}
    actual = {path.name for path in directory.iterdir()}
    if actual - {'SHA256SUMS'} != expected:
        raise ValueError(f'Incomplete or unexpected release bundle: {sorted(actual)}')
    lines = []
    for name in sorted(names):
        path = directory / name
        expected_line = f'{digest(path)}  {name}\n'
        if (directory / (name + '.sha256')).read_text() != expected_line:
            raise ValueError(f'Checksum mismatch: {name}')
        if name in extensions:
            with zipfile.ZipFile(path) as contents:
                identity = ElementTree.fromstring(contents.read('extension.vsixmanifest')).find('.//{*}Identity')
                if (identity is None or identity.get('TargetPlatform') != extensions[name][0]
                        or identity.get('Version') != version):
                    raise ValueError(f'Extension platform/version differs: {name}')
                manifest = json.loads(contents.read('extension/package.json'))
                if manifest.get('version') != version or manifest.get('name') != 'vizzi':
                    raise ValueError(f'Extension manifest differs: {name}')
                for license in licenses:
                    if contents.read('extension/' + license.name) != license.read_bytes():
                        raise ValueError(f'Extension project license missing or changed: {name}/{license.name}')
                prefix = f'vizzi-{tag}-{extensions[name][1]}'
                with tarfile.open(directory / (prefix + '.tar.gz')) as native:
                    binary = native.extractfile(prefix + '/vizzi').read()
                if contents.read('extension/bin/vizzi') != binary:
                    raise ValueError(f'Extension executable differs from native archive: {name}')
        else:
            prefix = name.removesuffix('.tar.gz')
            with tarfile.open(path) as contents:
                members = {member.name: member for member in contents.getmembers()}
                for license in licenses:
                    member = members.get(prefix + '/' + license.name)
                    expected_license = license.read_bytes()
                    if (member is None or not member.isfile()
                            or contents.extractfile(member).read(len(expected_license) + 1) != expected_license):
                        raise ValueError(f'Archive project license missing or changed: {name}/{license.name}')
        lines.append(expected_line)
    manifest = directory / 'SHA256SUMS'
    contents = ''.join(lines)
    if manifest.exists() and manifest.read_text() != contents:
        raise ValueError('Existing checksum manifest differs')
    manifest.write_text(contents)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    check = commands.add_parser('validate')
    check.add_argument('tag')
    create = commands.add_parser('package')
    create.add_argument('tag')
    create.add_argument('target', choices=TARGETS)
    create.add_argument('binary', type=Path)
    create.add_argument('output', type=Path)
    collect = commands.add_parser('bundle')
    collect.add_argument('tag')
    collect.add_argument('directory', type=Path)
    args = parser.parse_args()
    if args.command == 'validate':
        print(validate(args.tag))
    elif args.command == 'package':
        print(package(args.tag, args.target, args.binary, args.output))
    else:
        print(bundle(args.tag, args.directory))


if __name__ == '__main__':
    main()
