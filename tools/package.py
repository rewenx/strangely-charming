#!/usr/bin/env python3
"""Package the mod as a source-only `.hnmod`, the file Horizon's registry and launcher take.

The archive holds `horizon-native.json`, licensing documents and `mixins/<id>/` source and notices, with fixed dates and
modes, so the same sources always give the same bytes and the same SHA-256. Horizon's
`horizon_mods::index::native` reads it without running anything; the launcher compiles it on the pinned
Horizon revision. Based on levzzz5154's packager for Veinminer.
"""
import subprocess
import hashlib
import json
from pathlib import Path
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
GAME = '26.3'
ROOT_NOTICES = {'LICENSE': 'LICENSE', 'LICENSING.md': 'LICENSES.md'}
BUNDLE_NOTICES = ('NOTICE', 'LICENSE.template', 'LICENSE.charm', 'LICENSE.charm-and-strange',
                  'LICENSE.horizon', 'LICENSE.protomod', 'LICENSE.protogen-model')


def package():
    # Check required notices before regeneration or writing any package bytes.
    for name in ROOT_NOTICES:
        if not (ROOT / name).is_file():
            raise SystemExit(f'Missing packaged notice: {name}')
    subprocess.run(["python3", str(ROOT / "tools/embed_content.py")], check=True)
    compat = json.loads((ROOT / 'compatibility.json').read_text())
    manifests = sorted((ROOT / 'mixins').glob('*/horizon-mixin.toml'))
    if len(manifests) != 1:
        raise SystemExit('a .hnmod holds one bundle: keep one mixins/<id>/horizon-mixin.toml')
    bundle = manifests[0].parent
    mixin = tomllib.loads(manifests[0].read_text())
    mod = mixin['mod']
    if bundle.name != mod['id']:
        raise SystemExit(f"rename mixins/{bundle.name}/ to mixins/{mod['id']}/ (the registry finds it by id)")
    if not mod['id'].replace('_', '').isalnum() or not mod['id'].islower():
        raise SystemExit('the registry takes ids of lowercase letters, digits and _ only')
    if mixin.get('compatibility', {}).get('horizon_revision') != compat['horizon_revision']:
        raise SystemExit('horizon-mixin.toml and compatibility.json pin different Horizon revisions')
    manifest = dict(schema=1, id=mod['id'], version=mod['version'], game=GAME,
                    horizon=compat['horizon_revision'], mixins=compat['horizon_mixins_revision'])
    files = {'horizon-native.json': (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()}
    for path in sorted(bundle.iterdir()):
        if path.is_file() and (path.suffix in ('.rs', '.toml') or path.name.startswith(('NOTICE', 'LICENSE', 'README'))):
            files[path.relative_to(ROOT).as_posix()] = path.read_bytes()
    for required in BUNDLE_NOTICES:
        if f'mixins/{mod["id"]}/{required}' not in files:
            raise SystemExit(f'Missing packaged notice: {required}')
    # Horizon permits notices only below mixins/<id>/, with recognized names.
    # Keep LICENSING.md canonical in the repository and adapt its relative links
    # in the packaged LICENSES.md copy so the bundled notices remain navigable.
    for source_name, archive_name in ROOT_NOTICES.items():
        data = (ROOT / source_name).read_bytes()
        if source_name == 'LICENSING.md':
            data = (data.decode('utf-8')
                    .replace(f'(mixins/{mod["id"]}/', '(')
                    .replace('(tools/release_art.json)',
                             '(https://github.com/rewenx/strangely-charming/blob/main/tools/release_art.json)')
                    .encode('utf-8'))
        target = f'mixins/{mod["id"]}/{archive_name}'
        if target in files:
            raise SystemExit(f'Duplicate packaged notice: {target}')
        files[target] = data
    out = ROOT / 'target' / f"{mod['id']}-{mod['version']}.hnmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    print(json.dumps(dict(file=str(out), size=out.stat().st_size,
                          sha256=hashlib.sha256(out.read_bytes()).hexdigest()), indent=2))


if __name__ == '__main__':
    package()
