"""Generate the portal variants from one orientation convention for models and mosaic tiles."""
import json
from pathlib import Path

BIOMES = json.loads((Path(__file__).resolve().parents[1] / "art/portal_biomes.json").read_text())

def portal_files():
    ns = "strangely_charming"
    files = {}
    def put(path, obj):
        files[path] = (json.dumps(obj, separators=(",", ":")) + "\n").encode()
    for theme in ["nether", *(f"{t}_{i}" for t in (*BIOMES, "unknown") for i in range(9))]:
        tex = f"{ns}:block/portal_{theme}"
        put(f"assets/{ns}/textures/block/portal_{theme}.png.mcmeta", {"animation":{"frametime": 3}})
        for axis, front, back in [("x", "south", "north"), ("z", "west", "east")]:
            name = f"rune_portal_{theme}_{axis}"
            # Vanilla NetherPortalBlock removes itself when an adjacent tile has a different
            # block ID. A pass-through glass model lets the saved amethyst frame own integrity.
            shape = [0,0,.375,1,1,.625] if axis == "x" else [.375,0,0,.625,1,1]
            put(f"data/{ns}/horizon/block/{name}.json", {
                "template":"minecraft:glass","item":False, "shapes":{"default":[shape]},
                "collision_shapes":{"default":[]},
                "properties":{"light":11,"occludes":False,"hardness":-1,"explosion_resistance":3600000}})
            put(f"assets/{ns}/blockstates/{name}.json", {"variants":{"":{"model":f"{ns}:block/{name}"}}})
            lo, hi = ([0,0,6],[16,16,10]) if axis == "x" else ([6,0,0],[10,16,16])
            put(f"assets/{ns}/models/block/{name}.json", {"textures":{"portal":tex,"particle":tex},"elements":[{
                "from":lo,"to":hi,"faces":{front:{"uv":[0,0,16,16],"texture":"#portal"},
                back:{"uv":[16,0,0,16],"texture":"#portal"}}}]})
    return files.items()
