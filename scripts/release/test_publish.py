"""Exercise publication ordering with a local GitHub CLI peer; no network writes."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from test_package import release, extension_fixtures


class PublishTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        scripts = self.root / 'scripts/release'
        scripts.mkdir(parents=True)
        for name in ['package.py', 'publish.sh']:
            shutil.copy(Path(__file__).with_name(name), scripts / name)
        manifest = self.root / 'crates/vizzi/Cargo.toml'
        manifest.parent.mkdir(parents=True)
        manifest.write_text('[package]\nversion="0.1.0"\n')
        (self.root / 'LICENSE-APACHE').write_text('Fixture project license')
        binary = self.root / 'binary'
        binary.write_bytes(b'fixture executable')
        for target in release.TARGETS:
            release.package('v0.1.0', target, binary, self.root / 'dist', self.root)
        extension_fixtures(self.root, self.root / 'dist')
        release.bundle('v0.1.0', self.root / 'dist', self.root)
        self.tools = self.root / 'bin'
        self.tools.mkdir()
        git = self.tools / 'git'
        git.write_text('#!/bin/sh\nexit 0\n')
        git.chmod(0o700)
        gh = self.tools / 'gh'
        gh.write_text(r'''#!/usr/bin/env python3
import json,os,shutil,sys
from pathlib import Path
args=sys.argv[1:];root=Path(os.environ['PUBLISH_FIXTURE']);remote=root/'remote'
with (root/'calls.jsonl').open('a') as log:log.write(json.dumps(args)+'\n')
if args[0]=='api':
    endpoint=args[1]
    if '/git/ref/tags/' in endpoint:
        print(json.dumps({'object':{'type':'commit','sha':('b'*40 if os.environ.get('MOVED') else os.environ['GITHUB_SHA'])}}))
    elif '/releases/tags/' in endpoint:
        print('true' if not os.environ.get('MUTABLE') else 'false')
    else:print('main')
elif args[:2]==['release','view']:
    sys.exit(0 if os.environ.get('EXISTING') else 1)
elif args[:2]==['release','create']:
    remote.mkdir()
    for path in args[3:]:
        if path.startswith('--'):break
        shutil.copy(path,remote/Path(path).name)
    if os.environ.get('CORRUPT'):
        next(remote.glob('*.tar.gz')).write_bytes(b'corrupted upload')
elif args[:2]==['release','download']:
    destination=Path(args[args.index('--dir')+1])
    for path in remote.iterdir():shutil.copy(path,destination/path.name)
elif args[:2]==['release','edit']:
    (root/'published').write_text('published')
else:raise AssertionError(args)
''')
        gh.chmod(0o700)

    def run_publish(self, **settings):
        env = {**os.environ, 'PATH': str(self.tools) + os.pathsep + os.environ['PATH'],
               'RELEASE_TAG': 'v0.1.0', 'GITHUB_REPOSITORY': 'fixture/repo',
               'GITHUB_SHA': 'a' * 40, 'RUNNER_TEMP': str(self.root),
               'PUBLISH_FIXTURE': str(self.root), **settings}
        return subprocess.run(['bash', 'scripts/release/publish.sh'], cwd=self.root,
                              env=env, capture_output=True, text=True)

    def test_complete_assets_are_checked_before_publication(self):
        result = self.run_publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [json.loads(line) for line in (self.root / 'calls.jsonl').read_text().splitlines()]
        operations = [call[1] for call in calls if call[0] == 'release']
        self.assertEqual(operations, ['view', 'create', 'download', 'edit'])
        self.assertEqual(len(list((self.root / 'remote').iterdir())), 9)
        self.assertTrue((self.root / 'published').exists())

    def test_existing_releases_moved_tags_and_corruption_prevent_publication(self):
        for setting in ['EXISTING', 'MOVED', 'CORRUPT']:
            with self.subTest(setting=setting):
                result = self.run_publish(**{setting: '1'})
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((self.root / 'published').exists())
                if (self.root / 'remote').exists():
                    shutil.rmtree(self.root / 'remote')

    def test_mutable_publication_is_reported_as_a_failure(self):
        result = self.run_publish(MUTABLE='1')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('not immutable', result.stderr)

    def test_prerelease_versions_are_not_published_as_stable(self):
        (self.root / 'crates/vizzi/Cargo.toml').write_text('[package]\nversion="0.1.0-rc.1"\n')
        shutil.rmtree(self.root / 'dist')
        for target in release.TARGETS:
            release.package('v0.1.0-rc.1', target, self.root / 'binary', self.root / 'dist', self.root)
        extension_fixtures(self.root, self.root / 'dist')
        release.bundle('v0.1.0-rc.1', self.root / 'dist', self.root)
        result = self.run_publish(RELEASE_TAG='v0.1.0-rc.1')
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [json.loads(line) for line in (self.root / 'calls.jsonl').read_text().splitlines()]
        create = next(call for call in calls if call[:2] == ['release', 'create'])
        self.assertIn('--prerelease', create)


if __name__ == '__main__':
    unittest.main()
