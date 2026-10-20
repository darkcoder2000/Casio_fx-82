//! egui front end: draws the calculator in a fixed design space and scales it to the window.

use eframe::egui::{
    self, epaint::PathStroke, pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Painter,
    Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
};

use crate::calc::{Calc, Key, Lcd};
use crate::eval::Angle;

const W: f32 = 360.0;
const H: f32 = 640.0;

const GOLD: Color32 = Color32::from_rgb(0xe3, 0xa8, 0x3c);
const INK: Color32 = Color32::from_rgb(0x1d, 0x22, 0x1a);
const LCD_BG: Color32 = Color32::from_rgb(0xc4, 0xcd, 0xb9);

#[derive(Clone, Copy)]
enum Seg {
    T(&'static str),
    Sup(&'static str),
    Sub(&'static str),
    Root,
    Over(&'static str),
    Tri,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Dark,
    Digit,
    Blue,
}

struct KeySpec {
    key: Key,
    label: &'static [Seg],
    shift: &'static [Seg],
    style: Style,
    /// Extra legend printed below the key (DATA / DEL).
    below: &'static [Seg],
}

use Seg::*;

const fn k(key: Key, label: &'static [Seg], shift: &'static [Seg]) -> KeySpec {
    KeySpec { key, label, shift, style: Style::Dark, below: &[] }
}

const fn kd(d: u8, shift: &'static [Seg]) -> KeySpec {
    const DIGITS: [&[Seg]; 10] = [
        &[T("0")],
        &[T("1")],
        &[T("2")],
        &[T("3")],
        &[T("4")],
        &[T("5")],
        &[T("6")],
        &[T("7")],
        &[T("8")],
        &[T("9")],
    ];
    KeySpec { key: Key::Digit(d), label: DIGITS[d as usize], shift, style: Style::Digit, below: &[] }
}

const fn kb(key: Key, label: &'static [Seg], shift: &'static [Seg]) -> KeySpec {
    KeySpec { key, label, shift, style: Style::Blue, below: &[] }
}

const SMALL_ROWS: [[KeySpec; 6]; 3] = [
    [
        k(Key::Shift, &[T("SHIFT")], &[]),
        k(Key::Mode, &[T("MODE")], &[]),
        k(Key::Square, &[T("x"), Sup("2")], &[Root]),
        k(Key::Log, &[T("log")], &[T("10"), Sup("x")]),
        k(Key::Ln, &[T("ln")], &[T("e"), Sup("x")]),
        k(Key::On, &[T("ON")], &[]),
    ],
    [
        k(Key::AbC, &[T("a b/c")], &[T("d/c")]),
        k(Key::Dms, &[T("° ' \"")], &[Left]),
        k(Key::Hyp, &[T("hyp")], &[]),
        k(Key::Sin, &[T("sin")], &[T("sin"), Sup("-1")]),
        k(Key::Cos, &[T("cos")], &[T("cos"), Sup("-1")]),
        k(Key::Tan, &[T("tan")], &[T("tan"), Sup("-1")]),
    ],
    [
        k(Key::Neg, &[T("+/-")], &[Sup("3"), Root]),
        k(Key::Drg, &[Tri], &[T("x"), Sup("3")]),
        k(Key::LParen, &[T("(")], &[T("X↔Y")]),
        k(Key::RParen, &[T(")")], &[T("1/x")]),
        k(Key::Pow, &[T("x"), Sup("y")], &[Sup("x"), Root, T("y")]),
        k(Key::Mr, &[T("MR")], &[T("Min")]),
    ],
];

const BIG_ROWS: [[KeySpec; 5]; 4] = [
    [
        kd(7, &[Over("x")]),
        kd(8, &[T("σ"), Sub("n")]),
        kd(9, &[T("σ"), Sub("n-1")]),
        kb(Key::C, &[T("C")], &[T("X↔M")]),
        kb(Key::Ac, &[T("AC")], &[T("SAC")]),
    ],
    [
        kd(4, &[T("Σx"), Sup("2")]),
        kd(5, &[T("Σx")]),
        kd(6, &[T("n")]),
        k(Key::Mul, &[T("×")], &[Left, T("ENG")]),
        k(Key::Div, &[T("÷")], &[T("ENG")]),
    ],
    [
        kd(1, &[T("nPr")]),
        kd(2, &[T("nCr")]),
        kd(3, &[T("x!")]),
        k(Key::Add, &[T("+")], &[T("R"), Right, T("P")]),
        k(Key::Sub, &[T("-")], &[T("P"), Right, T("R")]),
    ],
    [
        kd(0, &[T("RND")]),
        KeySpec { key: Key::Dot, label: &[T("•")], shift: &[T("RAN#")], style: Style::Digit, below: &[] },
        k(Key::Exp, &[T("EXP")], &[T("π")]),
        k(Key::Eq, &[T("=")], &[T("%")]),
        KeySpec {
            key: Key::MPlus,
            label: &[T("M+")],
            shift: &[T("M-")],
            style: Style::Dark,
            below: &[T("DATA  DEL")],
        },
    ],
];

pub struct App {
    calc: Calc,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let calc = Calc::new();
        App { calc }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.calc.tick();
        self.handle_keyboard(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(0x2b, 0x2e, 0x35)))
            .show(ctx, |ui| {
                let avail = ui.available_rect_before_wrap();
                let s = (avail.width() / W).min(avail.height() / H);
                let origin = avail.center() - vec2(W * s, H * s) / 2.0;
                let lcd = self.calc.lcd();
                let painter = ui.painter().clone();
                let tr = Tr { origin, s };
                draw_body(&painter, &tr);
                draw_lcd(&painter, &tr, &lcd);
                self.draw_keys(ui, &painter, &tr);
            });
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}

/// Design → screen transform.
struct Tr {
    origin: Pos2,
    s: f32,
}

impl Tr {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.s
    }
    fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), vec2(w, h) * self.s)
    }
    fn r(&self, v: f32) -> u8 {
        (v * self.s).round().clamp(0.0, 255.0) as u8
    }
}

fn draw_body(p: &Painter, t: &Tr) {
    p.rect_filled(t.rect(0.0, 0.0, W, H), CornerRadius::same(t.r(26.0)), Color32::from_rgb(0x1c, 0x1c, 0x1f));
    // solar cell
    let solar = t.rect(30.0, 26.0, 300.0, 34.0);
    p.rect_filled(solar, CornerRadius::same(t.r(3.0)), Color32::from_rgb(0x3b, 0x2c, 0x28));
    for i in 1..8 {
        let x = 30.0 + i as f32 * 300.0 / 8.0;
        p.line_segment(
            [t.p(x, 27.0), t.p(x, 59.0)],
            Stroke::new(1.0 * t.s, Color32::from_rgb(0x55, 0x44, 0x3e)),
        );
    }
    p.text(
        t.p(34.0, 78.0),
        Align2::LEFT_CENTER,
        "CASIO",
        FontId::proportional(15.0 * t.s),
        Color32::WHITE,
    );
    p.text(
        t.p(98.0, 80.0),
        Align2::LEFT_CENTER,
        "fx-82 SOLAR II",
        FontId::proportional(10.0 * t.s),
        Color32::from_rgb(0xdd, 0xdd, 0xdd),
    );
}

fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn draw_lcd(p: &Painter, t: &Tr, lcd: &Lcd) {
    p.rect_filled(t.rect(34.0, 92.0, 292.0, 84.0), CornerRadius::same(t.r(6.0)), Color32::from_rgb(0x30, 0x30, 0x32));
    p.rect_filled(t.rect(38.0, 96.0, 284.0, 76.0), CornerRadius::same(t.r(3.0)), LCD_BG);
    if lcd.off {
        return;
    }
    let on = |lit: bool| if lit { INK } else { with_alpha(INK, 28) };
    let ind = &lcd.ind;
    let font = FontId::proportional(8.5 * t.s);
    let tag = |x: f32, txt: &str, lit: bool| {
        p.text(t.p(x, 104.0), Align2::CENTER_CENTER, txt, font.clone(), on(lit));
    };
    tag(56.0, "SHIFT", ind.shift);
    tag(128.0, "M", ind.m);
    tag(142.0, "K", ind.k);
    tag(158.0, "HYP", ind.hyp);
    tag(186.0, "DEG", ind.angle == Some(Angle::Deg));
    tag(208.0, "RAD", ind.angle == Some(Angle::Rad));
    tag(230.0, "GRA", ind.angle == Some(Angle::Gra));
    tag(256.0, "FIX", ind.fix);
    tag(282.0, "SCI", ind.sci);
    tag(304.0, "SD", ind.sd);

    if let Some(text) = &lcd.text {
        p.text(
            t.p(46.0, 143.0),
            Align2::LEFT_CENTER,
            text,
            FontId::monospace(19.0 * t.s),
            INK,
        );
        return;
    }

    // digit field: sign cell + 10 digit cells
    let (cw, ch, pitch) = (17.0, 34.0, 21.0);
    let top = 118.0;
    let x0 = 60.0;
    if lcd.neg {
        seg_line(p, t, (44.0, top + ch / 2.0), (54.0, top + ch / 2.0), 3.0, INK);
    }
    let cells: Vec<char> = lcd.cells.chars().collect();
    let mut slots: Vec<(char, bool)> = Vec::new(); // (char, has dot after)
    for &c in &cells {
        if c == '.' {
            match slots.last_mut() {
                Some(l) => l.1 = true,
                None => slots.push((' ', true)),
            }
        } else {
            slots.push((c, false));
        }
    }
    let first = 10usize.saturating_sub(slots.len());
    for (i, (c, dot)) in slots.iter().enumerate() {
        let x = x0 + (first + i) as f32 * pitch;
        draw_cell(p, t, *c, x, top, cw, ch, 3.0, INK);
        if *dot {
            p.circle_filled(t.p(x + cw + 2.0, top + ch), 1.9 * t.s, INK);
        }
    }
    if let Some(e) = &lcd.exp {
        let (sign, digits) = match e.strip_prefix('-') {
            Some(d) => (true, d),
            None => (false, e.as_str()),
        };
        if sign {
            seg_line(p, t, (277.0, top + 9.0), (283.0, top + 9.0), 1.8, INK);
        }
        for (i, c) in digits.chars().enumerate() {
            draw_cell(p, t, c, 286.0 + i as f32 * 14.0, top, 10.0, 17.0, 1.8, INK);
        }
    }
}

fn seg_line(p: &Painter, t: &Tr, a: (f32, f32), b: (f32, f32), w: f32, c: Color32) {
    p.line_segment([t.p(a.0, a.1), t.p(b.0, b.1)], Stroke::new(w * t.s, c));
}

/// One 7-segment style cell (slightly slanted like the real LCD).
fn draw_cell(p: &Painter, t: &Tr, c: char, x: f32, y: f32, w: f32, h: f32, th: f32, col: Color32) {
    let shear = 0.10;
    let pt = |px: f32, py: f32| -> (f32, f32) { (x + px + (h - py) * shear, y + py) };
    let mid = h / 2.0;
    let m = th * 0.55; // inset
    let line = |a: (f32, f32), b: (f32, f32)| {
        let (a, b) = (pt(a.0, a.1), pt(b.0, b.1));
        seg_line(p, t, a, b, th, col);
    };
    // segments: a b c d e f g
    let segs: [((f32, f32), (f32, f32)); 7] = [
        ((m, 0.0 + th / 2.0), (w - m, th / 2.0)),                 // a
        ((w - th / 2.0, m + 1.0), (w - th / 2.0, mid - m)),        // b
        ((w - th / 2.0, mid + m), (w - th / 2.0, h - m - 1.0)),    // c
        ((m, h - th / 2.0), (w - m, h - th / 2.0)),                // d
        ((th / 2.0, mid + m), (th / 2.0, h - m - 1.0)),            // e
        ((th / 2.0, m + 1.0), (th / 2.0, mid - m)),                // f
        ((m, mid), (w - m, mid)),                                  // g
    ];
    let mask: u8 = match c {
        '0' => 0b0111111,
        '1' => 0b0000110,
        '2' => 0b1011011,
        '3' => 0b1001111,
        '4' => 0b1100110,
        '5' => 0b1101101,
        '6' => 0b1111101,
        '7' => 0b0000111,
        '8' => 0b1111111,
        '9' => 0b1101111,
        _ => 0,
    };
    if mask != 0 {
        for (i, (a, b)) in segs.iter().enumerate() {
            if mask & (1 << i) != 0 {
                line(*a, *b);
            }
        }
        return;
    }
    match c {
        'J' => {
            // fraction separator: ┘
            line((w * 0.1, h - th / 2.0), (w - th / 2.0, h - th / 2.0));
            line((w - th / 2.0, h - th / 2.0), (w - th / 2.0, mid - 2.0));
        }
        '°' => {
            let (cx, cy) = pt(w / 2.0, th * 1.4);
            p.circle_stroke(t.p(cx, cy), 3.2 * t.s, Stroke::new(1.6 * t.s, col));
        }
        _ => {}
    }
}

impl App {
    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            match ev {
                egui::Event::Text(txt) => {
                    for ch in txt.chars() {
                        for &key in char_to_keys(ch) {
                            self.calc.press(key);
                        }
                    }
                }
                egui::Event::Key { key, pressed: true, .. } => {
                    let keys: &[Key] = match key {
                        egui::Key::Enter => &[Key::Eq],
                        egui::Key::Backspace => &[Key::Shift, Key::Dms],
                        egui::Key::Escape | egui::Key::Delete => &[Key::Ac],
                        _ => &[],
                    };
                    for &k in keys {
                        self.calc.press(k);
                    }
                }
                _ => {}
            }
        }
    }

    fn draw_keys(&mut self, ui: &mut egui::Ui, p: &Painter, t: &Tr) {
        // small keys: 6 columns
        for (r, row) in SMALL_ROWS.iter().enumerate() {
            let top = 206.0 + r as f32 * 48.0;
            for (c, spec) in row.iter().enumerate() {
                let x = 34.0 + c as f32 * (40.0 + 10.4);
                self.draw_key(ui, p, t, spec, Rect::from_min_size(pos2(x, top), vec2(40.0, 26.0)), 12.0, 11.5);
            }
        }
        for (r, row) in BIG_ROWS.iter().enumerate() {
            let top = 358.0 + r as f32 * 66.0;
            for (c, spec) in row.iter().enumerate() {
                let x = 34.0 + c as f32 * (48.0 + 13.0);
                self.draw_key(ui, p, t, spec, Rect::from_min_size(pos2(x, top), vec2(48.0, 36.0)), 13.0, 15.0);
            }
        }
    }

    fn draw_key(&mut self, ui: &mut egui::Ui, p: &Painter, t: &Tr, spec: &KeySpec, d: Rect, round: f32, size: f32) {
        let rect = Rect::from_min_size(t.p(d.min.x, d.min.y), d.size() * t.s);
        let id = ui.id().with(("key", d.min.x as i32, d.min.y as i32));
        let resp = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            self.calc.press(spec.key);
        }
        let down = resp.is_pointer_button_down_on();
        let (fill, border, fg) = match spec.style {
            Style::Dark => (Color32::from_rgb(0x34, 0x35, 0x3a), Color32::from_rgb(0x57, 0x58, 0x5e), Color32::WHITE),
            Style::Digit => (Color32::from_rgb(0x12, 0x12, 0x14), Color32::from_rgb(0x6a, 0x6a, 0x70), Color32::WHITE),
            Style::Blue => (Color32::from_rgb(0x2a, 0x5a, 0xcc), Color32::from_rgb(0x6f, 0x93, 0xe6), Color32::WHITE),
        };
        let fill = if down { fill.gamma_multiply(0.7) } else if resp.hovered() { fill.gamma_multiply(1.18) } else { fill };
        let off = if down { vec2(0.0, 1.5 * t.s) } else { vec2(0.0, 0.0) };
        let r = rect.translate(off);
        // drop shadow
        p.rect_filled(
            rect.translate(vec2(0.0, 2.0 * t.s)),
            CornerRadius::same(t.r(round)),
            Color32::from_rgba_unmultiplied(0, 0, 0, 110),
        );
        p.rect_filled(r, CornerRadius::same(t.r(round)), fill);
        p.rect_stroke(r, CornerRadius::same(t.r(round)), Stroke::new(1.1 * t.s, border), StrokeKind::Inside);
        let armed = spec.key == Key::Shift && self.calc.lcd().ind.shift;
        let fg = if armed { GOLD } else { fg };
        draw_label(p, t, r.center(), spec.label, size, fg, true);
        if !spec.shift.is_empty() {
            let c = pos2(r.center().x, rect.min.y - 8.5 * t.s);
            draw_label(p, t, c, spec.shift, size * 0.62, GOLD, true);
        }
        if !spec.below.is_empty() {
            let c = pos2(r.center().x, rect.max.y + 9.0 * t.s);
            draw_label(p, t, c, spec.below, size * 0.55, GOLD, true);
        }
    }
}

fn draw_label(p: &Painter, t: &Tr, center: Pos2, segs: &[Seg], size: f32, color: Color32, centered: bool) {
    let s = t.s;
    let font = |sz: f32| FontId::proportional(sz * s);
    // measure
    let width_of = |seg: &Seg| -> f32 {
        match seg {
            T(x) => p.layout_no_wrap(x.to_string(), font(size), color).size().x,
            Sup(x) | Sub(x) => p.layout_no_wrap(x.to_string(), font(size * 0.62), color).size().x,
            Over(x) => p.layout_no_wrap(x.to_string(), font(size), color).size().x,
            Root => size * 0.95 * s,
            Tri => size * 0.8 * s,
            Left | Right => size * 1.0 * s,
        }
    };
    let total: f32 = segs.iter().map(width_of).sum();
    let mut x = if centered { center.x - total / 2.0 } else { center.x };
    let cy = center.y;
    let stroke = Stroke::new((size * 0.09).max(1.0) * s, color);
    for seg in segs {
        let w = width_of(seg);
        match seg {
            T(txt) => {
                p.text(pos2(x, cy), Align2::LEFT_CENTER, txt, font(size), color);
            }
            Sup(txt) => {
                p.text(pos2(x, cy - size * 0.30 * s), Align2::LEFT_CENTER, txt, font(size * 0.62), color);
            }
            Sub(txt) => {
                p.text(pos2(x, cy + size * 0.25 * s), Align2::LEFT_CENTER, txt, font(size * 0.62), color);
            }
            Over(txt) => {
                p.text(pos2(x, cy), Align2::LEFT_CENTER, txt, font(size), color);
                let y = cy - size * 0.55 * s;
                p.line_segment([pos2(x + w * 0.1, y), pos2(x + w * 0.9, y)], stroke);
            }
            Root => {
                let h = size * s;
                let pts = vec![
                    pos2(x, cy + 0.05 * h),
                    pos2(x + 0.12 * h, cy),
                    pos2(x + 0.30 * h, cy + 0.38 * h),
                    pos2(x + 0.50 * h, cy - 0.42 * h),
                    pos2(x + w, cy - 0.42 * h),
                ];
                p.add(Shape::line(pts, PathStroke::new(stroke.width, color)));
            }
            Left | Right => {
                let (a, b) = if matches!(seg, Left) { (x + w * 0.9, x + w * 0.1) } else { (x + w * 0.1, x + w * 0.9) };
                let d = if a < b { -1.0 } else { 1.0 };
                let hd = size * 0.22 * s;
                p.line_segment([pos2(a, cy), pos2(b, cy)], stroke);
                p.line_segment([pos2(b, cy), pos2(b + d * hd, cy - hd)], stroke);
                p.line_segment([pos2(b, cy), pos2(b + d * hd, cy + hd)], stroke);
            }
            Tri => {
                let h = size * 0.55 * s;
                let pts = vec![
                    pos2(x, cy - h / 2.0),
                    pos2(x + w * 0.9, cy),
                    pos2(x, cy + h / 2.0),
                ];
                p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
            }
        }
        x += w;
    }
}

fn char_to_keys(c: char) -> &'static [Key] {
    use Key::*;
    match c {
        '0' => &[Digit(0)],
        '1' => &[Digit(1)],
        '2' => &[Digit(2)],
        '3' => &[Digit(3)],
        '4' => &[Digit(4)],
        '5' => &[Digit(5)],
        '6' => &[Digit(6)],
        '7' => &[Digit(7)],
        '8' => &[Digit(8)],
        '9' => &[Digit(9)],
        '.' | ',' => &[Dot],
        '+' => &[Add],
        '-' => &[Sub],
        '*' => &[Mul],
        '/' => &[Div],
        '(' => &[LParen],
        ')' => &[RParen],
        '^' => &[Pow],
        '=' => &[Eq],
        'e' | 'E' => &[Exp],
        'p' => &[Shift, Exp],
        's' => &[Sin],
        'c' => &[Cos],
        't' => &[Tan],
        'l' => &[Log],
        'n' => &[Ln],
        'r' => &[Shift, Square],
        'q' => &[Square],
        'h' => &[Hyp],
        'f' => &[AbC],
        'd' => &[Dms],
        '_' => &[Neg],
        'S' => &[Shift],
        'm' => &[Mode],
        '!' => &[Shift, Digit(3)],
        '%' => &[Shift, Eq],
        _ => &[],
    }
}
