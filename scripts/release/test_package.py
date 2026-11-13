import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('release_package', Path(__file__).with_name('package.py'))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


def extension_fixtures(root, output):
    manifest = root / 'editors/vscode/package.json'
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text('{"name":"vizzi","version":"0.2.0"}')
    for platform in release.VSIX_TARGETS:
        archive = output / f'vizzi-vscode-0.2.0-{platform}.vsix'
        with zipfile.ZipFile(archive, 'w') as contents:
            contents.writestr('extension.vsixmanifest',
                             f'<PackageManifest><Metadata><Identity Version="0.2.0" TargetPlatform="{platform}"/></Metadata></PackageManifest>')
            contents.write(manifest, 'extension/package.json')
            contents.write(root / 'LICENSE-APACHE', 'extension/LICENSE-APACHE')
            binary = root / 'built-vizzi' if (root / 'built-vizzi').exists() else root / 'binary'
            contents.write(binary, 'extension/bin/vizzi')
        Path(str(archive) + '.sha256').write_text(f'{release.digest(archive)}  {archive.name}\n')


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        manifest = self.root / 'crates/vizzi/Cargo.toml'
        manifest.parent.mkdir(parents=True)
        manifest.write_text('[package]\nversion="0.1.0"\n')
        (self.root / 'LICENSE-APACHE').write_text('Fixture project license')
        self.binary = self.root / 'built-vizzi'
        self.binary.write_bytes(b'fixture executable')
        self.output = self.root / 'dist'
        self.output.mkdir()
        extension_fixtures(self.root, self.output)

    def test_tags_must_be_safe_and_match_the_package_version(self):
        self.assertEqual(release.validate('v0.1.0', self.root), '0.1.0')
        for tag in ['v0.2.0', '0.1.0', 'v0.1.0/escape', 'v0.1.0\n', 'v0.1.0;echo bad']:
            with self.assertRaises(ValueError):
                release.validate(tag, self.root)

    def test_archive_contains_executable_metadata_and_matching_checksum(self):
        archive = release.package('v0.1.0', release.TARGETS[0], self.binary, self.output, self.root)
        with tarfile.open(archive) as contents:
            prefix = archive.name.removesuffix('.tar.gz')
            executable = contents.getmember(prefix + '/vizzi')
            self.assertEqual(executable.mode, 0o755)
            self.assertEqual(contents.extractfile(executable).read(), self.binary.read_bytes())
            metadata = json.load(contents.extractfile(prefix + '/BUILD.json'))
            self.assertEqual(metadata['tag'], 'v0.1.0')
            self.assertEqual(metadata['target'], release.TARGETS[0])
            readme = contents.extractfile(prefix + '/README.txt').read().decode()
            self.assertIn('https://github.com/fixed-width/vizzi/tree/v0.1.0/docs', readme)
        expected = hashlib.sha256(archive.read_bytes()).hexdigest() + '  ' + archive.name + '\n'
        self.assertEqual(Path(str(archive) + '.sha256').read_text(), expected)
        with self.assertRaises(FileExistsError):
            release.package('v0.1.0', release.TARGETS[0], self.binary, self.output, self.root)

    def test_archive_includes_project_license_variants_and_notice(self):
        names = ['LICENSE', 'LICENSE.md', 'LICENSE.txt', 'LICENSE-APACHE', 'LICENSE-MIT', 'NOTICE']
        for name in names:
            (self.root / name).write_text('Fixture notice: ' + name)
        archive = release.package('v0.1.0', release.TARGETS[0], self.binary, self.output, self.root)
        with tarfile.open(archive) as contents:
            prefix = archive.name.removesuffix('.tar.gz')
            for name in names:
                self.assertEqual(contents.extractfile(prefix + '/' + name).read(),
                                 (self.root / name).read_bytes())

    def test_packaging_requires_a_project_license_before_creating_an_archive(self):
        (self.root / 'LICENSE-APACHE').unlink()
        with self.assertRaisesRegex(ValueError, 'project license'):
            release.package('v0.1.0', release.TARGETS[0], self.binary, self.output, self.root)
        self.assertFalse(list(self.output.glob('*.tar.gz')))

    def test_complete_bundle_is_required_and_tampering_is_rejected(self):
        archives = []
        for target in release.TARGETS:
            archives.append(release.package('v0.1.0', target, self.binary, self.output, self.root))
        sums = release.bundle('v0.1.0', self.output, self.root)
        self.assertEqual(len(sums.read_text().splitlines()), 8)
        archives[0].write_bytes(b'tampered')
        with self.assertRaises(ValueError):
            release.bundle('v0.1.0', self.output, self.root)

    def test_bundle_rejects_missing_or_changed_project_license_even_with_a_matching_checksum(self):
        for damage in ['missing', 'changed']:
            with self.subTest(damage=damage):
                output = self.root / damage
                archives = [release.package('v0.1.0', target, self.binary, output, self.root)
                            for target in release.TARGETS]
                extension_fixtures(self.root, output)
                archive = archives[0]
                members = []
                with tarfile.open(archive) as contents:
                    for member in contents.getmembers():
                        data = contents.extractfile(member).read()
                        if member.name.endswith('/LICENSE-APACHE'):
                            if damage == 'missing':
                                continue
                            data = b'changed project license'
                            member.size = len(data)
                        members.append((member, data))
                with tarfile.open(archive, 'w:gz') as contents:
                    for member, data in members:
                        contents.addfile(member, io.BytesIO(data))
                Path(str(archive) + '.sha256').write_text(f'{release.digest(archive)}  {archive.name}\n')
                with self.assertRaisesRegex(ValueError, 'project license'):
                    release.bundle('v0.1.0', output, self.root)

    def test_missing_targets_and_unknown_targets_are_rejected(self):
        with self.assertRaises(ValueError):
            release.bundle('v0.1.0', self.output, self.root)
        with self.assertRaises(ValueError):
            release.package('v0.1.0', '../../bad', self.binary, self.output, self.root)

    def test_bundle_requires_all_extensions_and_matching_native_executables(self):
        for target in release.TARGETS:
            release.package('v0.1.0', target, self.binary, self.output, self.root)
        extension = next(self.output.glob('*.vsix'))
        original = extension.read_bytes()
        extension.unlink()
        with self.assertRaisesRegex(ValueError, 'Incomplete'):
            release.bundle('v0.1.0', self.output, self.root)
        extension.write_bytes(original)
        with zipfile.ZipFile(extension) as contents:
            entries = {name: contents.read(name) for name in contents.namelist()}
        entries['extension/bin/vizzi'] = b'wrong executable'
        with zipfile.ZipFile(extension, 'w') as contents:
            for name, data in entries.items():
                contents.writestr(name, data)
        Path(str(extension) + '.sha256').write_text(f'{release.digest(extension)}  {extension.name}\n')
        with self.assertRaisesRegex(ValueError, 'executable differs'):
            release.bundle('v0.1.0', self.output, self.root)

    def test_extension_platform_metadata_must_match_the_release_asset(self):
        for target in release.TARGETS:
            release.package('v0.1.0', target, self.binary, self.output, self.root)
        extension = next(self.output.glob('*.vsix'))
        with zipfile.ZipFile(extension) as contents:
            entries = {name: contents.read(name) for name in contents.namelist()}
        entries['extension.vsixmanifest'] = b'<PackageManifest><Metadata><Identity Version="0.2.0" TargetPlatform="wrong"/></Metadata></PackageManifest>'
        with zipfile.ZipFile(extension, 'w') as contents:
            for name, data in entries.items():
                contents.writestr(name, data)
        Path(str(extension) + '.sha256').write_text(f'{release.digest(extension)}  {extension.name}\n')
        with self.assertRaisesRegex(ValueError, 'platform/version differs'):
            release.bundle('v0.1.0', self.output, self.root)


if __name__ == '__main__':
    unittest.main()
