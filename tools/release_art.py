"""Recover the credited in-game artwork for reproducible public source builds."""
import ast
import hashlib
import json
import os
from pathlib import Path
import zipfile

FILES = Path(__file__).with_name("release_art.json")
ARCHIVE = "strangely_charming-0.2.42.hnmod"
SHA256 = "33ef3b5018bc5ce6ee3efca905553d4417afa87dc99451194069c843723fe10b"


def release_art(existing):
    """Use the source checkout for all code and the archive only for missing assets."""
    hashes = json.loads(FILES.read_text())
    missing = hashes.keys() - existing
    if not missing:
        return []
    archive = Path(os.environ.get("SC_ART_PACKAGE", FILES.parents[1] / ARCHIVE))
    if not archive.is_file():
        raise SystemExit(
            f"Download {ARCHIVE} from the v0.2.42 GitHub release and place it at "
            f"{archive} (or set SC_ART_PACKAGE to its location), then retry."
        )
    with archive.open("rb") as source:
        if hashlib.file_digest(source, "sha256").hexdigest() != SHA256:
            raise SystemExit(f"Expected the published v0.2.42 art package at {archive}")
        source.seek(0)
        supplied = {}
        with zipfile.ZipFile(source) as package:
            with package.open("mixins/strangely_charming/content_bundle.rs") as rust:
                for row in rust:
                    if not row.startswith(b'("'):
                        continue
                    name, sep, literal = row.rstrip(b"\r\n").partition(b'", b"')
                    if not sep:
                        continue
                    name = name[2:].decode("ascii")
                    if name in missing:
                        data = ast.literal_eval('b"' + literal.removesuffix(b"),").decode("ascii"))
                        if hashlib.sha256(data).hexdigest() != hashes[name]:
                            raise SystemExit(f"Release artwork failed verification: {name}")
                        supplied[name] = data
    if supplied.keys() != missing:
        raise SystemExit("Missing release artwork: " + ", ".join(sorted(missing - supplied.keys())))
    return list(supplied.items())
