// Classic 64 × 32 box UVs used by Charm's animal skins.
use super::{EntityModelDocument, MobTypes, Part, Polygon};
use std::path::Path;
use std::sync::Arc;

fn part(name: &str, at: [f32; 3]) -> Part {
    Part {
        name: name.into(),
        pose: [at[0], at[1], at[2], 0., 0., 0., 1., 1., 1.],
        polygons: Vec::new(),
        children: Vec::new(),
    }
}

fn cuboid(p: &mut Part, origin: [f32; 3], size: [f32; 3], uv: [f32; 2]) {
    let [x, y, z] = origin;
    let [w, h, d] = size;
    let [u, v] = uv;
    let (xx, yy, zz) = (x + w, y + h, z + d);
    let corners = [
        [x, y, z],
        [xx, y, z],
        [xx, yy, z],
        [x, yy, z],
        [x, y, zz],
        [xx, y, zz],
        [xx, yy, zz],
        [x, yy, zz],
    ];
    for (face, normal, [u0, v0, u1, v1]) in [
        ([5, 4, 0, 1], [0., -1., 0.], [u + d, v, u + d + w, v + d]),
        (
            [2, 3, 7, 6],
            [0., 1., 0.],
            [u + d + w, v + d, u + d + 2. * w, v],
        ),
        ([0, 4, 7, 3], [-1., 0., 0.], [u, v + d, u + d, v + d + h]),
        (
            [1, 0, 3, 2],
            [0., 0., -1.],
            [u + d, v + d, u + d + w, v + d + h],
        ),
        (
            [5, 1, 2, 6],
            [1., 0., 0.],
            [u + d + w, v + d, u + 2. * d + w, v + d + h],
        ),
        (
            [4, 5, 6, 7],
            [0., 0., 1.],
            [u + 2. * d + w, v + d, u + 2. * d + 2. * w, v + d + h],
        ),
    ] {
        let uv = [[u1, v0], [u0, v0], [u0, v1], [u1, v1]];
        let vertices = std::array::from_fn(|n| {
            let [x, y, z] = corners[face[n]];
            [x, y, z, uv[n][0] / 64., uv[n][1] / 32.]
        });
        p.polygons.push(Polygon {
            vertices,
            normal,
            cull_back: false,
        });
    }
}

fn squid() -> Part {
    let mut root = part("root", [0.; 3]);
    let mut body = part("body", [0., 16., 0.]);
    cuboid(&mut body, [-3., -4., -3.], [6., 8., 6.], [0., 0.]);
    root.children.push(body);
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.;
        let mut t = part(&format!("tentacle{i}"), [a.cos() * 2.5, 20., a.sin() * 2.5]);
        t.pose[4] = std::f32::consts::FRAC_PI_2 - a;
        cuboid(&mut t, [-0.5, 0., -0.5], [1., 7., 1.], [48., 0.]);
        root.children.push(t);
    }
    root
}

fn protogen_document(name: &str) -> serde_json::Value {
    let contents = horizon_mods::strangely_charming_content::FILES.iter()
        .find(|(path, _)| *path == name).expect("packaged protogen asset").1;
    serde_json::from_slice(contents).expect("valid protogen document")
}

pub(super) fn pose_protogen(s: &super::anim::State, poses: &mut super::anim::Poses) {
    use super::keyframes::Animation;
    static CLIPS: std::sync::OnceLock<[Animation; 3]> = std::sync::OnceLock::new();
    let [idle, walk, punch] = CLIPS.get_or_init(|| {
        let doc = protogen_document("assets/strangely_charming/protogen/clips.json");
        ["idle", "walk", "punch_right"].map(|name| Animation::parse(&doc[name]))
    });
    let speed = s.speed.clamp(0., 1.);
    idle.apply((s.age * 50.) as i64, 1. - speed, poses);
    if speed > 0. { walk.apply((s.pos * 175.) as i64, speed, poses); }
    if s.swing > 0. { punch.apply((s.swing * 2000.) as i64, 1., poses); }
    let head = poses.get("head");
    head[3] += s.x_rot.to_radians();
    head[4] += s.y_rot.to_radians();
}

pub(super) fn load(
    root: &Path,
    render: bool,
    document: Option<&EntityModelDocument>,
) -> crate::Result<MobTypes> {
    let mut types = MobTypes::load_with_models(root, render, document)?;
    if let Some(kind) = types.types.get_mut("strangely_charming:coral_squid") {
        kind.width = 0.45;
        kind.height = 0.6;
        kind.eye_height = 0.35;
        if render {
            let model = Arc::new(squid());
            kind.model = model.clone();
            kind.layers.insert("squid#main".into(), model);
        }
    }
    let skeleton = types.types.get("minecraft:skeleton").map(|k| (k.width, k.height, k.eye_height));
    let ocelot = types.types.get("minecraft:ocelot").map(|k| (k.sounds, k.step_sound, k.ambient_interval));
    if let Some(kind) = types.types.get_mut("strangely_charming:protogen") {
        if let Some(size) = skeleton { (kind.width, kind.height, kind.eye_height) = size; }
        if let Some(voice) = ocelot { (kind.sounds, kind.step_sound, kind.ambient_interval) = voice; }
        // Borrow melee AI, not the zombie's sunlight vulnerability or undead potion rules.
        kind.flags.burns_in_day = false;
        kind.flags.undead = false;
        kind.texture = "strangely_charming:entity/protogen".into();
        if render {
            let rig = protogen_document("assets/strangely_charming/protogen/rig.json");
            let model = Arc::new(Part::parse("root", &rig));
            kind.model = model.clone();
            kind.layers.insert("protogen#main".into(), model);
        }
    }
    Ok(types)
}
