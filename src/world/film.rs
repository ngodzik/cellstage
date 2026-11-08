//! A film the agent directs in a world: shots, each a place the camera
//! goes to and stays at for a while, words said over it, and how far the
//! world's clock runs meanwhile. The camera travels between places along
//! the path van Wijk and Nuij showed to look smoothest, which pulls back
//! before a long move and comes in again, and keeps an even speed of
//! zoom across scales: from a centimetre of skin to a nanometre of DNA is
//! one move.

use bevy::prelude::*;
use serde_json::Value;

/// Where the camera is: the point it looks at, how far it is from it, and
/// from which way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eye {
    pub focus: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Eye {
    pub fn transform(&self) -> Transform {
        let around = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        Transform::from_translation(self.focus + around * self.distance)
            .looking_at(self.focus, Vec3::Y)
    }

    /// Turn by a drag of `delta` points.
    pub fn turn(&mut self, delta: Vec2) {
        self.yaw -= delta.x * 0.005;
        self.pitch = (self.pitch + delta.y * 0.005).clamp(-1.45, 1.45);
    }

    /// Come closer by `lines` of the wheel, by the same fraction at any
    /// scale.
    pub fn zoom(&mut self, lines: f32) {
        self.distance = (self.distance * (-0.12 * lines).exp()).clamp(0.004, 60_000.0);
    }
}

/// A named place to look from, with what it shows.
#[derive(Debug, Clone)]
pub struct Place {
    pub name: &'static str,
    pub what: &'static str,
    pub eye: Eye,
}

/// One shot of a film.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    pub eye: Eye,
    /// How long the shot lasts, travel included.
    pub seconds: f32,
    /// How long the camera takes to get there, at most.
    pub travel: f32,
    /// What is said over it.
    pub say: String,
    /// The world's clock at the end of the shot, if it moves.
    pub clock: Option<f32>,
    /// How fast the camera turns about its focus while it stays, in
    /// radians a second.
    pub drift: f32,
    /// What it shows above the rest, which dims; nothing for all alike.
    pub highlight: Vec<String>,
}

/// A film: its shots, and the clock it starts from.
#[derive(Debug, Clone, Default)]
pub struct Film {
    pub shots: Vec<Shot>,
    pub start: f32,
    /// Where each shot leaves the camera.
    ends: Vec<Eye>,
}

/// What the film shows at a moment.
#[derive(Debug, Clone)]
pub struct Frame {
    pub eye: Eye,
    pub clock: f32,
    /// What is said, and how visible the words are, 0 to 1.
    pub say: Option<(String, f32)>,
    /// What the shot sets apart.
    pub highlight: Vec<String>,
}

fn smoother(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// The shorter way from one angle to another.
fn turn_to(from: f32, to: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let d = (to - from).rem_euclid(tau);
    if d > std::f32::consts::PI { d - tau } else { d }
}

/// A path from one view to another, as a function of how far along.
pub type ZoomPath = Box<dyn Fn(f32) -> (Vec3, f32)>;

/// Van Wijk and Nuij's smooth zoom and pan between two views, each a
/// point and a width seen, for `t` from 0 to 1, and how long the path is
/// in their units, to time it by.
pub fn zoom_path(a: (Vec3, f32), b: (Vec3, f32)) -> (f32, ZoomPath) {
    const RHO: f64 = std::f64::consts::SQRT_2;
    let (c0, w0) = (a.0, a.1 as f64);
    let (c1, w1) = (b.0, b.1 as f64);
    let delta = c1 - c0;
    let d1 = delta.length() as f64;
    let rho2 = RHO * RHO;
    let rho4 = rho2 * rho2;
    if d1 < 1e-9 * w0.max(w1) {
        let s = (w1 / w0).ln() / RHO;
        let path = move |t: f32| (c0 + delta * t, (w0 * (RHO * t as f64 * s).exp()) as f32);
        return (s.abs() as f32, Box::new(path));
    }
    let b0 = (w1 * w1 - w0 * w0 + rho4 * d1 * d1) / (2.0 * w0 * rho2 * d1);
    let b1 = (w1 * w1 - w0 * w0 - rho4 * d1 * d1) / (2.0 * w1 * rho2 * d1);
    let r0 = ((b0 * b0 + 1.0).sqrt() - b0).ln();
    let r1 = ((b1 * b1 + 1.0).sqrt() - b1).ln();
    let s = (r1 - r0) / RHO;
    let path = move |t: f32| {
        let at = t as f64 * s;
        let u = w0 / (rho2 * d1) * (r0.cosh() * (RHO * at + r0).tanh() - r0.sinh());
        let w = w0 * r0.cosh() / (RHO * at + r0).cosh();
        (c0 + delta * u as f32, w as f32)
    };
    (s.abs() as f32, Box::new(path))
}

/// The camera between two eyes, `t` of the way.
pub fn between(a: &Eye, b: &Eye, t: f32) -> Eye {
    let e = smoother(t);
    let (_, path) = zoom_path((a.focus, a.distance), (b.focus, b.distance));
    let (focus, distance) = path(e);
    Eye {
        focus,
        distance: distance.max(1e-4),
        yaw: a.yaw + turn_to(a.yaw, b.yaw) * e,
        pitch: a.pitch + (b.pitch - a.pitch) * e,
    }
}

/// How long a move between two eyes should take, in seconds, by how far
/// it goes across scales.
pub fn travel_time(a: &Eye, b: &Eye) -> f32 {
    let (s, _) = zoom_path((a.focus, a.distance), (b.focus, b.distance));
    let turn = turn_to(a.yaw, b.yaw).abs() + (b.pitch - a.pitch).abs();
    (s * 0.75 + turn * 0.5).clamp(0.0, 9.0)
}

impl Film {
    /// A film of `shots`, the world's clock at `clock` when it starts.
    pub fn new(shots: Vec<Shot>, clock: f32) -> Self {
        let ends = shots
            .iter()
            .map(|shot| {
                let mut end = shot.eye;
                let travel = shot.travel.min(shot.seconds);
                end.yaw += shot.drift * (shot.seconds - travel).max(0.0);
                end
            })
            .collect();
        Self {
            shots,
            start: clock,
            ends,
        }
    }

    pub fn length(&self) -> f32 {
        self.shots.iter().map(|s| s.seconds).sum()
    }

    /// What the film shows `clock` seconds in, starting from `from`.
    pub fn frame(&self, clock: f32, from: &Eye) -> Frame {
        let mut start = 0.0;
        let mut previous = *from;
        let mut years = self.start;
        for (i, shot) in self.shots.iter().enumerate() {
            let end = start + shot.seconds;
            if clock < end || i + 1 == self.shots.len() {
                let local = (clock - start).clamp(0.0, shot.seconds);
                let travel = shot.travel.min(shot.seconds);
                let eye = if local < travel {
                    between(&previous, &shot.eye, local / travel.max(1e-3))
                } else {
                    let mut e = shot.eye;
                    e.yaw += shot.drift * (local - travel);
                    e
                };
                let years = match shot.clock {
                    Some(to) => {
                        let span = (shot.seconds - travel).max(1e-3);
                        let f = ((local - travel) / span).clamp(0.0, 1.0);
                        years + (to - years) * smoother(f)
                    }
                    None => years,
                };
                let say = (!shot.say.is_empty()).then(|| {
                    let show = ((local - 0.2) / 0.5).clamp(0.0, 1.0)
                        * ((shot.seconds - local) / 0.4).clamp(0.0, 1.0);
                    (shot.say.clone(), show)
                });
                return Frame {
                    eye,
                    clock: years,
                    say,
                    highlight: shot.highlight.clone(),
                };
            }
            if let Some(to) = shot.clock {
                years = to;
            }
            previous = self.ends[i];
            start = end;
        }
        Frame {
            eye: *from,
            clock: years,
            say: None,
            highlight: Vec::new(),
        }
    }
}

/// A list of names, or one name, as the agent gave it.
pub fn names(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

/// Read the shots the agent wrote: each names a place or gives where to
/// look, how long it lasts, what is said, and the clock at its end.
pub fn read_shots(
    list: &[Value],
    find: impl Fn(&str) -> Option<Eye>,
    mut from: Eye,
) -> Result<Vec<Shot>, String> {
    let mut shots = Vec::new();
    for (i, s) in list.iter().enumerate() {
        let n = i + 1;
        let number = |k: &str| s.get(k).and_then(Value::as_f64).map(|v| v as f32);
        let mut eye = match s.get("place").and_then(Value::as_str) {
            Some(name) => find(name).ok_or_else(|| format!("shot {n}: no place {name}"))?,
            None => from,
        };
        if let Some(d) = number("distance") {
            if !(0.001..=80_000.0).contains(&d) {
                return Err(format!(
                    "shot {n}: distance is in micrometres, 0.001 to 80000"
                ));
            }
            eye.distance = d;
        }
        if let Some(y) = number("turn") {
            eye.yaw += y.to_radians();
        }
        if let Some(p) = number("tilt") {
            eye.pitch = p.to_radians().clamp(-1.45, 1.45);
        }
        let travel_needed = travel_time(&from, &eye);
        let seconds = number("seconds")
            .unwrap_or(travel_needed + 4.0)
            .clamp(0.5, 120.0);
        let travel = number("travel")
            .unwrap_or(travel_needed.max(if eye == from { 0.0 } else { 1.5 }))
            .clamp(0.0, seconds);
        shots.push(Shot {
            eye,
            seconds,
            travel,
            say: s
                .get("say")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            clock: number("clock").or(number("years")),
            drift: number("drift").unwrap_or(3.0).to_radians(),
            highlight: names(s.get("highlight")),
        });
        from = eye;
    }
    Ok(shots)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eye(focus: Vec3, distance: f32) -> Eye {
        Eye {
            focus,
            distance,
            yaw: 0.3,
            pitch: 0.5,
        }
    }

    #[test]
    fn a_zoom_across_six_scales_starts_and_ends_where_it_should_and_never_jumps() {
        let a = eye(Vec3::new(0.0, 95.0, 0.0), 20_000.0);
        let b = eye(Vec3::ZERO, 0.01);
        let mut last = between(&a, &b, 0.0);
        assert!((last.distance - 20_000.0).abs() < 1.0);
        for k in 1..=400 {
            let e = between(&a, &b, k as f32 / 400.0);
            // The distance changes by a bounded factor per step: an even
            // speed of zoom, no leap.
            let ratio = (e.distance / last.distance).ln().abs();
            assert!(ratio < 0.2, "step {k}: {ratio}");
            last = e;
        }
        assert!((last.distance - 0.01).abs() < 1e-4);
        assert!(last.focus.length() < 1e-3);
    }

    #[test]
    fn a_long_move_at_one_scale_pulls_back_on_the_way() {
        let a = eye(Vec3::ZERO, 10.0);
        let b = eye(Vec3::new(500.0, 0.0, 0.0), 10.0);
        let mid = between(&a, &b, 0.5);
        assert!(mid.distance > 50.0);
    }

    #[test]
    fn a_film_runs_the_worlds_clock_during_the_shot_that_asks_for_it() {
        let a = eye(Vec3::ZERO, 100.0);
        let shots = vec![
            Shot {
                eye: a,
                seconds: 4.0,
                travel: 0.0,
                say: "one".into(),
                clock: None,
                drift: 0.0,
                highlight: Vec::new(),
            },
            Shot {
                eye: eye(Vec3::X * 10.0, 50.0),
                seconds: 10.0,
                travel: 2.0,
                say: String::new(),
                clock: Some(8.0),
                drift: 0.0,
                highlight: Vec::new(),
            },
        ];
        let film = Film::new(shots, -1.0);
        assert_eq!(film.length(), 14.0);
        assert_eq!(film.frame(1.0, &a).clock, -1.0);
        assert!(film.frame(1.0, &a).say.is_some());
        assert_eq!(film.frame(5.0, &a).clock, -1.0);
        let late = film.frame(14.0, &a);
        assert!((late.clock - 8.0).abs() < 1e-4);
        assert!((late.eye.distance - 50.0).abs() < 1e-3);
    }
}
