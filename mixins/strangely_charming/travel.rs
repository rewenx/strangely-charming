use super::{
    Game,
    maps::MapData,
    sc_items::{custom, set_custom},
    sc_runes::{self, Address, Site},
    shield::OFFHAND_SLOT,
};
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
type Sessions = HashMap<(std::path::PathBuf, u128), (usize, String, Instant)>;
static VALID: OnceLock<Mutex<Sessions>> = OnceLock::new();

pub(super) const BOOK: &str = "strangely_charming:travel_journal";
pub(super) const PAGE: &str = "strangely_charming:travel_journal_page";
pub(super) const CHANNEL: &str = "strangely_charming:journal/";
pub(super) const DIAGRAM: &str = "strangely_charming:portal_diagram";
pub(super) const PREVIEW_DIAGRAM: &str = "strangely_charming:preview_diagram";
const DUST: &str = "strangely_charming:rune_dust";

#[derive(Serialize, Deserialize)]
struct Diagram {
    id: String,
    glyphs: Address,
    site: Site,
}

const ENTRY: &str = "strangely_charming:journal_entry";
pub(super) const COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];
pub(super) const RGB: [&str; 16] = [
    "#F9FFFE", "#F9801D", "#C74EBD", "#3AB3DA", "#FED83D", "#80C71F", "#F38BAA", "#474F52",
    "#9D9D97", "#169C9C", "#8932B8", "#3C44AA", "#835432", "#5E7C16", "#B02E26", "#1D1D21",
];
const CAP: usize = 30;
const PER: usize = 5;
pub(super) fn handles(id: &str) -> bool {
    (matches!(id, BOOK | PAGE) || (id == DIAGRAM && sc_runes::enabled()))
        && horizon_mods::evanders_config::enabled("travel_journal")
}
pub(super) fn s(x: impl AsRef<str>) -> Tag {
    Tag::String(x.as_ref().into())
}
fn text(t: Option<&Tag>) -> Option<String> {
    match t? {
        Tag::String(v) => Some(v.to_string_lossy()),
        _ => None,
    }
}
fn read<T: DeserializeOwned>(stack: &ItemStack, k: &str) -> Option<T> {
    let v = text(custom(stack, k))?;
    (v.len() <= 65_536)
        .then(|| serde_json::from_str(&v).ok())
        .flatten()
}
fn write<T: Serialize>(stack: &mut ItemStack, k: &str, v: &T) {
    set_custom(stack, k, s(serde_json::to_string(v).expect("journal data")));
}
fn newid() -> Option<String> {
    let mut v = [0; 16];
    horizon_protocol::crypto::random_bytes(&mut v).ok()?;
    Some(format!("{:032x}", u128::from_be_bytes(v)))
}
pub(super) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn name_ok(n: &str) -> bool {
    !n.trim().is_empty() && n.encode_utf16().count() <= 24 && !n.chars().any(char::is_control)
}
pub(super) fn notes_ok(n: &str) -> bool {
    n.encode_utf16().count() <= 240 && !n.chars().any(|c| c.is_control() && c != '\n')
}
pub(super) fn stack(id: &str) -> ItemStack {
    ItemStack {
        item: id.into(),
        count: 1,
        damage: 0,
        extra: None,
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Entry {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) pos: [i32; 3],
    pub(super) dim: String,
    pub(super) color: usize,
    #[serde(default)]
    pub(super) notes: String,
}
pub(super) fn dimension_name(dimension: &str) -> String {
    match dimension {
        "minecraft:overworld" => "Overworld".into(),
        "minecraft:the_nether" => "Nether".into(),
        "minecraft:the_end" => "The End".into(),
        other => other
            .split_once(':')
            .map_or(other, |(_, name)| name)
            .replace('_', " "),
    }
}
impl Entry {
    pub(super) fn valid(&self) -> bool {
        valid_id(&self.id)
            && name_ok(&self.name)
            && notes_ok(&self.notes)
            && self.color < 16
            && self.dim.len() <= 128
            && self.dim.contains(':')
            && self
                .dim
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./:-".contains(&c))
            && self.pos.iter().all(|p| p.unsigned_abs() <= 30_000_000)
    }
    pub(super) fn dimension_name(&self) -> String {
        dimension_name(&self.dim)
    }
    fn label(&self) -> Tag {
        Tag::compound([("text", s(&self.name)), ("color", s(RGB[self.color]))])
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Book {
    pub(super) id: String,
    pub(super) rev: u64,
    page: usize,
    pub(super) entries: Vec<Entry>,
}
impl Book {
    pub(super) fn from(stack: &ItemStack) -> Option<Self> {
        if stack.item.as_ref() != BOOK {
            return None;
        }
        if custom(stack, BOOK).is_none() {
            return Some(Self {
                id: newid()?,
                rev: 0,
                page: 0,
                entries: vec![],
            });
        }
        let b: Book = read(stack, BOOK)?;
        (valid_id(&b.id) && b.entries.len() <= CAP && b.entries.iter().all(Entry::valid))
            .then_some(b)
    }
    pub(super) fn addr(&self, op: &str) -> String {
        format!("{CHANNEL}{}/{}/{op}", self.id, self.rev)
    }
    fn cannot_import(&self, e: &Entry) -> Option<&'static str> {
        if self
            .entries
            .iter()
            .any(|v| v.pos == e.pos && v.dim == e.dim)
        {
            Some("you cannot have two locations at the same coordinates")
        } else if self.entries.iter().any(|v| v.id == e.id) {
            Some("This location is already in the journal.")
        } else if self.entries.len() >= CAP {
            Some("This journal already contains 30 locations.")
        } else {
            None
        }
    }
    fn bump(&mut self) {
        self.rev = self.rev.saturating_add(1);
    }
}
impl Game<'_> {
    pub(super) fn sc_near_entry(&self, e: &Entry) -> bool {
        self.player.as_ref().is_some_and(|p| {
            self.dim.id() == e.dim
                && (p.position.x - e.pos[0] as f64).powi(2)
                    + (p.position.y - e.pos[1] as f64).powi(2)
                    + (p.position.z - e.pos[2] as f64).powi(2)
                    <= 20. * 20.
        })
    }
    pub(super) fn sc_held_entry(&self, id: &str) -> Option<Entry> {
        [self.selected, OFFHAND_SLOT]
            .into_iter()
            .filter_map(|slot| self.stack_in(slot))
            .find_map(|s| {
                if s.item.as_ref() == PAGE {
                    read::<Entry>(s, ENTRY).filter(|e| e.valid() && e.id == id)
                } else {
                    Book::from(s)?.entries.into_iter().find(|e| e.id == id)
                }
            })
    }
    // Blank books carry no identity or components. Only the current menu has a
    // nonce; the first successful write promotes it into a persistent journal ID.
    fn sc_session_book(&mut self, slot: usize, opening: bool) -> Option<Book> {
        let st = self.stack_in(slot)?;
        let blank = custom(st, BOOK).is_none();
        let mut b = Book::from(st)?;
        if !blank {
            return Some(b);
        }
        let key = (self.world_root.clone(), self.native_acting_id());
        let mut valid = VALID.get_or_init(Default::default).lock().ok()?;
        if opening {
            valid.retain(|_, (_, _, since)| since.elapsed() < Duration::from_secs(86400));
            valid.insert(key, (slot, b.id.clone(), Instant::now()));
        } else {
            let (s, id, _) = valid.get(&key)?;
            if *s != slot {
                return None;
            }
            b.id = id.clone();
        }
        Some(b)
    }
    fn sc_put_book(&mut self, slot: usize, b: &Book) {
        if self
            .stack_in(slot)
            .is_some_and(|s| custom(s, BOOK).is_none())
        {
            let key = (self.world_root.clone(), self.native_acting_id());
            if let Some(m) = VALID.get()
                && let Ok(mut m) = m.lock()
            {
                m.remove(&key);
            }
        }
        write(
            self.stack_in_mut(slot).as_mut().expect("held journal"),
            BOOK,
            b,
        )
    }
    pub(super) fn sc_journal_material(&self, item: &str) -> Option<usize> {
        [self.selected, OFFHAND_SLOT]
            .into_iter()
            .chain(0..36)
            .find(|&slot| {
                self.stack_in(slot)
                    .is_some_and(|s| s.count > 0 && s.item.as_ref() == item)
            })
    }
    fn sc_mark_compass(&mut self, slot: usize, entry: &Entry) {
        // A fixed journal waypoint need not contain a physical lodestone.
        let tracker = Tag::compound([
            ("tracked", Tag::Byte(0)),
            (
                "target",
                Tag::compound([
                    ("dimension", s(&entry.dim)),
                    ("pos", Tag::IntArray(entry.pos.to_vec())),
                ]),
            ),
        ]);
        let Some(input) = self.stack_in_mut(slot).as_mut() else {
            return;
        };
        let mut marked = input.clone();
        marked.count = 1;
        marked
            .data_mut()
            .set_other("minecraft:lodestone_tracker", Some(tracker));
        marked.data_mut().custom_name = Some(entry.label());
        if input.count == 1 {
            *input = marked;
        } else {
            input.count -= 1;
            self.give_or_drop(marked);
        }
    }
    fn sc_spend(&mut self, slot: usize) {
        let s = self.stack_in_mut(slot);
        if let Some(st) = s.as_mut() {
            if st.count <= 1 {
                *s = None
            } else {
                st.count -= 1
            }
        }
    }
    pub(super) fn sc_draw_held_page(&self, b: &mut super::HudBatch) {
        if self.screen != super::Screen::Playing
            || !self.camera_type.first_person()
            || self.dialog_open()
            || self.player.is_none()
        {
            return;
        }
        let hand = [self.selected, OFFHAND_SLOT]
            .into_iter()
            .find(|&i| self.stack_in(i).is_some_and(|s| s.item.as_ref() == PAGE));
        let Some(slot) = hand else { return };
        if !handles(PAGE) {
            return;
        }
        let Some(e) = self
            .stack_in(slot)
            .and_then(|s| read::<Entry>(s, ENTRY))
            .filter(Entry::valid)
        else {
            return;
        };
        let w = 106.;
        let x = if slot == OFFHAND_SLOT {
            5.
        } else {
            (b.width - w - 5.).max(0.)
        };
        let y = (b.height - 132.).max(0.);
        let a = &self.gui.atlas;
        b.fill(a, x, y, w, 94., super::rgba(95, 68, 39, 255));
        b.fill(
            a,
            x + 1.,
            y + 1.,
            w - 2.,
            92.,
            super::rgba(250, 245, 226, 255),
        );
        let c = u32::from_str_radix(&RGB[e.color][1..], 16).unwrap();
        let ink = super::rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, 255);
        let mut label = e.name.clone();
        while !label.is_empty() && a.font.width(&label) as f32 > w - 8. {
            label.pop();
        }
        b.text(
            a,
            &label,
            x + 4.,
            y + 4.,
            ink,
            matches!(e.color, 0 | 3 | 4 | 5 | 6 | 8),
        );
        if !self.sc_page_thumbnail(b, &e.id, x + 4., y + 17.) {
            b.text(
                a,
                "No local photo",
                x + 7.,
                y + 34.,
                super::rgba(100, 79, 58, 255),
                false,
            );
        }
        let coords = format!("{}  {}  {}", e.pos[0], e.pos[1], e.pos[2]);
        let scale = ((w - 10.) / a.font.width(&coords).max(1.)).min(1.);
        let start = b.vertices.len();
        b.text(
            a,
            &coords,
            x + 5.,
            y + 77.,
            super::rgba(51, 32, 21, 255),
            false,
        );
        b.scale_since(start, x + 5., y + 77., scale, scale);
    }
    fn sc_scribe_diagram(&mut self, entry: &Entry) -> bool {
        if self.remote.is_some() || !handles(DIAGRAM) {
            return false;
        }
        let site = Site {
            pos: entry.pos,
            dim: entry.dim.clone(),
        };
        let Some(glyphs) = self.sc_known_frame_address(&site) else {
            return false;
        };
        let (Some(paper), Some(dust)) = (
            self.sc_journal_material("minecraft:paper"),
            self.sc_journal_material(DUST),
        ) else {
            self.sc_notice("You need one sheet of paper and one Rune Dust.");
            return false;
        };
        let Some(id) = newid() else {
            self.sc_notice("The diagram could not be written.");
            return false;
        };
        let mut item = stack(DIAGRAM);
        write(&mut item, DIAGRAM, &Diagram { id, glyphs, site });
        item.data_mut().custom_name = Some(Tag::compound([
            ("text", s(format!("Portal Diagram: {}", entry.name))),
            ("color", s(RGB[entry.color])),
        ]));
        self.sc_spend(paper);
        self.sc_spend(dust);
        self.give_or_drop(item);
        self.sc_notice(format!("Scribed a portal diagram for {}.", entry.name));
        true
    }

    fn sc_read_diagram(&self, item: &ItemStack) -> Option<Diagram> {
        if item.item.as_ref() != DIAGRAM || item.count == 0 {
            return None;
        }
        read::<Diagram>(item, DIAGRAM).filter(|d| {
            valid_id(&d.id) && sc_runes::registered(&self.sc_rune_dir(), d.glyphs, &d.site)
        })
    }

    pub(super) fn sc_preview_diagram(&mut self, id: &str) {
        if handles(DIAGRAM) && valid_id(id) {
            self.sc_begin_preview(format!("{CHANNEL}{id}/0/build_diagram"));
        }
    }

    fn sc_use_diagram(&mut self, item: &ItemStack) {
        let Some(diagram) = self.sc_read_diagram(item) else {
            self.sc_notice("This diagram has no registered destination here.");
            return;
        };
        if self.sc_amethyst_count() < 12 {
            self.sc_notice("You need 12 amethyst blocks to build this portal.");
            return;
        }
        if self.lan_acting_uuid.is_some() {
            use horizon_protocol::packets::{configuration::CustomPayload, play::Clientbound};
            let who = self.native_acting_id();
            self.native_send(
                who,
                Clientbound::CustomPayload(CustomPayload {
                    channel: PREVIEW_DIAGRAM.into(),
                    data: diagram.id.into_bytes(),
                }),
            );
        } else {
            self.sc_preview_diagram(&diagram.id);
        }
    }
    pub(super) fn sc_travel_use(&mut self, st: &ItemStack) -> bool {
        if !handles(&st.item) {
            return false;
        }
        if st.item.as_ref() == DIAGRAM {
            if self.remote.is_none() {
                self.sc_use_diagram(st);
            }
            return true;
        }
        let slot = self.held_slot();
        if st.item.as_ref() == BOOK {
            if self.native_sneaking() {
                return false;
            }
            if let Some(mut b) = self.sc_session_book(slot, true) {
                b.page = b.page.min(b.entries.len().saturating_sub(1) / PER);
                self.sc_send_journal(&b, None, false);
            } else {
                self.sc_notice("This journal could not be read.");
            }
            return true;
        }
        let Some(e) = read::<Entry>(st, ENTRY).filter(Entry::valid) else {
            self.sc_notice("This page has no recorded location.");
            return true;
        };
        let Some(target) = self.sc_journal_material(BOOK) else {
            self.sc_notice("you need a strangely charming journal to record this location");
            return true;
        };
        let Some(book) = self.sc_session_book(target, true) else {
            self.sc_notice("This journal could not be read.");
            return true;
        };
        if let Some(reason) = book.cannot_import(&e) {
            self.sc_notice(reason);
            return true;
        }
        let prompt = Book {
            entries: vec![e],
            ..book
        };
        self.sc_send_journal(&prompt, Some(0), true);
        true
    }
    fn sc_confirm_page(&mut self, id: &str, rev: u64, entry_id: &str) {
        let Some((paper, entry)) = [self.selected, OFFHAND_SLOT].into_iter().find_map(|slot| {
            let st = self.stack_in(slot)?;
            if st.item.as_ref() != PAGE || st.count == 0 {
                return None;
            }
            let e = read::<Entry>(st, ENTRY)?;
            (e.valid() && e.id == entry_id).then_some((slot, e))
        }) else {
            self.sc_notice("Hold the location paper to add it.");
            return;
        };
        let Some((slot, mut book)) = (0..36).chain([OFFHAND_SLOT]).find_map(|slot| {
            let b = self.sc_session_book(slot, false)?;
            (b.id == id).then_some((slot, b))
        }) else {
            self.sc_notice("you need a strangely charming journal to record this location");
            return;
        };
        if let Some(reason) = book.cannot_import(&entry) {
            self.sc_notice(reason);
            return;
        }
        if book.rev != rev {
            self.sc_notice("Your journal changed. Use the paper again.");
            return;
        }
        book.entries.push(entry);
        book.bump();
        self.sc_put_book(slot, &book);
        self.sc_spend(paper);
        self.sc_notice("Location added to your journal.");
    }
    pub(super) fn sc_travel_action(&mut self, action: &str, payload: Option<&Tag>) {
        if !handles(BOOK) {
            return;
        }
        let mut a = action.split('/');
        let (Some(id), Some(rev), Some(op)) = (
            a.next(),
            a.next().and_then(|x| x.parse::<u64>().ok()),
            a.next(),
        ) else {
            return;
        };
        let argument = a.next();
        let i = argument.and_then(|s| s.parse::<usize>().ok());
        if a.next().is_some() || !valid_id(id) {
            return;
        }
        if op == "build_diagram" {
            if rev != 0 || argument.is_some() || self.remote.is_some() || !handles(DIAGRAM) {
                return;
            }
            // The request identifies the sheet. Only the server's held stack supplies the address.
            let held = [self.selected, OFFHAND_SLOT].into_iter().find_map(|slot| {
                let diagram = self.sc_read_diagram(self.stack_in(slot)?)?;
                (diagram.id == id).then_some((slot, diagram))
            });
            if let Some((slot, diagram)) = held {
                if self.sc_start_frame_build(diagram.glyphs, payload) {
                    self.sc_spend(slot);
                }
            } else {
                self.sc_notice("Hold the same Portal Diagram and try again.");
            }
            return;
        }
        if op == "import" {
            if let Some(eid) = argument.filter(|v| valid_id(v)) {
                self.sc_confirm_page(id, rev, eid);
            }
            return;
        }
        let Some((slot, mut b)) = [self.selected, OFFHAND_SLOT].into_iter().find_map(|slot| {
            let b = self.sc_session_book(slot, false)?;
            (b.id == id && b.rev == rev).then_some((slot, b))
        }) else {
            return;
        };
        if op == "build" {
            if let Some(e) = i.and_then(|i| b.entries.get(i)) {
                self.sc_confirm_builder(e, payload);
            }
            return;
        }
        if op == "rune_name" {
            if let (Some(id), Some(n)) = (i, text(payload.and_then(|v| v.get("name")))) {
                self.sc_rename_rune(id, &n);
            }
            return;
        }
        let mut focus = None;
        match op {
            "list" => {}
            "prev" => b.page = b.page.saturating_sub(1),
            "next" => b.page = (b.page + 1).min(b.entries.len().saturating_sub(1) / PER),
            "add" => {
                if b.entries.len() >= CAP {
                    return;
                }
                let Some(p) = self.player.as_ref().map(|p| p.position) else {
                    return;
                };
                let pos = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
                if b.entries
                    .iter()
                    .any(|e| e.dim == self.dim.id() && e.pos == pos)
                {
                    self.sc_notice("you cannot have two locations at the same coordinates");
                    return;
                }
                let Some(eid) = newid() else { return };
                let biome = self
                    .biome_at([p.x, p.y, p.z])
                    .split(':')
                    .next_back()
                    .unwrap_or("Location")
                    .replace('_', " ");
                let mut name: String = biome.chars().take(24).collect();
                if let Some(ch) = name.get_mut(..1) {
                    ch.make_ascii_uppercase()
                }
                if !name_ok(&name) {
                    name = "New location".into()
                }
                let mut color = 15;
                if let Some(input) = payload {
                    let Some(n) = text(input.get("name")).filter(|n| name_ok(n)) else {
                        return;
                    };
                    let Some(c) =
                        text(input.get("color")).and_then(|v| COLORS.iter().position(|&x| x == v))
                    else {
                        return;
                    };
                    name = n.trim().into();
                    color = c;
                }
                focus = Some(b.entries.len());
                b.page = b.entries.len() / PER;
                b.entries.push(Entry {
                    id: eid,
                    name,
                    pos,
                    dim: self.dim.id().into(),
                    color,
                    notes: String::new(),
                });
            }
            _ => {
                let Some(i) = i.filter(|&i| i < b.entries.len()) else {
                    return;
                };
                let e = &mut b.entries[i];
                focus = Some(i);
                match op {
                    "edit" => {}
                    "save" => {
                        let Some(n) =
                            text(payload.and_then(|p| p.get("name"))).filter(|v| name_ok(v))
                        else {
                            return;
                        };
                        let Some(c) = text(payload.and_then(|p| p.get("color")))
                            .and_then(|v| COLORS.iter().position(|&c| c == v))
                        else {
                            return;
                        };
                        if let Some(tag) = payload.and_then(|p| p.get("notes")) {
                            let Some(notes) = text(Some(tag)).filter(|v| notes_ok(v)) else {
                                return;
                            };
                            e.notes = notes;
                        }
                        e.name = n.trim().into();
                        e.color = c;
                    }
                    "delete" => {
                        b.entries.remove(i);
                        b.page = b.page.min(b.entries.len().saturating_sub(1) / PER);
                        focus = None;
                    }
                    "make_diagram" => {
                        if !self.sc_scribe_diagram(e) {
                            self.sc_send_journal(&b, focus, false);
                            return;
                        }
                    }
                    "compass" => {
                        let Some(input) = self.sc_journal_material("minecraft:compass") else {
                            self.sc_notice("You need a compass.");
                            self.sc_send_journal(&b, Some(i), false);
                            return;
                        };
                        self.sc_mark_compass(input, e);
                        self.sc_notice(format!("Compass set to {}.", e.name));
                    }
                    "share" | "map" => {
                        let need = if op == "share" {
                            "minecraft:paper"
                        } else {
                            "minecraft:map"
                        };
                        let Some(input) = self.sc_journal_material(need) else {
                            self.sc_notice(if op == "share" {
                                "You need one sheet of paper."
                            } else {
                                "You need one empty map."
                            });
                            self.sc_send_journal(&b, Some(i), false);
                            return;
                        };
                        let mut out = if op == "share" {
                            let mut page = stack(PAGE);
                            write(&mut page, ENTRY, e);
                            page
                        } else {
                            self.sc_marked_map(e)
                        };
                        out.data_mut().custom_name = Some(if op == "share" {
                            Tag::compound([
                                ("text", s(format!("Location Paper: {}", e.name))),
                                ("color", s(RGB[e.color])),
                            ])
                        } else {
                            e.label()
                        });
                        self.sc_spend(input);
                        self.give_or_drop(out);
                    }
                    _ => return,
                }
            }
        }
        if !b.entries.is_empty()
            || self
                .stack_in(slot)
                .is_some_and(|s| custom(s, BOOK).is_some())
        {
            b.bump();
            self.sc_put_book(slot, &b);
        }
        self.sc_send_journal(&b, focus, false);
    }
    fn sc_send_journal(&mut self, book: &Book, focus: Option<usize>, loose: bool) {
        self.sc_book_runes(book);
        let data = serde_json::to_vec(&(book, focus, loose)).expect("serializable journal");
        if self.lan_acting_uuid.is_some() {
            use horizon_protocol::packets::{configuration::CustomPayload, play::Clientbound};
            let who = self.native_acting_id();
            self.native_send(
                who,
                Clientbound::CustomPayload(CustomPayload {
                    channel: super::sc_journal::VIEW.into(),
                    data,
                }),
            );
        } else {
            self.sc_receive_journal(&data);
        }
    }
    fn sc_marked_map(&mut self, e: &Entry) -> ItemStack {
        let mut d = MapData::fresh(e.pos[0] as f64, e.pos[2] as f64, 2, &e.dim);
        d.unlimited = true;
        let id = self.new_map_id(d);
        if e.dim == self.dim.id() {
            self.render_biome_preview(id);
        }
        let color = COLORS[e.color];
        let marker = match color {
            "black" | "blue" | "purple" | "red" | "brown" | "green" | "light_gray" => {
                format!("minecraft:banner_{color}")
            }
            _ => "minecraft:target_x".into(),
        };
        let mut out = stack("minecraft:filled_map");
        out.data_mut().map_id = Some(id);
        out.data_mut().set_other(
            "minecraft:map_decorations",
            Some(Tag::compound([(
                "+",
                Tag::compound([
                    ("type", s(marker)),
                    ("x", Tag::Double((e.pos[0] as f64).to_bits())),
                    ("z", Tag::Double((e.pos[2] as f64).to_bits())),
                    ("rotation", Tag::Float(180f32.to_bits())),
                ]),
            )])),
        );
        out
    }
}
