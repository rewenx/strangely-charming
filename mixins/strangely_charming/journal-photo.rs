// Photographs only live on the device that took them. A page carries the entry
// UUID, never a path or image data supplied by another player.
use super::{Game, HudBatch, Screen, rgba, sc_travel::valid_id};
use std::{
    cell::RefCell,
    fs,
    io::BufReader,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
struct Thumb {
    rgb: Vec<[u8; 3]>,
    w: usize,
    h: usize,
}
fn photo_path(id: &str) -> PathBuf {
    horizon_client::logging::player_dir()
        .join("screenshots/strangely_charming")
        .join(format!("{id}.png"))
}
fn thumbnail(path: &Path, w: usize, h: usize) -> Option<Thumb> {
    let mut dec = png::Decoder::new(BufReader::new(fs::File::open(path).ok()?));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut r = dec.read_info().ok()?;
    let n = r.output_buffer_size()?;
    if n > 128 * 1024 * 1024 {
        return None;
    }
    let mut pixels = vec![0; n];
    let info = r.next_frame(&mut pixels).ok()?;
    let ch = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return None,
    };
    let (ow, oh) = (info.width as usize, info.height as usize);
    let (cw, chh) = if ow * h > oh * w {
        ((oh * w / h).max(1), oh)
    } else {
        (ow, (ow * h / w).max(1))
    };
    let (ox, oy) = ((ow - cw) / 2, (oh - chh) / 2);
    let mut rgb = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let i = ((oy + y * chh / h) * ow + ox + x * cw / w) * ch;
            rgb.push([pixels[i], pixels[i + 1], pixels[i + 2]]);
        }
    }
    Some(Thumb { rgb, w, h })
}
#[derive(Default)]
struct Cache {
    id: String,
    h: usize,
    thumb: Option<Thumb>,
}
thread_local! {
    static CACHE:RefCell<Cache>=RefCell::default();
    static CAMERA:RefCell<Option<Camera>>=const{RefCell::new(None)};
}
pub(super) fn has_photo(id: &str) -> bool {
    CACHE.with_borrow(|c| {
        if c.id == id {
            c.thumb.is_some()
        } else {
            valid_id(id) && photo_path(id).is_file()
        }
    })
}
enum Phase {
    Aim,
    Requested,
    Cancelled,
    Capture,
    Shutter,
}
struct Camera {
    id: String,
    temp: PathBuf,
    started: Instant,
    phase: Phase,
    previous_hud: bool,
}
impl Game<'_> {
    fn sc_thumb(&self, b: &mut HudBatch, id: &str, x: f32, y: f32, h: usize) -> bool {
        if !valid_id(id) {
            return false;
        }
        CACHE.with_borrow_mut(|c| {
            if c.id != id || c.h != h {
                *c = Cache {
                    id: id.into(),
                    h,
                    thumb: thumbnail(&photo_path(id), 96, h),
                };
            }
            let Some(t) = &c.thumb else { return false };
            for (r, row) in t.rgb.chunks_exact(t.w).take(t.h).enumerate() {
                let mut col = 0;
                while col < t.w {
                    let pixel = row[col];
                    let mut end = col + 1;
                    while end < t.w && row[end] == pixel {
                        end += 1;
                    }
                    b.fill(
                        &self.gui.atlas,
                        x + col as f32,
                        y + r as f32,
                        (end - col) as f32,
                        1.,
                        rgba(pixel[0], pixel[1], pixel[2], 255),
                    );
                    col = end;
                }
            }
            true
        })
    }
    pub(super) fn sc_journal_thumbnail(&self, b: &mut HudBatch, id: &str, x: f32, y: f32) -> bool {
        self.sc_thumb(b, id, x, y, 74)
    }
    pub(super) fn sc_page_thumbnail(&self, b: &mut HudBatch, id: &str, x: f32, y: f32) -> bool {
        self.sc_thumb(b, id, x, y, 54)
    }
    pub(super) fn sc_start_camera(&mut self, id: &str) {
        if !valid_id(id)
            || !self
                .sc_held_entry(id)
                .is_some_and(|e| self.sc_near_entry(&e))
        {
            self.sc_notice("Return to the recorded location to take a photo.");
            return;
        }
        if self.screenshot_sequence.is_some()
            || self
                .renderer
                .as_ref()
                .is_none_or(|r| r.screenshot.is_some())
        {
            self.sc_notice("The camera is busy.");
            return;
        }
        let path = photo_path(id);
        if fs::create_dir_all(path.parent().unwrap()).is_err() {
            self.sc_notice("The photo folder could not be opened.");
            return;
        }
        let mut salt = [0; 8];
        if horizon_protocol::crypto::random_bytes(&mut salt).is_err() {
            return;
        }
        let temp = path.with_file_name(format!(".{id}-{:016x}.png", u64::from_be_bytes(salt)));
        let c = Camera {
            id: id.into(),
            temp,
            started: Instant::now(),
            phase: Phase::Aim,
            previous_hud: self.ui.hide_gui,
        };
        self.native_mod_screen_close("strangely_charming");
        self.ui.hide_gui = true;
        CAMERA.with_borrow_mut(|v| *v = Some(c));
    }
    pub(super) fn sc_camera_button(&self, primary: bool, pressed: bool) -> bool {
        CAMERA.with_borrow_mut(|camera| {
            let Some(c) = camera else { return false };
            if primary && pressed && matches!(c.phase, Phase::Aim) {
                c.phase = Phase::Requested;
                c.started = Instant::now();
            }
            true
        })
    }
    pub(super) fn sc_camera_cancel(&self) -> bool {
        CAMERA.with_borrow_mut(|camera| {
            let Some(c) = camera else { return false };
            c.phase = Phase::Cancelled;
            c.started = Instant::now();
            true
        })
    }
    pub(super) fn sc_photo_frame(&mut self) {
        let Some(mut c) = CAMERA.with_borrow_mut(Option::take) else {
            return;
        };
        let valid = self.renderer.is_some()
            && self.title.is_none()
            && self.player.is_some()
            && self.screen == Screen::Playing
            && self.screenshot_sequence.is_none()
            && self
                .sc_held_entry(&c.id)
                .is_some_and(|e| self.sc_near_entry(&e));
        let dt = c.started.elapsed();
        let mut finished = false;
        if valid && (matches!(c.phase, Phase::Aim) || dt < Duration::from_secs(8)) {
            self.ui.hide_gui = true;
            match c.phase {
                Phase::Requested => {
                    if let Some(r) = self.renderer.as_mut().filter(|r| r.screenshot.is_none()) {
                        r.screenshot = Some(horizon_client::gfx::ScreenshotRequest {
                            path: c.temp.clone(),
                            size: None,
                            scaled: true,
                        });
                        c.phase = Phase::Capture;
                        c.started = Instant::now();
                    }
                }
                Phase::Capture if c.temp.is_file() => {
                    c.phase = Phase::Shutter;
                    c.started = Instant::now();
                    self.audio
                        .play("minecraft:block.tripwire.click_on", None, 0.6, 1.7);
                }
                Phase::Shutter if dt >= Duration::from_millis(200) => {
                    if fs::rename(&c.temp, photo_path(&c.id)).is_ok() {
                        CACHE.with_borrow_mut(|v| *v = Cache::default());
                    } else {
                        self.sc_notice("The photograph could not be saved.");
                    }
                    finished = true;
                }
                Phase::Cancelled => finished = true,
                _ => {}
            }
            if !finished {
                CAMERA.with_borrow_mut(|v| *v = Some(c));
                return;
            }
        } else if valid {
            self.sc_notice("The camera timed out.");
        }
        self.ui.hide_gui = c.previous_hud;
        if let Some(r) = self.renderer.as_mut()
            && r.screenshot.as_ref().is_some_and(|s| s.path == c.temp)
        {
            r.screenshot = None;
        }
        let _ = fs::remove_file(&c.temp);
        if valid {
            self.native_mod_screen_open("strangely_charming");
        }
    }
    pub(super) fn sc_camera_overlay(&self, b: &mut HudBatch) {
        CAMERA.with_borrow(|v| {
            let Some(c) = v else { return };
            let a = &self.gui.atlas;
            let (cx, cy) = (b.width / 2., b.height / 2.);
            match c.phase {
                Phase::Aim => {
                    let hint = "Click to take photo   •   Esc to cancel";
                    b.text(
                        a,
                        hint,
                        cx - a.font.width(hint) / 2.,
                        b.height - 19.,
                        u32::MAX,
                        true,
                    );
                    let h = (b.height * 0.38).min(b.width * 0.36 * 74. / 96.);
                    let w = h * 96. / 74.;
                    for sx in [-1., 1.] {
                        for sy in [-1., 1.] {
                            let (x, y) = (cx + sx * w, cy + sy * h);
                            b.fill(a, x - if sx > 0. { 12. } else { 0. }, y, 12., 1., u32::MAX);
                            b.fill(a, x, y - if sy > 0. { 12. } else { 0. }, 1., 12., u32::MAX);
                        }
                    }
                    b.fill(a, cx - 3., cy, 7., 1., u32::MAX);
                    b.fill(a, cx, cy - 3., 1., 7., u32::MAX);
                }
                Phase::Shutter => {
                    let f = (c.started.elapsed().as_secs_f32() / 0.2).min(1.);
                    let closed = 1. - (f * 2. - 1.).abs();
                    let h = (b.height / 2. + 1.) * closed;
                    b.fill(a, 0., 0., b.width, h, rgba(26, 22, 29, 255));
                    b.fill(a, 0., b.height - h, b.width, h, rgba(26, 22, 29, 255));
                }
                Phase::Requested | Phase::Capture | Phase::Cancelled => {}
            }
        });
    }
}
