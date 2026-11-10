use super::{Game, HudBatch, rgba};
use horizon_protocol::packets::{
    configuration::CustomPayload,
    play::{Clientbound, Serverbound},
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

pub(super) const CHANNEL: &str = "strangely_charming:grave_view";
type Pos = [i32; 3];

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) enum MiningRule {
    Decoration,
    Grave(u64),
    Unavailable,
}
impl MiningRule {
    pub(super) fn warning(self, elapsed: Duration, hammer: bool) -> Option<String> {
        match self {
            Self::Decoration => None,
            Self::Unavailable => Some("Grave data is unavailable.".into()),
            Self::Grave(seconds) => {
                let left = seconds.saturating_sub(elapsed.as_secs());
                if left > 0 {
                    Some(format!(
                        "you cannot break this for {} minutes",
                        left.div_ceil(60)
                    ))
                } else if !hammer {
                    Some("you need a runebreaker to break this".into())
                } else {
                    None
                }
            }
        }
    }
}
#[derive(Serialize, Deserialize)]
pub(super) enum View {
    Bearing {
        dimension: String,
        grave: Option<Pos>,
    },
    Mining {
        dimension: String,
        pos: Pos,
        rule: MiningRule,
    },
}
struct Inspection {
    dimension: String,
    pos: Pos,
    sent: Instant,
    rule: Option<(MiningRule, Instant)>,
    clicked: bool,
}
thread_local! {
    static LOCATION: RefCell<Option<(String, String, Pos)>> = const { RefCell::new(None) };
    static GUIDE:RefCell<Option<(String,Pos,Instant)>> = const {RefCell::new(None)};
    static INSPECT:RefCell<Option<Inspection>> = const {RefCell::new(None)};
}
pub(super) fn clear() {
    LOCATION.with_borrow_mut(|s| *s = None);
    GUIDE.with_borrow_mut(|s| *s = None);
    INSPECT.with_borrow_mut(|s| *s = None);
}

impl Game<'_> {
    pub(super) fn sc_send_grave_view(&mut self, who: u128, view: View) {
        if let Ok(data) = serde_json::to_vec(&view) {
            self.native_send(
                who,
                Clientbound::CustomPayload(CustomPayload {
                    channel: CHANNEL.into(),
                    data,
                }),
            );
        }
    }
    pub(super) fn sc_read_grave_view(&mut self, data: &[u8]) {
        if data.len() > 512 {
            return;
        }
        match serde_json::from_slice(data) {
            Ok(View::Bearing { dimension, grave }) if dimension == self.dim.id() => {
                GUIDE.with_borrow_mut(|s| *s = grave.map(|pos| (dimension, pos, Instant::now())));
            }
            Ok(View::Mining {
                dimension,
                pos,
                rule,
            }) if dimension == self.dim.id() => {
                let clicked = INSPECT.with_borrow_mut(|s| {
                    let Some(s) = s
                        .as_mut()
                        .filter(|s| s.pos == pos && s.dimension == dimension)
                    else {
                        return false;
                    };
                    s.rule = Some((rule, Instant::now()));
                    std::mem::take(&mut s.clicked)
                });
                if clicked && let Some(warning) = rule.warning(Duration::ZERO, self.sc_has_hammer())
                {
                    self.sc_notice(warning);
                }
            }
            _ => {}
        }
    }
    pub(super) fn sc_point_to(&self, entry: &super::sc_travel::Entry) {
        LOCATION.replace(Some((entry.name.clone(), entry.dim.clone(), entry.pos)));
    }
    pub(super) fn sc_cancel_waypoint(&self) -> bool {
        self.native_playing() && LOCATION.take().is_some()
    }
    pub(super) fn sc_draw_grave_guide(&self, batch: &mut HudBatch) {
        let Some(p) = self.player.as_ref().filter(|p| !p.health.dead_or_dying()) else {
            return;
        };
        let navigating = LOCATION.with_borrow(|target| {
            let Some((name, dim, pos)) = target else { return false };
            let from = [p.position.x, p.position.y, p.position.z];
            let (distance, [r,g,b]) = super::sc_graves::waypoint_bearing(from, self.yaw, *pos);
            let here = dim == self.dim.id();
            let color = if here { rgba(r,g,b,255) } else { rgba(255,216,102,255) };
            let detail = if here { format!("{distance:.0} blocks") }
                         else { format!("Destination in {}", super::sc_travel::dimension_name(dim)) };
            let atlas = &self.gui.atlas;
            for (line,y,scale,tint) in [
                (format!("Navigating to: {name}"), 8., 1., color),
                ("press Esc to cancel".to_owned(), 19., 0.75, u32::MAX),
                (detail, 27., 0.85, color),
            ] {
                let x = ((batch.width - HudBatch::text_width(atlas,&line)*scale)*0.5).round();
                let start = batch.vertices.len();
                batch.text(atlas,&line,x,y,tint,true);
                batch.scale_since(start,x,y,scale,scale);
            }
            true
        });
        if navigating {return;}
        GUIDE.with_borrow(|s| {
            let Some((dim, grave, stamp)) = s else { return };
            if dim != self.dim.id() || stamp.elapsed() > Duration::from_secs(3) {
                return;
            }
            let from = [p.position.x, p.position.y, p.position.z];
            let Some((line, [r, g, b])) = super::sc_graves::grave_bearing(from, self.yaw, *grave)
            else {
                return;
            };
            let atlas = &self.gui.atlas;
            let x = ((batch.width - HudBatch::text_width(atlas, &line)) * 0.5).round();
            batch.text(atlas, &line, x, 8., rgba(r, g, b, 255), true);
        });
    }
    pub(super) fn sc_inspect_grave(&mut self, pos: Pos) {
        let dim = self.dim.id();
        let request = INSPECT.with_borrow_mut(|state| {
            let now = Instant::now();
            if let Some(s) = state
                .as_mut()
                .filter(|s| s.pos == pos && s.dimension == dim)
            {
                if s.sent.elapsed() < Duration::from_secs(2) {
                    return false;
                }
                s.sent = now;
            } else {
                *state = Some(Inspection {
                    dimension: dim.into(),
                    pos,
                    sent: now,
                    rule: None,
                    clicked: false,
                });
            }
            true
        });
        if request {
            let data = serde_json::to_vec(&(dim, pos)).unwrap();
            self.send(Serverbound::CustomPayload(CustomPayload {
                channel: CHANNEL.into(),
                data,
            }));
        }
    }
    pub(super) fn sc_watch_grave(&mut self) {
        if self.remote.is_none() {
            return;
        }
        if let Some(hit) = self
            .pick()
            .filter(|h| super::sc_graves::is_marker(&self.block_name(h.pos)))
        {
            self.sc_inspect_grave(hit.pos);
        } else {
            INSPECT.with_borrow_mut(|s| *s = None)
        }
    }
    pub(super) fn sc_client_block_grave(&mut self, pos: Pos, start: bool) -> bool {
        self.sc_inspect_grave(pos);
        let hammer = self.sc_has_hammer();
        let (blocked, message) = INSPECT.with_borrow_mut(|s| {
            let s = s.as_mut().unwrap();
            if let Some((rule, t)) = s.rule.filter(|(_, t)| t.elapsed() < Duration::from_secs(5)) {
                let warning = rule.warning(t.elapsed(), hammer);
                (warning.is_some(), if start { warning } else { None })
            } else {
                s.clicked |= start;
                (true, None)
            }
        });
        if let Some(message) = message {
            self.sc_notice(message)
        }
        if blocked {
            self.stop_remote_mining();
            self.mining = None;
        }
        blocked
    }
    pub(super) fn sc_answer_grave_inspection(&mut self, data: &[u8]) {
        if data.len() > 128 {
            return;
        }
        let Ok((dim, pos)) = serde_json::from_slice::<(String, Pos)>(data) else {
            return;
        };
        let Some(p) = self.player.as_ref() else {
            return;
        };
        if self.dim.id() != dim || p.health.dead_or_dying() {
            return;
        }
        let distance = (p.position.x - f64::from(pos[0]) - 0.5)
            .hypot(p.position.z - f64::from(pos[2]) - 0.5)
            .hypot(p.position.y + 1.5 - f64::from(pos[1]) - 0.5);
        if distance > self.reach() + 2. || !super::sc_graves::is_marker(&self.block_name(pos)) {
            return;
        }
        let rule = self.sc_grave_rule(pos).unwrap_or(MiningRule::Unavailable);
        let who = self.native_acting_id();
        self.sc_send_grave_view(
            who,
            View::Mining {
                dimension: dim,
                pos,
                rule,
            },
        );
    }
}
