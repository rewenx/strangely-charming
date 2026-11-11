#!/usr/bin/env python3
"""Regression checks for licensing documents in the source-only mod archive."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

SPEC = importlib.util.spec_from_file_location('sc_package', Path(__file__).with_name('package.py'))
packager = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(packager)


class PackageNoticesTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.bundle = self.root / 'mixins/strangely_charming'
        self.bundle.mkdir(parents=True)
        (self.root / 'compatibility.json').write_text(json.dumps({
            'horizon_revision': 'abc123', 'horizon_mixins_revision': 'def456',
        }))
        (self.bundle / 'horizon-mixin.toml').write_text(
            '[mod]\nid = "strangely_charming"\nversion = "0.0.1"\n'
            '[compatibility]\nhorizon_revision = "abc123"\n')
        self.notices = {}
        for source, archive in packager.ROOT_NOTICES.items():
            content = f'Root notice: {source}\n'.encode()
            (self.root / source).write_bytes(content)
            self.notices[f'mixins/strangely_charming/{archive}'] = content
        for name in packager.BUNDLE_NOTICES:
            self.notices[f'mixins/strangely_charming/{name}'] = f'Upstream notice: {name}\n'.encode()
        for name, content in self.notices.items():
            if Path(name).name not in packager.ROOT_NOTICES.values():
                (self.root / name).write_bytes(content)
        (self.bundle / 'example.rs').write_text('// Native source\n')
        (self.bundle / 'private.log').write_text('Do not package\n')
        self.archive = self.root / 'target/strangely_charming-0.0.1.hnmod'

    def build(self):
        with patch.object(packager, 'ROOT', self.root), patch.object(packager.subprocess, 'run') as run:
            with contextlib.redirect_stdout(io.StringIO()):
                packager.package()
            run.assert_called_once()
        return self.archive.read_bytes()

    def test_all_notices_are_packaged_byte_for_byte(self):
        self.build()
        with zipfile.ZipFile(self.archive) as archive:
            for name, expected in self.notices.items():
                self.assertEqual(archive.read(name), expected, name)
            self.assertIn('mixins/strangely_charming/example.rs', archive.namelist())
            self.assertNotIn('mixins/strangely_charming/private.log', archive.namelist())
            for entry in archive.infolist():
                self.assertEqual(entry.date_time, (2026, 1, 1, 0, 0, 0))
                self.assertEqual(entry.external_attr >> 16, 0o100644)

    def test_packaged_links_resolve_to_bundled_notices(self):
        (self.root / 'LICENSING.md').write_text(
            '[MIT](LICENSE) [Model](mixins/strangely_charming/LICENSE.protogen-model) '
            '[Art](tools/release_art.json)\n')
        self.build()
        with zipfile.ZipFile(self.archive) as archive:
            text = archive.read('mixins/strangely_charming/LICENSES.md').decode()
            self.assertIn('[MIT](LICENSE)', text)
            self.assertIn('[Model](LICENSE.protogen-model)', text)
            self.assertIn('(https://github.com/rewenx/strangely-charming/blob/main/tools/release_art.json)', text)
            self.assertNotIn('mixins/strangely_charming/LICENSE.protogen-model)', text)

    def test_package_is_reproducible(self):
        self.assertEqual(self.build(), self.build())

    def test_missing_root_notice_fails_before_embed(self):
        for name in packager.ROOT_NOTICES:
            with self.subTest(name=name):
                path = self.root / name
                content = path.read_bytes()
                path.unlink()
                with patch.object(packager, 'ROOT', self.root), patch.object(packager.subprocess, 'run') as run:
                    with self.assertRaisesRegex(SystemExit, 'Missing packaged notice'):
                        packager.package()
                    run.assert_not_called()
                path.write_bytes(content)

    def test_missing_required_component_notices_fail(self):
        for name in packager.BUNDLE_NOTICES:
            with self.subTest(name=name):
                path = self.bundle / name
                content = path.read_bytes()
                path.unlink()
                with self.assertRaisesRegex(SystemExit, 'Missing packaged notice'):
                    self.build()
                path.write_bytes(content)


if __name__ == '__main__':
    unittest.main()
