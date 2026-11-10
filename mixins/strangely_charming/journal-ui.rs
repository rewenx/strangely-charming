use super::{
    Game, HudBatch, Screen, rgba,
    sc_handbook::{self, PAGES},
    sc_travel::{self, Book, COLORS, Entry, RGB, s},
    shield::OFFHAND_SLOT,
};
use horizon_world::nbt::Tag;
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};
use winit::keyboard::KeyCode;
pub(super) const VIEW: &str = "strangely_charming:journal_view";
pub(super) const QUEST_VIEW: &str = "strangely_charming:quest_preview";
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct QuestPreview {
    pub(super) serial: u64,
    pub(super) kind: super::sc_quests::Kind,
    pub(super) progress: Option<u32>,
}
const QUEST_ROWS: usize = 3;
const QUEST_SCROLL: [f32; 4] = [239., 64., 7., 27.];
const TURN_IN: &str = "Once you've completed the criteria, shift right-click the scroll on a scrollkeeper to complete the quest.";
const OWNER: &str = "strangely_charming";
const ROWS: usize = 10;
const WIDTH: f32 = 306.;
// Native artwork includes the bookmark below the two parchment pages.
const BOOK_HEIGHT: f32 = 200.;
#[derive(Default, Clone, Copy, PartialEq)]
enum Spread {
    #[default]
    Locations,
    Entry(usize),
    Contents,
    Feature(usize),
    Gateway,
    Research,
}
#[derive(Clone, Copy, PartialEq)]
enum Modal {
    Glyph(u8),
    Record,
    Rename,
    Remove,
}
#[derive(Clone, Copy, PartialEq)]
enum Field {
    Search,
    Name,
}
#[derive(Clone)]
enum Action {
    Navigate,
    QuestChoice(&'static str),
    Build,
    MakeDiagram,
    Gateway,
    Research,
    Glyph(u8),
    Search,
    ClearSearch,
    Locations,
    Contents,
    Entry(usize),
    Feature(usize),
    Record,
    Rename,
    Remove,
    ConfirmRemove,
    Import,
    Cancel,
    Save,
    Ink,
    Photo,
    Map,
    Paper,
    Compass,
    Name,
    Scroll,
    Back,
    Next,
}
struct Control {
    rect: [f32; 4],
    action: Action,
    label: String,
    hint: String,
    enabled: bool,
}
impl Control {
    fn hit(&self, x: f32, y: f32) -> bool {
        let [a, b, w, h] = self.rect;
        x >= a && y >= b && x < a + w && y < b + h
    }
}
#[derive(Default)]
struct Journal {
    quest: Option<QuestPreview>,
    quest_lines: Vec<String>,
    book: Option<Book>,
    loose: bool,
    dismiss: bool,
    spread: Spread,
    scroll: usize,
    dragging: bool,
    modal: Option<Modal>,
    field: Option<Field>,
    query: String,
    name: String,
    color: usize,
    caret: usize,
    caret_since: Option<Instant>,
    anchor: usize,
    pending: Option<(String, Option<Tag>)>,
    photo: Option<String>,
    builder: Option<String>,
    sound: bool,
}
thread_local! {
    static UI: RefCell<Journal> = RefCell::default();
    static LOOK: RefCell<Lookup> = RefCell::default();
}
#[derive(Default)]
struct Lookup {
    hover: Option<(usize, Instant)>,
    hold: Option<(usize, Instant)>,
}
fn view(w: f32, h: f32) -> (f32, f32, f32) {
    let scale = ((w - 4.) / WIDTH).min((h - 4.) / BOOK_HEIGHT).clamp(0.1, 1.);
    (
        ((w / scale - WIDTH) / 2.).floor(),
        ((h / scale - BOOK_HEIGHT) / 2.).floor(),
        scale,
    )
}
fn ink() -> u32 {
    rgba(57, 39, 37, 255)
}
fn tone(color: usize) -> u32 {
    let h = u32::from_str_radix(&RGB[color][1..], 16).unwrap();
    rgba((h >> 16) as u8, (h >> 8) as u8, h as u8, 255)
}
fn pale_ink(color: usize) -> bool {
    matches!(color, 0 | 3 | 4 | 5 | 6 | 8)
}
fn point(g: &Game<'_>) -> (f32, f32) {
    let (w, h) = g.gui_size();
    let (x, y, k) = view(w, h);
    (g.cursor.0 / k - x, g.cursor.1 / k - y)
}
fn sprite(g: &Game<'_>, b: &mut HudBatch, n: &str, x: f32, y: f32, w: f32, h: f32) {
    b.sprite_scaled(
        &g.gui.atlas,
        &format!("strangely_charming:gui/journal/{n}"),
        x,
        y,
        w,
        h,
        u32::MAX,
    );
}
fn fit(g: &Game<'_>, s: &str, w: f32) -> String {
    if g.gui.atlas.font.width(s) as f32 <= w {
        return s.into();
    }
    let mut v = s.to_owned();
    while !v.is_empty() && g.gui.atlas.font.width(&format!("{v}…")) as f32 > w {
        v.pop();
    }
    format!("{v}…")
}
fn text(g: &Game<'_>, b: &mut HudBatch, t: &str, x: f32, y: f32, color: u32) {
    b.text(&g.gui.atlas, t, x, y, color, false);
}
fn lines(g: &Game<'_>, t: &str, w: f32) -> Vec<String> {
    let mut result = Vec::new();
    for part in t.split('\n') {
        if part.is_empty() {
            result.push(String::new());
            continue;
        }
        for row in horizon_client::multiplayer::wrap(part, w, &g.gui.atlas) {
            if g.gui.atlas.font.width(&row) <= w {
                result.push(row);
                continue;
            }
            let mut chunk = String::new();
            for ch in row.chars() {
                chunk.push(ch);
                if g.gui.atlas.font.width(&chunk) > w && chunk.len() > ch.len_utf8() {
                    chunk.pop();
                    result.push(std::mem::take(&mut chunk));
                    chunk.push(ch);
                }
            }
            result.push(chunk);
        }
    }
    result
}
fn paragraph(g: &Game<'_>, b: &mut HudBatch, t: &str, x: f32, y: f32, w: f32, rows: usize) {
    for (i, line) in lines(g, t, w).iter().take(rows).enumerate() {
        text(g, b, line, x, y + i as f32 * 10., ink());
    }
}
fn caption(
    g: &Game<'_>,
    b: &mut HudBatch,
    t: &str,
    x: f32,
    y: f32,
    width: f32,
    color: u32,
    shadow: bool,
) {
    let size = (width / g.gui.atlas.font.width(t).max(1.)).min(1.);
    let start = b.vertices.len();
    b.text(&g.gui.atlas, t, x, y, color, shadow);
    b.scale_since(start, x, y, size, size);
}
fn centered(g: &Game<'_>, b: &mut HudBatch, label: &str, middle: f32, y: f32,
            width: f32, max_scale: f32, color: u32, shadow: bool) {
    let measured = g.gui.atlas.font.width(label).max(1.);
    let scale = (width / measured).min(max_scale);
    let x = middle - measured * scale * 0.5;
    let start = b.vertices.len();
    b.text(&g.gui.atlas, label, x, y, color, shadow);
    b.scale_since(start, x, y, scale, scale);
}
fn handbook_text(g: &Game<'_>, b: &mut HudBatch, t: &str, x: f32, y: f32, height: f32) {
    // Fit the complete entry above its navigation at each GUI scale.
    for scale in [1.0, 0.95, 0.9, 0.85, 0.8, 0.75] {
        let rows = lines(g, t, 117. / scale);
        if rows.len() as f32 * 10. * scale <= height || scale == 0.75 {
            let start = b.vertices.len();
            for (r, line) in rows.iter().enumerate() {
                text(g, b, line, x, y + r as f32 * 10., ink());
            }
            b.scale_since(start, x, y, scale, scale);
            break;
        }
    }
}
fn coords(e: &Entry) -> String {
    format!("{}  {}  {}", e.pos[0], e.pos[1], e.pos[2])
}
fn pages() -> Vec<usize> {
    let mut indices: Vec<_> = (0..PAGES.len()).filter(|&i| PAGES[i].enabled()).collect();
    indices.sort_unstable_by_key(|&i| PAGES[i].title);
    indices
}
impl Journal {
    fn matches(&self) -> Vec<usize> {
        let search = self.query.trim().to_ascii_lowercase();
        pages()
            .into_iter()
            .filter(|&i| search.is_empty() || PAGES[i].title.to_ascii_lowercase().contains(&search))
            .collect()
    }
    fn entry(&self) -> Option<&Entry> {
        if let Spread::Entry(i) = self.spread {
            self.book.as_ref()?.entries.get(i)
        } else {
            None
        }
    }
    fn go(&mut self, spread: Spread) {
        self.spread = spread;
        self.modal = None;
        self.field = None;
        self.dragging = false;
        self.sound = true;
        if let Some(e) = self.entry().cloned() {
            self.name = e.name;
            self.color = e.color;
        }
    }
    fn max_scroll(&self) -> usize {
        if self.quest.is_some() {
            return self.quest_lines.len().saturating_sub(QUEST_ROWS);
        }
        match self.spread {
            Spread::Locations => self
                .book
                .as_ref()
                .map_or(0, |b| b.entries.len())
                .saturating_sub(ROWS),
            Spread::Research => super::sc_runes::CLIENT
                .with_borrow(|r| r.learned.len().saturating_sub(12).div_ceil(2)),
            Spread::Contents => self.matches().len().saturating_sub(ROWS * 2).div_ceil(2),
            _ => 0,
        }
    }
    fn scroll_to(&mut self, y: f32) {
        let (top, travel) = if self.quest.is_some() {
            (QUEST_SCROLL[1], QUEST_SCROLL[3] - 12.)
        } else {
            (42., 107.)
        };
        self.scroll = ((y - top - 6.) / travel * self.max_scroll() as f32)
            .round()
            .clamp(0., self.max_scroll() as f32) as usize;
    }
    fn focus(&mut self, field: Field) {
        self.field = Some(field);
        self.caret_since = Some(Instant::now());
        self.caret = match field {
            Field::Name => &self.name,
            Field::Search => &self.query,
        }
        .chars()
        .count();
        self.anchor = self.caret;
    }
    fn edit(&mut self, f: impl FnOnce(&mut horizon_client::ui::EditBox)) {
        let Some(field) = self.field else { return };
        let (value, max) = match field {
            Field::Name => (&mut self.name, 24),
            Field::Search => (&mut self.query, 48),
        };
        let mut edit = horizon_client::ui::EditBox::new(0., 0., 114., 60.);
        edit.max_length = max;
        edit.set_value(value);
        edit.set_cursor(self.anchor, false);
        edit.set_cursor(self.caret, true);
        f(&mut edit);
        if edit.value.encode_utf16().count() <= max {
            if field == Field::Search && *value != edit.value {
                self.scroll = 0;
            }
            *value = edit.value;
            self.caret = edit.cursor;
            self.anchor = edit.highlight;
            self.caret_since = Some(Instant::now());
        }
    }
    fn draw_edit(
        &self,
        g: &Game<'_>,
        b: &mut HudBatch,
        value: &str,
        x: f32,
        y: f32,
        width: f32,
        color: u32,
    ) {
        let font = &g.gui.atlas.font;
        let byte = value
            .char_indices()
            .nth(self.caret)
            .map_or(value.len(), |(i, _)| i);
        let mut left = 0;
        while left < byte && font.width(&value[left..byte]) > width - 2. {
            left += value[left..].chars().next().unwrap().len_utf8();
        }
        let visible = &value[left..];
        let mut end = visible.len();
        while end > 0 && font.width(&visible[..end]) > width {
            end = visible[..end].char_indices().next_back().unwrap().0;
        }
        text(g, b, &visible[..end], x, y, color);
        if self
            .caret_since
            .is_none_or(|t| t.elapsed().as_millis() % 1000 < 500)
        {
            b.fill(
                &g.gui.atlas,
                x + font.width(&value[left..byte]),
                y - 1.,
                1.,
                9.,
                color,
            );
        }
    }
    fn request(&mut self, op: &str, payload: Option<Tag>) {
        if let Some(book) = &self.book {
            self.pending = Some((book.addr(op), payload));
        }
    }
    fn controls(&self, g: &Game<'_>) -> Vec<Control> {
        let mut c = vec![];
        let mut add = |rect, action, label: &str, hint: &str, enabled| {
            c.push(Control {
                rect,
                action,
                label: label.into(),
                hint: hint.into(),
                enabled,
            })
        };
        if let Some(quest) = &self.quest {
            if quest.progress.is_none() {
                add([41., 169., 65., 19.], Action::QuestChoice("accept"), "Accept", "", true);
                add([117., 169., 65., 19.], Action::QuestChoice("decline"), "Reject", "Reject this offer and discard the scroll", true);
                add([193., 169., 65., 19.], Action::QuestChoice("close"), "Close", "Keep this offer for later", true);
            } else {
                add([79., 169., 65., 19.], Action::QuestChoice("abandon"), "Reject", "Abandon this active quest", true);
                add([155., 169., 65., 19.], Action::QuestChoice("close"), "Close", "", true);
            }
            if self.max_scroll() > 0 {
                add(QUEST_SCROLL, Action::Scroll, "", "Scroll quest text", true);
            }
            return c;
        }
        if self.loose {
            add([76., 120., 64., 15.], Action::Import, "Add", "", true);
            add([149., 120., 64., 15.], Action::Cancel, "Cancel", "", true);
            return c;
        }
        if let Some(modal) = self.modal {
            if modal != Modal::Remove {
                add(
                    [76., 62., 137., 16.],
                    Action::Name,
                    &self.name,
                    if matches!(modal, Modal::Glyph(_)) {
                        "Rune name"
                    } else {
                        "Location name"
                    },
                    true,
                );
                if !matches!(modal, Modal::Glyph(_)) {
                    add(
                        [76., 87., 137., 16.],
                        Action::Ink,
                        &COLORS[self.color].replace('_', " "),
                        "Change ink color",
                        true,
                    );
                }
            }
            add(
                [76., 120., 64., 15.],
                if modal == Modal::Remove {
                    Action::ConfirmRemove
                } else {
                    Action::Save
                },
                if modal == Modal::Remove {
                    "Remove"
                } else {
                    "Save"
                },
                "",
                modal == Modal::Remove || !self.name.trim().is_empty(),
            );
            add([149., 120., 64., 15.], Action::Cancel, "Cancel", "", true);
            return c;
        }
        add(
            [282., 29., 23., 19.],
            Action::Locations,
            "",
            "Gazetteer",
            true,
        );
        add(
            [285., 55., 21., 19.],
            Action::Contents,
            "",
            "Vade Mecum",
            true,
        );
        if super::sc_runes::enabled() {
            add(
                [281., 83., 24., 19.],
                Action::Gateway,
                "",
                "Teleport Log",
                true,
            );
            add(
                [284., 111., 22., 19.],
                Action::Research,
                "",
                "Runic Research",
                true,
            );
        }
        match self.spread {
            Spread::Gateway => {}
            Spread::Research => super::sc_runes::CLIENT.with_borrow(|r| {
                for (j, &id) in r.learned.iter().skip(self.scroll * 2).take(12).enumerate() {
                    let x = if j % 2 == 0 { 18. } else { 155. };
                    add(
                        [x + 90., 42. + (j / 2) as f32 * 21., 24., 15.],
                        Action::Glyph(id),
                        r.name(id),
                        "edit name",
                        true,
                    );
                }
            }),
            Spread::Locations => {
                let n = self.book.as_ref().map_or(0, |b| b.entries.len());
                let ok = self.book.is_some() && !self.loose && n < 30;
                add(
                    [62., 106., 20., 20.],
                    Action::Record,
                    "",
                    if ok {
                        "Record this Location"
                    } else if n == 30 {
                        "Journal is full (30 locations)"
                    } else {
                        "Hold your journal to record a location"
                    },
                    ok,
                );
                if let Some(book) = &self.book {
                    for (r, e) in book.entries.iter().skip(self.scroll).take(ROWS).enumerate() {
                        add(
                            [155., 42. + r as f32 * 12., 112., 12.],
                            Action::Entry(self.scroll + r),
                            &e.name,
                            &e.name,
                            true,
                        );
                    }
                }
            }
            Spread::Entry(i) => {
                if let Some(e) = self.entry() {
                    if !self.loose {
                        if super::sc_runes::enabled() {
                            let known = super::sc_runes::DIAGRAMS.with_borrow(|d| {
                                d.get(&e.id).is_some_and(|a| {
                                    super::sc_runes::CLIENT
                                        .with_borrow(|r| a.iter().all(|n| r.learned.contains(n)))
                                })
                            });
                            let enough = g.sc_amethyst_count() >= 12;
                            add(
                                [191., 81., 20., 20.],
                                Action::Build,
                                "",
                                if !known {
                                    "Learn every glyph in this address"
                                } else if !enough {
                                    "Requires 12 amethyst blocks"
                                } else {
                                    "Preview portal frame"
                                },
                                known && enough,
                            );
                            let materials = g.sc_journal_material("minecraft:paper").is_some()
                                && g.sc_journal_material("strangely_charming:rune_dust")
                                    .is_some();
                            add(
                                [213., 81., 20., 20.],
                                Action::MakeDiagram,
                                "",
                                if !known {
                                    "Learn every glyph in this address"
                                } else if !materials {
                                    "Requires 1 paper and 1 Rune Dust"
                                } else {
                                    "Make Diagram (1 paper and 1 Rune Dust)"
                                },
                                known && materials,
                            );
                        }
                        let near = g.sc_near_entry(e);
                        let photo = super::sc_photo::has_photo(&e.id);
                        add(
                            [52., 132., 25., 19.],
                            Action::Photo,
                            "",
                            if !near {
                                "Return to this location to take a photo"
                            } else if photo {
                                "Replace Photo"
                            } else {
                                "Take a Photo"
                            },
                            near,
                        );
                        add([80., 132., 25., 19.], Action::Navigate, "", "Point me there", true);
                        for (x, action, item, hint, missing) in [
                            (
                                176.,
                                Action::Compass,
                                "minecraft:compass",
                                "Find location",
                                "Find location – requires a compass",
                            ),
                            (
                                202.,
                                Action::Paper,
                                "minecraft:paper",
                                "Copy to paper",
                                "Copy to paper – requires paper",
                            ),
                            (
                                228.,
                                Action::Map,
                                "minecraft:map",
                                "Copy to map",
                                "Copy to map – requires a blank map",
                            ),
                        ] {
                            let available = g.sc_journal_material(item).is_some();
                            add(
                                [x, 131., 22., 20.],
                                action,
                                "",
                                if available { hint } else { missing },
                                available,
                            );
                        }
                        add(
                            [156., 151., 41., 16.],
                            Action::Rename,
                            "Edit",
                            "Edit name and ink",
                            true,
                        );
                        add(
                            [201., 151., 19., 16.],
                            Action::Remove,
                            "Delete",
                            "Remove this location",
                            true,
                        );
                        add(
                            [225., 151., 45., 16.],
                            Action::Locations,
                            "Back",
                            "My Saved Locations",
                            true,
                        );
                    }
                    let _ = i;
                }
            }
            Spread::Contents => {
                add([155., 20., 97., 14.], Action::Search, &self.query, "", true);
                if !self.query.is_empty() {
                    add(
                        [253., 20., 14., 14.],
                        Action::ClearSearch,
                        "×",
                        "Clear search",
                        true,
                    );
                }
                for (r, &idx) in self
                    .matches()
                    .iter()
                    .skip(self.scroll * 2)
                    .take(ROWS * 2)
                    .enumerate()
                {
                    let x = if r % 2 == 0 { 18. } else { 155. };
                    add(
                        [x, 42. + (r / 2) as f32 * 12., 112., 12.],
                        Action::Feature(idx),
                        PAGES[idx].title,
                        PAGES[idx].title,
                        true,
                    );
                }
            }
            Spread::Feature(index) => {
                let list = pages();
                let row = list.iter().position(|i| *i == index).unwrap_or(0);
                let start = row / 2 * 2;
                for (col, &idx) in list.iter().skip(start).take(2).enumerate() {
                    let x = if col == 0 { 19. } else { 156. };
                    if PAGES[idx].items.contains(&sc_travel::BOOK) {
                        add(
                            [x, 132., 111., 13.],
                            Action::Locations,
                            "Open Gazetteer",
                            "Gazetteer",
                            true,
                        );
                    }
                }
                let width = 45.;
                add(
                    [212. - width / 2., 143., width, 16.],
                    Action::Contents,
                    "Back to guide",
                    "Back to guide index",
                    true,
                );
                if start > 0 {
                    add(
                        [21., 157., 18., 12.],
                        Action::Back,
                        "",
                        "Previous spread",
                        true,
                    );
                }
                if start + 2 < list.len() {
                    add(
                        [249., 157., 18., 12.],
                        Action::Next,
                        "",
                        "Next spread",
                        true,
                    );
                }
            }
        }
        if self.max_scroll() > 0 {
            add(
                [268., 42., 6., 119.],
                Action::Scroll,
                "",
                "Scroll entries",
                true,
            );
        }
        c
    }
    fn act(&mut self, g: &Game<'_>, action: Action) {
        match action {
            Action::Gateway => {
                self.scroll = 0;
                self.go(Spread::Gateway);
            }
            Action::Research => {
                self.scroll = 0;
                self.go(Spread::Research);
            }
            Action::Glyph(id) => {
                self.name = super::sc_runes::CLIENT.with_borrow(|r| r.name(id).to_owned());
                self.modal = Some(Modal::Glyph(id));
                self.focus(Field::Name);
            }
            Action::Entry(i) => self.go(Spread::Entry(i)),
            Action::Feature(i) => self.go(Spread::Feature(i)),
            Action::Contents => {
                self.scroll = 0;
                self.go(Spread::Contents);
            }
            Action::Locations => {
                self.scroll = 0;
                self.go(Spread::Locations);
            }
            Action::Back | Action::Next => {
                let Spread::Feature(i) = self.spread else {
                    return;
                };
                let list = pages();
                let row = list.iter().position(|&p| p == i).unwrap_or(0) / 2 * 2;
                if matches!(action, Action::Back) && row < 2 {
                    self.go(Spread::Contents);
                } else if let Some(&p) = list.get(if matches!(action, Action::Back) {
                    row - 2
                } else {
                    row + 2
                }) {
                    self.go(Spread::Feature(p));
                }
            }
            Action::Record => {
                self.name = g
                    .player
                    .as_ref()
                    .map(|p| {
                        g.biome_at([p.position.x, p.position.y, p.position.z])
                            .split(':')
                            .next_back()
                            .unwrap_or("Location")
                            .replace('_', " ")
                    })
                    .unwrap_or_default()
                    .chars()
                    .take(24)
                    .collect();
                if let Some(c) = self.name.get_mut(..1) {
                    c.make_ascii_uppercase();
                }
                self.color = 15;
                self.modal = Some(Modal::Record);
                self.focus(Field::Name);
            }
            Action::Rename => {
                self.modal = Some(Modal::Rename);
                self.focus(Field::Name);
            }
            Action::Remove => {
                self.modal = Some(Modal::Remove);
                self.field = None;
            }
            Action::Import => {
                if let Some(id) = self.entry().map(|e| e.id.clone()) {
                    self.request(&format!("import/{id}"), None);
                    self.dismiss = true;
                }
            }
            Action::Cancel if self.loose => self.dismiss = true,
            Action::Cancel => {
                self.modal = None;
                self.field = None;
                if let Some(e) = self.entry().cloned() {
                    self.name = e.name;
                    self.color = e.color;
                }
            }
            Action::Ink => self.color = (self.color + 1) % 16,
            Action::Search => self.focus(Field::Search),
            Action::ClearSearch => {
                self.query.clear();
                self.scroll = 0;
                self.focus(Field::Search);
            }
            Action::Name => self.focus(Field::Name),
            Action::Scroll => {
                self.dragging = true;
                self.scroll_to(point(g).1);
            }
            Action::Save if matches!(self.modal, Some(Modal::Glyph(_))) => {
                if let Some(Modal::Glyph(id)) = self.modal
                    && !self.name.trim().is_empty()
                {
                    self.request(
                        &format!("rune_name/{id}"),
                        Some(Tag::compound([("name", s(self.name.trim()))])),
                    );
                    self.modal = None;
                    self.field = None;
                }
            }
            Action::Save => {
                let op = if self.modal == Some(Modal::Record) {
                    "add".to_string()
                } else if let Spread::Entry(i) = self.spread {
                    format!("save/{i}")
                } else {
                    return;
                };
                self.request(
                    &op,
                    Some(Tag::compound([
                        ("name", s(&self.name)),
                        ("color", s(COLORS[self.color])),
                    ])),
                );
            }
            Action::ConfirmRemove
            | Action::Map
            | Action::Paper
            | Action::Compass
            | Action::MakeDiagram => {
                if let Spread::Entry(i) = self.spread {
                    let op = match action {
                        Action::MakeDiagram => "make_diagram",
                        Action::Map => "map",
                        Action::Paper => "share",
                        Action::Compass => "compass",
                        _ => "delete",
                    };
                    self.request(&format!("{op}/{i}"), None);
                }
            }
            Action::Build => {
                if let (Some(book), Spread::Entry(i)) = (&self.book, self.spread) {
                    self.builder = Some(book.addr(&format!("build/{i}")));
                }
            }
            Action::QuestChoice(choice) => {
                if let Some(preview) = &self.quest {
                    if choice != "close" {
                        self.pending = Some((format!("strangely_charming:quest/{choice}/{}", preview.serial), None));
                    }
                    self.dismiss = true;
                }
            }
            Action::Navigate => {
                if let Some(entry) = self.entry() {g.sc_point_to(entry);self.dismiss = true;}
            }
            Action::Photo => self.photo = self.entry().map(|e| e.id.clone()),
        }
    }
}
impl Game<'_> {
    fn sc_own_journal(&self) -> bool {
        self.native_mod_screen() == Some(OWNER)
    }
    pub(super) fn sc_receive_quest_preview(&mut self, data: &[u8]) {
        if data.len() > 65536 || !horizon_mods::evanders_config::enabled("scrollkeepers") {return;}
        let Ok(preview) = serde_json::from_slice::<QuestPreview>(data) else {return};
        if self.native_mod_screen_open(OWNER) {
            let quest_lines = lines(self, &format!("{}.", preview.kind.objective()), 165.);
            UI.with_borrow_mut(|u| *u = Journal {
                quest: Some(preview),
                quest_lines,
                ..Journal::default()
            });
        }
    }
    pub(super) fn sc_receive_journal(&mut self, data: &[u8]) {
        if !sc_travel::handles(sc_travel::BOOK) || data.len() > 65536 {
            return;
        }
        let Ok((book, focus, loose)) = serde_json::from_slice::<(Book, Option<usize>, bool)>(data)
        else {
            return;
        };
        if !sc_travel::valid_id(&book.id)
            || book.entries.len() > 30
            || !book.entries.iter().all(Entry::valid)
            || focus.is_some_and(|i| i >= book.entries.len())
            || (loose && (book.entries.len() != 1 || focus != Some(0)))
        {
            return;
        }
        if !self.native_mod_screen_open(OWNER) {
            return;
        }
        UI.with_borrow_mut(|u| {
            *u = Journal {
                book: Some(book),
                loose,
                ..Journal::default()
            };
            u.go(focus.map_or(Spread::Locations, Spread::Entry));
        });
    }
    pub(super) fn sc_draw_journal(&self, b: &mut HudBatch) {
        if !self.sc_own_journal() {
            return;
        }
        let start = b.vertices.len();
        let (x, y, k) = view(b.width, b.height);
        let mouse = point(self);
        UI.with_borrow(|u| self.sc_draw_spread(b, u, x, y, k, mouse));
        b.scale_since(start, 0., 0., k, k);
    }
    fn sc_draw_spread(
        &self,
        b: &mut HudBatch,
        u: &Journal,
        x: f32,
        y: f32,
        k: f32,
        mouse: (f32, f32),
    ) {
        if let Some(quest) = &u.quest {
            b.fill(&self.gui.atlas, 0., 0., b.width / k, b.height / k, rgba(34, 26, 19, 115));
            let middle=x+149.;
            sprite(self,b,"quest_accept_scroll",middle-109.,y+9.,218.,129.);
            sprite(self, b, "quest_salutation", middle - 66., y + 29., 132., 31.3);
            for (i, row) in u.quest_lines.iter().skip(u.scroll).take(QUEST_ROWS).enumerate() {
                centered(self, b, row, middle, y + 64. + i as f32 * 9., 165., 0.9, ink(), false);
            }
            // Keep the request in the voice of the painted salutation. Contract details
            // sit below their own ink rule and never scroll into the sentence.
            let reward = quest.kind.reward();
            b.fill(&self.gui.atlas, middle - 53., y + 94., 106., 0.6, rgba(142, 102, 60, 154));
            centered(self, b, &format!("Reward: {reward} emeralds and {} XP", reward * 3), middle, y + 98., 165., 0.82, ink(), false);
            centered(self, b, &format!("Hero of the Village {}", quest.kind.hero_name()), middle, y + 106.5, 165., 0.82, ink(), false);
            if let Some(progress) = quest.progress {
                centered(self, b, &format!("Progress: {progress}/{}", quest.kind.target()), middle, y + 115., 165., 0.82, ink(), false);
            }
            for (i,line) in lines(self,TURN_IN,264.).iter().enumerate() {
                centered(self,b,line,middle,y+143.+8.*i as f32,223.,0.80,u32::MAX,true);
            }
        } else if u.loose {
            b.fill(
                &self.gui.atlas,
                x + 65.,
                y + 52.,
                160.,
                95.,
                rgba(97, 62, 38, 255),
            );
            b.fill(
                &self.gui.atlas,
                x + 67.,
                y + 54.,
                156.,
                91.,
                rgba(250, 238, 205, 255),
            );
            text(self, b, "Location Paper", x + 76., y + 62., ink());
            if let Some(e) = u.entry() {
                paragraph(
                    self,
                    b,
                    &format!("Add {} to your journal?", e.name),
                    x + 76.,
                    y + 80.,
                    138.,
                    3,
                );
            }
        } else {
            sprite(self, b, "journal_new_2", x, y, 290., BOOK_HEIGHT);
            let title = match u.spread {
                Spread::Locations => Some(("gazetteer_title", 155., 20.3)),
                Spread::Contents => Some(("vademecum_title", 18., 20.)),
                _ => None,
            };
            if let Some((name, dx, height)) = title {
                sprite(self, b, name, x + dx, y + 16., 114., height);
            }
            match u.spread {
                Spread::Locations => {
                    paragraph(
                        self,
                        b,
                        "Keep a record of the places you find. Use the quill to save your current location.",
                        x + 20.,
                        y + 42.,
                        112.,
                        6,
                    );
                    if u.book.as_ref().is_none_or(|b| b.entries.is_empty()) {
                        paragraph(
                            self,
                            b,
                            if u.book.is_none() {
                                "Hold and open your journal to see your saved places."
                            } else {
                                "Your recorded locations will appear here."
                            },
                            x + 156.,
                            y + 42.,
                            110.,
                            6,
                        );
                    }
                }
                Spread::Entry(_) => {
                    if let Some(e) = u.entry() {
                        let (px, py) = (x + 21., y + 25.);
                        let photo = self.sc_journal_thumbnail(b, &e.id, px + 5., py + 5.);
                        sprite(self, b, "polaroid", px, py, 106., 104.);
                        if !photo {
                            text(
                                self,
                                b,
                                "No photograph",
                                px + 16.,
                                py + 37.,
                                rgba(92, 83, 77, 255),
                            );
                        }
                        sprite(self, b, "paperclip", px + 78., py - 4., 9., 21.);
                        caption(self, b, &e.name, px + 5., py + 82., 96., ink(), false);
                        caption(self, b, &coords(e), px + 5., py + 92., 96., ink(), false);
                        let tx = x + 155.;
                        centered(self,b,&e.name,tx+57.,y+20.,114.,1.,tone(e.color),pale_ink(e.color));
                        centered(self,b,&e.dimension_name(),tx+57.,y+31.,114.,0.78,ink(),false);
                        sprite(self,b,"doc_hr",tx,y+39.,114.,3.);
                        if super::sc_runes::enabled() {
                            super::sc_runes::DIAGRAMS.with_borrow(|d| {
                                if let Some(a) = d.get(&e.id) {
                                    sprite(self,b,"rune_frame",tx+6.,y+43.,102.,86.);
                                    super::sc_runes::CLIENT.with_borrow(|r| {
                                        for (i, id) in a.iter().enumerate() {
                                            let (column, row) = super::sc_runes::FRAME_GLYPHS[i];
                                            // Cell centers measured on the painted frame; the four corners stay empty.
                                            let cx = tx + 57. + column as f32 * 16.2;
                                            let cy = y + [119., 105.5, 89., 72.5, 56.][row as usize];
                                            if r.learned.contains(id) {
                                                sprite(
                                                    self,
                                                    b,
                                                    &format!("rune_{id}"),
                                                    cx - 6.,
                                                    cy - 6.,
                                                    12.,
                                                    12.,
                                                );
                                            } else {
                                                centered(self, b, "?", cx, cy - 4., 12., 1., ink(), false);
                                            }
                                        }
                                    });
                                } else {
                                    caption(
                                        self,
                                        b,
                                        "Address unavailable",
                                        tx,
                                        y + 87.,
                                        115.,
                                        ink(),
                                        false,
                                    );
                                }
                            });
                        }
                    }
                }
                Spread::Gateway => {
                    sprite(self, b, "teleport_log_header", x + 18., y + 15., 112., 19.);
                    sprite(self, b, "origin_header", x + 50., y + 36., 48., 14.);
                    sprite(self, b, "destination_header", x + 183., y + 36., 57., 14.);
                    super::sc_runes::CLIENT.with_borrow(|r| {
                        if r.trips.is_empty() {
                            paragraph(
                                self,
                                b,
                                "Rune and portal journeys appear here after you arrive.",
                                x + 20.,
                                y + 55.,
                                105.,
                                5,
                            );
                        }
                        for (i, trip) in r.trips.iter().enumerate() {
                            for (xx, site) in [(18., &trip.from), (155., &trip.to)]
                            {
                                let top = y + 56. + i as f32 * 23.;
                                if i > 0 {
                                    b.fill(
                                        &self.gui.atlas,
                                        x + xx,
                                        top - 3.,
                                        111.,
                                        1.,
                                        rgba(192, 164, 122, 255),
                                    );
                                }
                                let inset = if xx == 18.
                                    && let Some(icon) = trip.icon
                                {
                                    match icon {
                                        super::sc_runes::TripIcon::Mishap => {
                                            let start = b.vertices.len();
                                            text(self, b, "?", x + xx + 4., top + 2., ink());
                                            b.scale_since(start, x + xx + 4., top + 2., 1.5, 1.5);
                                        }
                                        super::sc_runes::TripIcon::Carving(g) => sprite(
                                            self,
                                            b,
                                            &format!("rune_{g}"),
                                            x + xx,
                                            top + 1.,
                                            16.,
                                            16.,
                                        ),
                                        super::sc_runes::TripIcon::Amethyst(g) => self
                                            .draw_item_model(
                                                b,
                                                &sc_travel::stack(&format!(
                                                    "strangely_charming:runic_amethyst_{g}"
                                                )),
                                                x + xx,
                                                top + 1.,
                                            ),
                                    }
                                    18.
                                } else {
                                    0.
                                };
                                let dimension = sc_travel::dimension_name(&site.dim);
                                let name = if i == 0 && xx == 18. { format!("Latest · {dimension}") } else { dimension };
                                caption(self, b, &name, x + xx + inset, top, 110. - inset, ink(), false);
                                text(
                                    self,
                                    b,
                                    &fit(
                                        self,
                                        &format!(
                                            "{}  {}  {}",
                                            site.pos[0], site.pos[1], site.pos[2]
                                        ),
                                        110. - inset,
                                    ),
                                    x + xx + inset,
                                    top + 10.,
                                    ink(),
                                );
                            }
                        }
                    });
                }
                Spread::Research => {
                    sprite(
                        self,
                        b,
                        "runic_research_header",
                        x + 18.,
                        y + 18.,
                        112.,
                        15.3,
                    );
                    super::sc_runes::CLIENT.with_borrow(|r| {
                        caption(
                            self,
                            b,
                            &format!("{} / 26", r.learned.len()),
                            x + 155.,
                            y + 21.,
                            111.,
                            ink(),
                            false,
                        )
                    });
                    sprite(self, b, "doc_hr", x + 155., y + 33., 112., 3.);
                    if super::sc_runes::CLIENT.with_borrow(|r| r.learned.is_empty()) {
                        paragraph(
                            self,
                            b,
                            "Use a pearl on a carved rune to begin your research.",
                            x + 18.,
                            y + 39.,
                            107.,
                            6,
                        );
                    }
                }
                Spread::Contents => {
                    if u.matches().is_empty() {
                        text(self, b, "No matching entries.", x + 18., y + 42., ink());
                    }
                }
                Spread::Feature(i) => {
                    let list = pages();
                    let first = list.iter().position(|p| *p == i).unwrap_or(0) / 2 * 2;
                    for (col, &idx) in list.iter().skip(first).take(2).enumerate() {
                        let p = &PAGES[idx];
                        let px = x + if col == 0 { 18. } else { 155. };
                        let title = lines(self, p.title, 114.);
                        for (r, line) in title.iter().enumerate() {
                            text(
                                self,
                                b,
                                line,
                                px + (114. - self.gui.atlas.font.width(line)) * 0.5,
                                y + 21. + r as f32 * 10.,
                                ink(),
                            );
                        }
                        let mut top = y + 24. + title.len() as f32 * 10.;
                        sprite(self, b, "doc_hr", px, top - 3., 114., 3.);
                        if let Some(icon) = p.illustration() {
                            self.draw_item_model(b, &sc_travel::stack(icon), px + 49., top);
                            top += 19.;
                        }
                        let bottom = if p.key == "travel_journal" {
                            129.
                        } else {
                            140.
                        };
                        handbook_text(self, b, &p.description(self), px, top, y + bottom - top);
                        text(
                            self,
                            b,
                            &format!("{}", first + col + 1),
                            px + 50.,
                            y + 162.,
                            ink(),
                        );
                    }
                }
            }
            if let Some(m) = u.modal {
                b.fill(
                    &self.gui.atlas,
                    x + 65.,
                    y + 35.,
                    160.,
                    113.,
                    rgba(97, 62, 38, 255),
                );
                b.fill(
                    &self.gui.atlas,
                    x + 67.,
                    y + 37.,
                    156.,
                    109.,
                    rgba(250, 238, 205, 255),
                );
                text(
                    self,
                    b,
                    match m {
                        Modal::Record => "Record this Location",
                        Modal::Rename => "Edit location",
                        Modal::Remove => "Remove location?",
                        Modal::Glyph(_) => "Name this glyph",
                    },
                    x + 76.,
                    y + 46.,
                    ink(),
                );
                if m == Modal::Remove {
                    paragraph(
                        self,
                        b,
                        "This removes the entry from this journal.",
                        x + 76.,
                        y + 70.,
                        131.,
                        4,
                    );
                }
            }
        }
        let mut tip = None;
        for c in u.controls(self) {
            let [dx, dy, w, h] = c.rect;
            let (px, py) = (x + dx, y + dy);
            let hot = c.hit(mouse.0, mouse.1);
            if hot && !c.hint.is_empty() {
                let missing =
                    !c.enabled && matches!(c.action, Action::Compass | Action::Map | Action::Paper);
                tip = Some((
                    c.hint.clone(),
                    if missing {
                        rgba(255, 100, 100, 255)
                    } else {
                        u32::MAX
                    },
                ));
            }
            let fg = if c.enabled {
                ink()
            } else {
                rgba(114, 110, 105, 255)
            };
            match c.action {
                Action::Locations | Action::Contents | Action::Gateway | Action::Research
                    if dx > 280. =>
                {
                    let color = match c.action {
                        Action::Locations => "red",
                        Action::Gateway => "cyan",
                        Action::Research => "green",
                        _ => "blue",
                    };
                    let on = matches!(
                        (&c.action, u.spread),
                        (Action::Locations, Spread::Locations | Spread::Entry(_))
                            | (Action::Contents, Spread::Contents | Spread::Feature(_))
                            | (Action::Gateway, Spread::Gateway)
                            | (Action::Research, Spread::Research)
                    );
                    sprite(
                        self,
                        b,
                        &format!("{color}_tab{}", if on || hot { "_selected" } else { "" }),
                        px - 2.,
                        py + 1.,
                        w + 3.,
                        15.,
                    );
                    let ix = px + 3.;
                    let iy = py + 3.;
                    let start = b.vertices.len();
                    self.draw_item_model(
                        b,
                        &sc_travel::stack(match c.action {
                            Action::Locations => "minecraft:filled_map",
                            Action::Gateway => super::sc_runes::PEARL,
                            Action::Research => "strangely_charming:rune_dust",
                            _ => "minecraft:book",
                        }),
                        ix,
                        iy,
                    );
                    b.scale_since(start, ix, iy, 0.75, 0.75);
                }

                Action::Glyph(id) => {
                    sprite(self, b, &format!("rune_{id}"), px - 90., py, 16., 16.);
                    text(self, b, &fit(self, &c.label, 68.), px - 71., py + 4., ink());
                    sprite(
                        self,
                        b,
                        if hot { "rename_glyph_hover" } else { "rename_glyph" },
                        px,
                        py,
                        w,
                        h,
                    );
                }
                Action::Rename | Action::Remove | Action::Locations
                    if matches!(u.spread, Spread::Entry(_)) =>
                {
                    let name = match c.action {
                        Action::Rename => "edit_button",
                        Action::Remove => "trash_button",
                        _ => "back_button",
                    };
                    sprite(self, b, &format!("{name}{}",if hot {"_hover"} else {""}), px, py, w, h);
                }
                Action::Back | Action::Next => {
                    let start = b.vertices.len();
                    b.sprite_scaled(&self.gui.atlas, "strangely_charming:gui/journal/journal_bullet_arrow",
                        px + 4., py, 10., 10., if hot {u32::MAX} else {rgba(197,165,128,255)});
                    if matches!(c.action, Action::Back) {
                        b.scale_since(start, px + w / 2., py, -1., 1.);
                    }
                }
                Action::Record
                | Action::Build
                | Action::Compass
                | Action::Paper
                | Action::Map
                | Action::Photo
                | Action::Navigate
                | Action::MakeDiagram => {
                    let name = match c.action {
                        Action::Compass => "compass",
                        Action::Record => "feather_new_location",
                        Action::Build => "hammer",
                        Action::Paper => "write_scroll",
                        Action::MakeDiagram => "feather",
                        Action::Navigate => "point_there",
                        Action::Map => "paper",
                        _ => "camera",
                    };
                    let suffix = if c.enabled && hot { "_hover" } else { "" };
                    let tint = if c.enabled {
                        u32::MAX
                    } else {
                        rgba(153, 153, 153, 135)
                    };
                    b.sprite_scaled(
                        &self.gui.atlas,
                        &format!("strangely_charming:gui/journal/{name}{suffix}"),
                        px + (w - 20.) / 2.,
                        py + (h - 16.) / 2.,
                        20.,
                        16.,
                        tint,
                    );
                }
                Action::QuestChoice(choice) => {
                    let image = match choice {
                        "accept" => "quest_accept",
                        "decline" | "abandon" => "quest_reject",
                        _ => "quest_close",
                    };
                    let suffix = if hot { "_hover" } else { "" };
                    sprite(self, b, &format!("{image}{suffix}"), px, py, w, h);
                }
                Action::Scroll => {
                    b.fill(
                        &self.gui.atlas,
                        px + 2.,
                        py,
                        3.,
                        h,
                        rgba(204, 182, 146, 255),
                    );
                    b.fill(
                        &self.gui.atlas,
                        px,
                        py + u.scroll as f32 / u.max_scroll().max(1) as f32 * (h - 12.),
                        7.,
                        12.,
                        rgba(130, 93, 68, 255),
                    );
                }
                Action::Feature(_) | Action::Entry(_) => {
                    if hot {
                        b.fill(&self.gui.atlas, px, py, w, h, rgba(188, 138, 72, 42));
                        b.fill(&self.gui.atlas, px, py + 1., 1., h - 2., rgba(151, 99, 53, 185));
                    }
                    let (ix, iy) = (px + 3., py + 2.);
                    let mut color = fg;
                    let mut shadow = false;
                    match c.action {
                        Action::Feature(i) => {
                            if let Some(item) = PAGES[i].illustration() {
                                let start = b.vertices.len();
                                self.draw_item_model(b, &sc_travel::stack(item), ix, iy);
                                b.scale_since(start, ix, iy, 0.5, 0.5);
                            } else {
                                sprite(self, b, "journal_bullet_arrow", ix + 1., iy, 5.5, 8.);
                            }
                        }
                        Action::Entry(i) => {
                            sprite(self, b, "journal_bullet_reticle", ix, iy, 8., 8.);
                            if let Some(entry) =
                                u.book.as_ref().and_then(|book| book.entries.get(i))
                            {
                                color = tone(entry.color);
                                shadow = pale_ink(entry.color);
                            }
                        }
                        _ => unreachable!(),
                    }
                    b.text(
                        &self.gui.atlas,
                        &fit(self, &c.label, w - 16.),
                        px + 14.,
                        py + 2.,
                        color,
                        shadow,
                    );
                }
                Action::Contents if matches!(u.spread, Spread::Feature(_)) => {
                    sprite(self, b, "back_button", px + (w - 45.) * 0.5, py, 45., 16.);
                }
                Action::Search => {
                    let active = u.field == Some(Field::Search);
                    let border = if active {
                        ink()
                    } else {
                        rgba(173, 149, 111, 255)
                    };
                    b.fill(&self.gui.atlas, px, py, w, h, border);
                    b.fill(
                        &self.gui.atlas,
                        px + 1.,
                        py + 1.,
                        w - 2.,
                        h - 2.,
                        rgba(249, 239, 214, 255),
                    );
                    sprite(
                        self,
                        b,
                        if hot { "mglass_hover" } else { "mglass" },
                        px + 2.,
                        py + (h - 11.) / 2.,
                        11.,
                        11.,
                    );
                    if active {
                        u.draw_edit(self, b, &u.query, px + 15., py + 3., w - 18., fg);
                    } else if u.query.is_empty() {
                        text(
                            self,
                            b,
                            "Search…",
                            px + 15.,
                            py + 3.,
                            rgba(158, 126, 94, 255),
                        );
                    } else {
                        text(
                            self,
                            b,
                            &fit(self, &u.query, w - 18.),
                            px + 15.,
                            py + 3.,
                            fg,
                        );
                    }
                }
                _ => {
                    let shade = if !c.enabled {
                        rgba(213, 208, 194, 255)
                    } else if hot {
                        rgba(222, 194, 143, 255)
                    } else {
                        rgba(236, 218, 177, 255)
                    };
                    b.fill(&self.gui.atlas, px, py, w, h, rgba(173, 149, 111, 255));
                    b.fill(&self.gui.atlas, px + 1., py + 1., w - 2., h - 2., shade);
                    {
                        if matches!(c.action, Action::Name) && u.field == Some(Field::Name) {
                            u.draw_edit(
                                self,
                                b,
                                &c.label,
                                px + 3.,
                                py + (h - 8.) * 0.5,
                                w - 6.,
                                fg,
                            );
                            continue;
                        }
                        let label = fit(self, &c.label, w - 4.);
                        text(
                            self,
                            b,
                            &label,
                            px + (w - self.gui.atlas.font.width(&label) as f32) * 0.5,
                            py + (h - 8.) * 0.5,
                            fg,
                        );
                    }
                }
            }
        }
        if let Some((tip, color)) = tip {
            let ls = lines(self, &tip, 170.);
            let tw = ls
                .iter()
                .map(|s| self.gui.atlas.font.width(s))
                .fold(0.0_f32, f32::max)
                + 8.;
            let th = ls.len() as f32 * 10. + 4.;
            let tx = (x + mouse.0 + 6.).clamp(1., (b.width / k - tw - 1.).max(1.));
            let ty = (y + mouse.1 + 12.).min(b.height / k - th - 1.);
            b.fill(&self.gui.atlas, tx, ty, tw, th, rgba(42, 26, 43, 251));
            for (r, line) in ls.iter().enumerate() {
                text(self, b, line, tx + 4., ty + 3. + r as f32 * 10., color);
            }
        }
    }
    pub(super) fn sc_journal_mouse(&self, button: i32) -> bool {
        if !self.sc_own_journal() {
            return false;
        }
        if button == 0 {
            UI.with_borrow_mut(|u| {
                let (x, y) = point(self);
                if let Some(c) = u
                    .controls(self)
                    .into_iter()
                    .find(|c| c.enabled && c.hit(x, y))
                {
                    u.act(self, c.action);
                }
            });
        }
        true
    }
    pub(super) fn sc_journal_drag(&self) -> bool {
        if !self.sc_own_journal() {
            return false;
        }
        UI.with_borrow_mut(|u| {
            if u.dragging {
                u.scroll_to(point(self).1);
            }
        });
        true
    }
    pub(super) fn sc_journal_scroll(&self, amount: f64) -> bool {
        if !self.sc_own_journal() {
            return false;
        }
        UI.with_borrow_mut(|u| {
            if u.modal.is_none() {
                u.scroll = (u.scroll as isize - amount.signum() as isize)
                    .clamp(0, u.max_scroll() as isize) as usize;
            }
        });
        true
    }
    pub(super) fn sc_journal_char(&self, c: char) -> bool {
        if !self.sc_own_journal() {
            return false;
        }
        if !c.is_control() {
            UI.with_borrow_mut(|u| u.edit(|e| e.insert_text(&c.to_string())));
        }
        true
    }
    pub(super) fn sc_journal_key(&self, key: KeyCode) -> bool {
        if !self.sc_own_journal() {
            return false;
        }
        UI.with_borrow_mut(|u| {
            match key {
                KeyCode::Escape if u.quest.is_some() => u.act(self, Action::QuestChoice("close")),
                KeyCode::Enter | KeyCode::NumpadEnter if u.quest.is_some() => {
                    let choice = if u.quest.as_ref().is_some_and(|q| q.progress.is_some()) { "close" } else { "accept" };
                    u.act(self, Action::QuestChoice(choice));
                }
                KeyCode::ArrowDown | KeyCode::PageDown | KeyCode::End if u.quest.is_some() => {
                    let amount = match key { KeyCode::End => usize::MAX, KeyCode::PageDown => QUEST_ROWS, _ => 1 };
                    u.scroll = u.scroll.saturating_add(amount).min(u.max_scroll());
                }
                KeyCode::ArrowUp | KeyCode::PageUp | KeyCode::Home if u.quest.is_some() => {
                    let amount = match key { KeyCode::Home => usize::MAX, KeyCode::PageUp => QUEST_ROWS, _ => 1 };
                    u.scroll = u.scroll.saturating_sub(amount);
                }
                _ if u.quest.is_some() => {}
                KeyCode::Escape if u.loose => u.act(self, Action::Cancel),
                KeyCode::Enter | KeyCode::NumpadEnter if u.loose => u.act(self, Action::Import),
                _ if u.loose => {}
                KeyCode::Escape if u.modal.is_some() => u.act(self, Action::Cancel),
                KeyCode::Enter | KeyCode::NumpadEnter if u.field == Some(Field::Search) => {
                    u.field = None
                }
                KeyCode::Escape if u.field.is_some() => u.field = None,
                KeyCode::Escape => return false,
                key if u.field.is_some()
                    && !matches!(key, KeyCode::Enter | KeyCode::NumpadEnter) =>
                {
                    u.edit(|e| {
                        e.key(
                            key,
                            self.modifiers.control_key() || self.modifiers.super_key(),
                            self.modifiers.shift_key(),
                        );
                    });
                }
                KeyCode::Enter | KeyCode::NumpadEnter
                    if matches!(
                        u.modal,
                        Some(Modal::Record | Modal::Rename | Modal::Glyph(_))
                    ) && !u.name.trim().is_empty() =>
                {
                    u.act(self, Action::Save)
                }
                KeyCode::ArrowLeft if u.field.is_none() && u.modal.is_none() => {
                    u.act(self, Action::Back)
                }
                KeyCode::ArrowRight if u.field.is_none() && u.modal.is_none() => {
                    u.act(self, Action::Next)
                }
                _ => {}
            }
            true
        })
    }
    fn sc_carries_journal(&self) -> bool {
        (0..self.inventory.len()).chain([OFFHAND_SLOT]).any(|slot| {
            self.stack_in(slot)
                .is_some_and(|s| s.item.as_ref() == sc_travel::BOOK)
        })
    }
    fn sc_journal_bind(&self) -> String {
        self.native_mod_key(OWNER, "travel_journal", "key.keyboard.left.control")
    }
    pub(super) fn sc_hover_item(&self, id: &str) {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative) {
            return;
        }
        let idx = self
            .sc_carries_journal()
            .then(|| sc_handbook::lookup(id))
            .flatten();
        LOOK.with_borrow_mut(|l| {
            l.hover = idx.map(|i| (i, Instant::now()));
            if l.hold.is_some_and(|(old, _)| idx != Some(old)) {
                l.hold = None;
            }
        });
    }
    pub(super) fn sc_lookup_hint(&self, id: &str) -> Option<String> {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative)
            || !self.sc_carries_journal()
            || !sc_travel::handles(sc_travel::BOOK)
        {
            return None;
        }
        sc_handbook::lookup(id)?;
        let key = self.sc_journal_bind();
        if key.is_empty() || key == "key.keyboard.unknown" {
            return None;
        }
        Some(format!(
            "Hold {} for Description",
            sc_handbook::binding_label(&key)
        ))
    }
    pub(super) fn sc_lookup_press(&self, name: &str, repeat: bool) -> bool {
        if !matches!(self.screen, Screen::Inventory | Screen::Creative)
            || name != self.sc_journal_bind()
        {
            return false;
        }
        LOOK.with_borrow_mut(|l| {
            let Some((i, _)) = l
                .hover
                .filter(|(_, t)| t.elapsed() < Duration::from_millis(200))
            else {
                return false;
            };
            if !repeat && l.hold.is_none() {
                l.hold = Some((i, Instant::now()));
            }
            true
        })
    }
    pub(super) fn sc_journal_release(&self, name: &str) {
        if name == self.sc_journal_bind() {
            LOOK.with_borrow_mut(|l| l.hold = None);
        }
        if name == "key.mouse.left" {
            UI.with_borrow_mut(|u| u.dragging = false);
        }
    }
    pub(super) fn sc_journal_frame(&mut self) {
        self.sc_preview_frame();
        self.sc_photo_frame();
        let lookup = LOOK.with_borrow_mut(|l| {
            if !matches!(self.screen, Screen::Inventory | Screen::Creative)
                || !self.focused
                || l.hover
                    .is_none_or(|(_, t)| t.elapsed() > Duration::from_millis(200))
            {
                *l = Lookup::default();
                return None;
            }
            let (idx, t) = l.hold?;
            if !self.sc_carries_journal() {
                *l = Lookup::default();
                return None;
            }
            if t.elapsed() < Duration::from_secs(1) {
                return None;
            }
            *l = Lookup::default();
            Some(idx)
        });
        if let Some(idx) = lookup {
            let book = [self.selected, OFFHAND_SLOT]
                .into_iter()
                .filter_map(|s| self.stack_in(s))
                .filter(|s| super::sc_items::custom(s, sc_travel::BOOK).is_some())
                .find_map(Book::from);
            self.pad_clear_focus();
            if self.screen == Screen::Inventory {
                self.close_container();
            } else {
                self.mod_browser_closed();
                self.carried = None;
            }
            self.screen = Screen::Playing;
            if self.native_mod_screen_open(OWNER) {
                UI.with_borrow_mut(|u| {
                    *u = Journal {
                        book,
                        ..Journal::default()
                    };
                    u.go(Spread::Feature(idx));
                });
            }
        }
        if !self.sc_own_journal() {
            return;
        }
        let (request, photo, builder, sound, dismiss) = UI.with_borrow_mut(|u| {
            (
                u.pending.take(),
                u.photo.take(),
                u.builder.take(),
                std::mem::take(&mut u.sound),
                std::mem::take(&mut u.dismiss),
            )
        });
        if sound {
            self.audio
                .play("minecraft:item.book.page_turn", None, 1., 1.);
        }
        if let Some((id, payload)) = request {
            if let Some(remote) = self.remote.as_mut() {
                remote.session.send_custom_click(&id, payload);
            } else if let Some(action) = id.strip_prefix(sc_travel::CHANNEL) {
                self.sc_travel_action(action, payload.as_ref());
            } else if let Some(action) = id.strip_prefix("strangely_charming:quest/") {
                self.sc_quest_action(action);
            }
        }
        if dismiss {
            self.native_mod_screen_close(OWNER);
        }
        if let Some(request) = builder {
            self.sc_begin_preview(request);
        }
        if let Some(id) = photo {
            self.sc_start_camera(&id);
        }
    }
}
