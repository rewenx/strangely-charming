// The preview never changes the world. The server resolves the journal page and
// owns the construction clock, material debits, and final inscription.
use super::{
    Game, Screen,
    sc_runes::{self, Address, FRAME_GLYPHS, Pos, Site},
    sc_travel::{self, Entry},
    shield::OFFHAND_SLOT,
};
use horizon_world::nbt::Tag;
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};
const BLOCK: &str = "minecraft:amethyst_block";
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout {
    base: Pos,
    axis: usize,
    sign: i32,
}
impl Layout {
    fn at(self, h: i32, y: i32) -> Pos {
        let mut p = self.base;
        p[self.axis] += h * self.sign;
        p[1] += y;
        p
    }
    fn ring(self) -> [Pos; 12] {
        FRAME_GLYPHS.map(|(h, y)| self.at(h, y))
    }
    fn field(self) -> impl Iterator<Item = Pos> {
        (-1..=1).flat_map(move |h| (1..=3).map(move |y| self.at(h, y)))
    }
    fn packet(self) -> Tag {
        Tag::IntArray(vec![
            self.base[0],
            self.base[1],
            self.base[2],
            self.axis as i32,
            self.sign,
        ])
    }
}
struct Preview {
    request: String,
    dimension: String,
}
struct Job {
    layout: Layout,
    dim: String,
    glyphs: Address,
    count: usize,
    next: i64,
    seen: Instant,
}
thread_local! {
    static PREVIEW:RefCell<Option<Preview>>=const {RefCell::new(None)};
    static JOBS:RefCell<HashMap<(PathBuf,u128),Job>>=RefCell::default();
}
impl Game<'_> {
    pub(super) fn sc_amethyst_count(&self) -> u32 {
        (0..36)
            .chain([OFFHAND_SLOT])
            .filter_map(|i| self.stack_in(i))
            .filter(|s| s.item.as_ref() == BLOCK)
            .map(|s| s.count)
            .fold(0, u32::saturating_add)
    }
    fn sc_frame_clear(&self, p: Pos) -> bool {
        let [x, y, z] = p;
        let state = self.world.block(x, y, z);
        let facts = self.res.facts();
        let n = facts.get(state).name.as_ref();
        facts.is_air(state)
            || matches!(
                n,
                "minecraft:short_grass"
                    | "minecraft:tall_grass"
                    | "minecraft:fern"
                    | "minecraft:large_fern"
                    | "minecraft:leaf_litter"
                    | "minecraft:dead_bush"
                    | "minecraft:bush"
                    | "minecraft:firefly_bush"
                    | "minecraft:snow"
                    | "minecraft:pink_petals"
                    | "minecraft:wildflowers"
            )
            || self.items().block_in_tag("minecraft:flowers", n)
    }
    fn sc_frame_aim(&self) -> Option<Layout> {
        let dir = super::view_vector(self.yaw, self.pitch);
        let hit = self.clip_ray(self.eye_position()?, dir, 6.)?;
        let base = if self.sc_frame_clear(hit.pos) {
            hit.pos
        } else {
            hit.beside()
        };
        let axis = if dir[2].abs() >= dir[0].abs() { 0 } else { 2 };
        let sign = if (axis == 0 && dir[2] <= 0.) || (axis == 2 && dir[0] >= 0.) {
            1
        } else {
            -1
        };
        Some(Layout { base, axis, sign })
    }
    fn sc_frame_room(&self, l: Layout, n: usize) -> bool {
        let [bx, bz, ex, ez] = self.border.bounds();
        let in_bounds = |[x, y, z]: Pos| {
            self.world.loaded(x, z)
                && y >= self.world.shape.min_y
                && y < self.world.shape.max_y()
                && f64::from(x) >= bx
                && f64::from(z) >= bz
                && f64::from(x) + 1. <= ex
                && f64::from(z) + 1. <= ez
        };
        self.may_build()
            && !self.spectator()
            && l.field().all(|p| in_bounds(p) && self.sc_frame_clear(p))
            && l.ring().into_iter().enumerate().all(|(i, p)| {
                in_bounds(p)
                    && if i < n {
                        self.block_name(p).as_ref() == BLOCK
                    } else {
                        self.sc_frame_clear(p)
                    }
            })
    }
    pub(super) fn sc_begin_preview(&mut self, request: String) {
        self.native_mod_screen_close("strangely_charming");
        // A diagram opens the preview in the playing screen. Release the Use
        // press that opened it before the preview takes ownership of the mouse.
        self.toggles.0[3].reset();
        self.right = false;
        self.use_clicks = 0;
        self.release_use_for_keybind_screen();
        PREVIEW.with_borrow_mut(|v| {
            *v = Some(Preview {
                request,
                dimension: self.dim.id().into(),
            })
        });
    }
    pub(super) fn sc_cancel_preview(&self) -> bool {
        PREVIEW.with_borrow_mut(|v| v.take().is_some())
    }
    pub(super) fn sc_preview_frame(&self) {
        if self.screen != Screen::Playing
            || !self.focused
            || self.player.is_none()
            || PREVIEW.with_borrow(|v| v.as_ref().is_some_and(|p| p.dimension != self.dim.id()))
        {
            self.sc_cancel_preview();
        }
    }
    pub(super) fn sc_draw_placement_prompt(&self, batch: &mut super::HudBatch) {
        if PREVIEW.with_borrow(|v| v.is_some()) {
            self.sc_yellow_text(
                batch,
                "Right-click to place portal. Press Esc to cancel.",
                255,
                20.,
            );
        }
    }
    pub(super) fn sc_draw_preview(
        &self,
        out: &mut horizon_client::world_primitives::WorldPrimitives,
    ) {
        if !PREVIEW.with_borrow(|v| v.is_some()) {
            return;
        }
        let Some(l) = self.sc_frame_aim() else { return };
        let rgb = if self.sc_frame_room(l, 0) && self.sc_amethyst_count() >= 12 {
            0x45ef83
        } else {
            0xff4741
        };
        for p in l.ring() {
            let min = p.map(|x| f64::from(x) - 0.005);
            let max = p.map(|x| f64::from(x) + 1.005);
            out.filled_box(min, max, 0x50000000 | rgb);
            out.box_outline(min, max, 0xd0000000 | rgb, 1.5);
        }
    }
    pub(super) fn sc_preview_button(&mut self, commit: bool, pressed: bool) -> bool {
        if !PREVIEW.with_borrow(|v| v.is_some()) {
            return false;
        }
        if commit && pressed {
            let Some(l) = self
                .sc_frame_aim()
                .filter(|&l| self.sc_frame_room(l, 0) && self.sc_amethyst_count() >= 12)
            else {
                self.sc_notice("The frame needs clear space and 12 amethyst blocks.");
                return true;
            };
            let req = PREVIEW.with_borrow_mut(|v| v.take().unwrap().request);
            let data = l.packet();
            if let Some(remote) = self.remote.as_mut() {
                remote.session.send_custom_click(&req, Some(data));
            } else if let Some(action) = req.strip_prefix(sc_travel::CHANNEL) {
                self.sc_travel_action(action, Some(&data));
            }
        }
        true
    }
    pub(super) fn sc_known_frame_address(&mut self, site: &Site) -> Option<Address> {
        let glyphs = match sc_runes::address(&self.sc_rune_dir(), site) {
            Ok(glyphs) => glyphs,
            Err(err) => {
                log::error!("Could not read the portal address: {err}");
                self.sc_notice("This portal address could not be read.");
                return None;
            }
        };
        if !self
            .sc_research()
            .is_ok_and(|r| glyphs.iter().all(|g| r.learned.contains(g)))
        {
            self.sc_notice("Learn every glyph in this address first.");
            return None;
        }
        Some(glyphs)
    }
    pub(super) fn sc_confirm_builder(&mut self, e: &Entry, data: Option<&Tag>) {
        if !sc_runes::enabled() || self.remote.is_some() {
            return;
        }
        let site = Site {
            dim: e.dim.clone(),
            pos: e.pos,
        };
        if let Some(glyphs) = self.sc_known_frame_address(&site) {
            self.sc_start_frame_build(glyphs, data);
        }
    }
    pub(super) fn sc_start_frame_build(&mut self, glyphs: Address, data: Option<&Tag>) -> bool {
        // Both journal and diagram builds must match the acting player's actual aim.
        let Some(l) = self.sc_frame_aim().filter(|l| Some(&l.packet()) == data) else {
            self.sc_notice("Your aim changed. Preview the frame again.");
            return false;
        };
        if self.sc_amethyst_count() < 12 || !self.sc_frame_room(l, 0) {
            self.sc_notice("The frame needs clear space and 12 amethyst blocks.");
            return false;
        }
        let key = (self.world_root.clone(), self.native_acting_id());
        let busy = JOBS.with_borrow_mut(|jobs| {
            jobs.retain(|_, j| j.seen.elapsed() < Duration::from_secs(90));
            if jobs.contains_key(&key)
                || jobs.iter().any(|((world, _), j)| {
                    world == &key.0
                        && j.dim == self.dim.id()
                        && l.ring().into_iter().chain(l.field()).any(|p| {
                            j.layout.ring().contains(&p) || j.layout.field().any(|q| q == p)
                        })
                })
            {
                return true;
            }
            jobs.insert(
                key,
                Job {
                    layout: l,
                    dim: self.dim.id().into(),
                    glyphs,
                    count: 0,
                    next: self.world.game_time + 20,
                    seen: Instant::now(),
                },
            );
            false
        });
        if busy {
            self.sc_notice("A frame is already being built here or for this player.");
        }
        !busy
    }
    pub(super) fn sc_build_tick(&mut self) {
        let key = (self.world_root.clone(), self.native_acting_id());
        let Some(mut job) = JOBS.with_borrow_mut(|jobs| jobs.remove(&key)) else {
            return;
        };
        if job.dim != self.dim.id()
            || job.seen.elapsed() > Duration::from_secs(90)
            || self.player.as_ref().is_none_or(|p| p.health.health() <= 0.)
        {
            return;
        }
        job.seen = Instant::now();
        if self.world.game_time >= job.next {
            // Pay only for blocks actually placed. An interrupted build leaves
            // normal, mineable amethyst, with all unspent materials in inventory.
            if !self.sc_frame_room(job.layout, job.count) {
                self.sc_notice("Frame construction stopped: the space changed.");
                return;
            }
            let Some(slot) = self.sc_journal_material(BLOCK) else {
                self.sc_notice("Frame construction stopped: no amethyst left.");
                return;
            };
            let ring = job.layout.ring();
            let p = ring[job.count];
            if !self.sc_grave_set(p, BLOCK) {
                return;
            }
            let stack = self.stack_in_mut(slot);
            if let Some(s) = stack.as_mut() {
                s.count -= 1;
                if s.count == 0 {
                    *stack = None;
                }
            }
            let center = p.map(|x| x as f64 + 0.5);
            if let Some(level) = self.native_level(self.dim.id()) {
                self.native_level_play_sound(
                    level,
                    "minecraft:block.amethyst_block.place",
                    center,
                    4,
                    1.,
                    1.,
                );
            }
            job.count += 1;
            job.next = self.world.game_time + 20;
            if job.count == 12 {
                if !self.sc_frame_room(job.layout, 12) {
                    self.sc_notice("The frame changed before inscription.");
                    return;
                }
                for p in job.layout.field() {
                    if !self.sc_grave_set(p, "minecraft:air") {
                        return;
                    }
                }
                for (p, g) in ring.into_iter().zip(job.glyphs) {
                    if !self.sc_inscribe_frame_rune(p, g, job.layout.axis, job.layout.sign) {
                        return;
                    }
                }
                self.spawn_bolt(job.layout.base.map(|x| x as f64 + 0.5), true);
                self.sc_notice("Frame complete. Use a Knowledge Stone to light it.");
                return;
            }
        }
        JOBS.with_borrow_mut(|jobs| jobs.insert(key, job));
    }
}
