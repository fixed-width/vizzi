import hashlib
import importlib.util
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('release_package', Path(__file__).with_name('package.py'))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        manifest = self.root / 'crates/vizzi/Cargo.toml'
        manifest.parent.mkdir(parents=True)
        manifest.write_text('[package]\nversion="0.1.0"\n')
        self.binary = self.root / 'built-vizzi'
        self.binary.write_bytes(b'fixture executable')
        self.output = self.root / 'dist'

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

    def test_complete_bundle_is_required_and_tampering_is_rejected(self):
        archives = []
        for target in release.TARGETS:
            archives.append(release.package('v0.1.0', target, self.binary, self.output, self.root))
        sums = release.bundle('v0.1.0', self.output, self.root)
        self.assertEqual(len(sums.read_text().splitlines()), 4)
        archives[0].write_bytes(b'tampered')
        with self.assertRaises(ValueError):
            release.bundle('v0.1.0', self.output, self.root)

    def test_missing_targets_and_unknown_targets_are_rejected(self):
        self.output.mkdir()
        with self.assertRaises(ValueError):
            release.bundle('v0.1.0', self.output, self.root)
        with self.assertRaises(ValueError):
            release.package('v0.1.0', '../../bad', self.binary, self.output, self.root)


if __name__ == '__main__':
    unittest.main()
