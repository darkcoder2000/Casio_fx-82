//! Calculator state machine: key handling, entry buffer, modes, memory, statistics.

use std::time::{Duration, Instant};

use crate::eval::*;
use crate::format::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Shift,
    Mode,
    Square,
    Log,
    Ln,
    On,
    AbC,
    Dms,
    Hyp,
    Sin,
    Cos,
    Tan,
    Neg,
    Drg,
    LParen,
    RParen,
    Pow,
    Mr,
    Digit(u8),
    C,
    Ac,
    Mul,
    Div,
    Add,
    Sub,
    Dot,
    Exp,
    Eq,
    MPlus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalcMode {
    Comp,
    Sd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Menu {
    None,
    Main(u8),
    Drg,
    FixDigits,
    SciDigits,
    NormDigits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum EKind {
    #[default]
    Plain,
    Frac,
    Dms,
}

/// The number currently being typed.
#[derive(Clone, Debug, Default)]
struct Entry {
    segs: Vec<String>,
    kind: EKind,
    cur: String,
    exp: Option<String>,
    neg: bool,
}

impl Entry {
    fn value(&self) -> CResult<Value> {
        let sign = if self.neg { -1.0 } else { 1.0 };
        match self.kind {
            EKind::Plain => {
                let m = if self.cur.is_empty() { "0" } else { self.cur.trim_end_matches('.') };
                let m = if m.is_empty() { "0" } else { m };
                let text = match &self.exp {
                    Some(e) => {
                        let e = if e.is_empty() || e == "-" { "0" } else { e };
                        format!("{m}e{e}")
                    }
                    None => m.to_string(),
                };
                let x: f64 = text.parse().map_err(|_| CalcError::Syntax)?;
                Ok(Value::F(sign * x))
            }
            EKind::Frac => {
                let mut parts: Vec<&str> = self.segs.iter().map(String::as_str).collect();
                parts.push(&self.cur);
                let nums: Vec<i64> = parts
                    .iter()
                    .map(|p| p.parse::<i64>().map_err(|_| CalcError::Syntax))
                    .collect::<CResult<_>>()?;
                let (w, n, d) = match nums.as_slice() {
                    [n, d] => (0, *n, *d),
                    [w, n, d] => (*w, *n, *d),
                    _ => return Err(CalcError::Syntax),
                };
                if d == 0 {
                    return Err(CalcError::Math);
                }
                let total = w as i128 * d as i128 + n as i128;
                let total = if self.neg { -total } else { total };
                Value::rat(total, d as i128).ok_or(CalcError::Math)
            }
            EKind::Dms => {
                let mut parts: Vec<&str> = self.segs.iter().map(String::as_str).collect();
                if !self.cur.is_empty() {
                    parts.push(&self.cur);
                }
                let mut total = 0.0;
                let mut div = 1.0;
                for p in parts {
                    total += p.trim_end_matches('.').parse::<f64>().unwrap_or(0.0) / div;
                    div *= 60.0;
                }
                Ok(Value::F(fix12(sign * total)))
            }
        }
    }

    fn display(&self) -> (bool, String, Option<String>) {
        let sep = if self.kind == EKind::Frac { 'J' } else { '°' };
        let mut cells = String::new();
        for s in &self.segs {
            cells.push_str(s);
            cells.push(sep);
        }
        cells.push_str(&self.cur);
        if cells.is_empty() && !self.neg {
            cells.push('0');
        }
        let exp = self.exp.as_ref().map(|e| {
            let (sign, digits) = match e.strip_prefix('-') {
                Some(d) => ("-", d),
                None => ("", e.as_str()),
            };
            format!("{sign}{digits:0>2}")
        });
        (self.neg, cells, exp)
    }

    fn is_blank(&self) -> bool {
        self.segs.is_empty() && self.cur.is_empty() && self.exp.is_none() && !self.neg
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Val(Value),
    Dms(f64),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Indicators {
    pub shift: bool,
    pub m: bool,
    pub k: bool,
    pub hyp: bool,
    pub angle: Option<Angle>,
    pub fix: bool,
    pub sci: bool,
    pub sd: bool,
}

#[derive(Clone, Debug)]
pub struct Lcd {
    pub neg: bool,
    pub cells: String,
    pub exp: Option<String>,
    /// Plain text (menus, errors) replaces the digit field.
    pub text: Option<String>,
    pub ind: Indicators,
    pub off: bool,
}

pub struct Calc {
    toks: Vec<Tok>,
    entry: Option<Entry>,
    shown: Shown,
    ans: Value,
    mem: f64,
    y_reg: f64,
    angle: Angle,
    fmt: DispFmt,
    mode: CalcMode,
    shift: bool,
    hyp: bool,
    k: bool,
    error: Option<CalcError>,
    menu: Menu,
    eng: Option<i32>,
    improper: bool,
    repeat: Option<(Bin, Value)>,
    last_const: bool,
    stats: Vec<(f64, f64)>,
    off: bool,
    last_input: Instant,
    rng: u64,
}

const AUTO_OFF: Duration = Duration::from_secs(600);

fn negate(v: Value) -> Value {
    match v {
        Value::F(x) => Value::F(-x),
        Value::R(n, d) => Value::R(-n, d),
    }
}

impl Default for Calc {
    fn default() -> Self {
        Self::new()
    }
}

impl Calc {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15);
        Calc {
            toks: vec![],
            entry: None,
            shown: Shown::Val(Value::F(0.0)),
            ans: Value::F(0.0),
            mem: 0.0,
            y_reg: 0.0,
            angle: Angle::Deg,
            fmt: DispFmt::Norm(1),
            mode: CalcMode::Comp,
            shift: false,
            hyp: false,
            k: false,
            error: None,
            menu: Menu::None,
            eng: None,
            improper: false,
            repeat: None,
            last_const: false,
            stats: vec![],
            off: false,
            last_input: Instant::now(),
            rng: seed | 1,
        }
    }

    /// Call periodically; switches the display off after 10 minutes idle.
    pub fn tick(&mut self) {
        if !self.off && self.last_input.elapsed() > AUTO_OFF {
            self.off = true;
        }
    }

    // ------------------------------------------------------------------ LCD

    pub fn lcd(&self) -> Lcd {
        let mut lcd = Lcd {
            neg: false,
            cells: String::new(),
            exp: None,
            text: None,
            ind: Indicators::default(),
            off: self.off,
        };
        if self.off {
            return lcd;
        }
        lcd.ind = Indicators {
            shift: self.shift,
            m: self.mem != 0.0,
            k: self.k,
            hyp: self.hyp,
            angle: Some(self.angle),
            fix: matches!(self.fmt, DispFmt::Fix(_)),
            sci: matches!(self.fmt, DispFmt::Sci(_)),
            sd: self.mode == CalcMode::Sd,
        };
        if let Some(e) = self.error {
            lcd.text = Some(
                match e {
                    CalcError::Math => "Math ERROR",
                    CalcError::Syntax => "Syntax ERROR",
                    CalcError::Stack => "Stack ERROR",
                }
                .into(),
            );
            return lcd;
        }
        let menu_text = match self.menu {
            Menu::None => None,
            Menu::Main(0) => Some("COMP:1 SD:2"),
            Menu::Main(1) => Some("Deg:1 Rad:2 Gra:3"),
            Menu::Main(2) => Some("Fix:1 Sci:2 Norm:3"),
            Menu::Main(_) => Some("Mcl:1 Mode:2 All:3"),
            Menu::Drg => Some("DRG> D:1 R:2 G:3"),
            Menu::FixDigits => Some("Fix 0-9?"),
            Menu::SciDigits => Some("Sci 0-9?"),
            Menu::NormDigits => Some("Norm 1-2?"),
        };
        if let Some(t) = menu_text {
            lcd.text = Some(t.into());
            return lcd;
        }
        if let Some(e) = &self.entry {
            let (neg, cells, exp) = e.display();
            lcd.neg = neg;
            lcd.cells = cells;
            lcd.exp = exp;
            return lcd;
        }
        let f = match self.shown {
            Shown::Dms(x) => format_dms(x),
            Shown::Val(Value::R(n, d)) if d != 1 => format_frac(n, d, self.improper)
                .unwrap_or_else(|| format_f64(n as f64 / d as f64, self.fmt)),
            Shown::Val(v) => {
                let x = v.f();
                match self.eng {
                    Some(e) if x != 0.0 => format_eng(x, e),
                    _ => format_f64(x, self.fmt),
                }
            }
        };
        lcd.neg = f.neg;
        lcd.cells = f.cells;
        lcd.exp = f.exp.map(|e| format!("{}{:02}", if e < 0 { "-" } else { "" }, e.abs()));
        lcd
    }

    // ---------------------------------------------------------- key handling

    pub fn press(&mut self, key: Key) {
        self.last_input = Instant::now();
        if self.off {
            if key == Key::On {
                self.off = false;
            }
            return;
        }
        if key == Key::Shift {
            self.shift = !self.shift;
            return;
        }
        let shift = std::mem::take(&mut self.shift);
        let hyp = std::mem::take(&mut self.hyp);
        self.k = false;
        let eng_prev = if shift && matches!(key, Key::Mul | Key::Div) { self.eng } else { None };
        self.eng = None;

        if self.error.is_some() {
            if matches!(key, Key::Ac | Key::C | Key::On) {
                self.clear_expr();
            }
            return;
        }
        let r = if self.menu != Menu::None {
            self.press_menu(key)
        } else {
            self.dispatch(key, shift, hyp, eng_prev)
        };
        if let Err(e) = r {
            self.error = Some(e);
            self.toks.clear();
            self.entry = None;
            self.menu = Menu::None;
        }
    }

    fn clear_expr(&mut self) {
        self.toks.clear();
        self.entry = None;
        self.shown = Shown::Val(Value::F(0.0));
        self.error = None;
        self.menu = Menu::None;
        self.repeat = None;
        self.last_const = false;
    }

    fn dispatch(&mut self, key: Key, shift: bool, hyp: bool, eng_prev: Option<i32>) -> CResult<()> {
        use Key::*;
        match (key, shift) {
            (Shift, _) => {}
            (Mode, _) => self.menu = Menu::Main(0),
            (On, _) | (Ac, false) => self.clear_expr(),
            (Ac, true) => self.stats.clear(),
            (C, false) => self.clear_entry(),
            (C, true) => {
                let old = self.mem;
                let cur = self.eval_for_store()?;
                self.mem = fix12(cur.f());
                self.replace_value(|_| Value::F(old))?;
            }
            (Hyp, _) => self.hyp = true,
            (Digit(d), false) => self.input_char((b'0' + d) as char)?,
            (Dot, false) => self.input_char('.')?,
            (Exp, false) => self.input_exp(),
            (Exp, true) => self.push_const(Tok::Pi, Value::F(std::f64::consts::PI))?,
            (Eq, false) => self.equals()?,
            (Eq, true) => self.push_post(Post::Percent)?,
            (Add, false) => self.push_bin(Bin::Add)?,
            (Add, true) => self.push_bin(Bin::ToPol)?,
            (Sub, false) => self.push_bin(Bin::Sub)?,
            (Sub, true) => self.push_bin(Bin::ToRec)?,
            (Mul, false) => self.push_bin(Bin::Mul)?,
            (Div, false) => self.push_bin(Bin::Div)?,
            (Mul, true) | (Div, true) => self.engineering(key == Div, eng_prev),
            (Pow, false) => self.push_bin(Bin::Pow)?,
            (Pow, true) => self.push_bin(Bin::Root)?,
            (Square, false) => self.push_post(Post::Sq)?,
            (Square, true) => self.push_pre(Func::Sqrt)?,
            (Log, false) => self.push_pre(Func::Log)?,
            (Log, true) => self.push_pre(Func::Pow10)?,
            (Ln, false) => self.push_pre(Func::Ln)?,
            (Ln, true) => self.push_pre(Func::ExpE)?,
            (Sin, _) | (Cos, _) | (Tan, _) => {
                let f = match (key, hyp, shift) {
                    (Sin, false, false) => Func::Sin,
                    (Cos, false, false) => Func::Cos,
                    (Tan, false, false) => Func::Tan,
                    (Sin, false, true) => Func::Asin,
                    (Cos, false, true) => Func::Acos,
                    (Tan, false, true) => Func::Atan,
                    (Sin, true, false) => Func::Sinh,
                    (Cos, true, false) => Func::Cosh,
                    (Tan, true, false) => Func::Tanh,
                    (Sin, true, true) => Func::Asinh,
                    (Cos, true, true) => Func::Acosh,
                    _ => Func::Atanh,
                };
                self.push_pre(f)?;
            }
            (Neg, false) => self.negate_current(),
            (Neg, true) => self.push_pre(Func::Cbrt)?,
            (Drg, false) => self.menu = Menu::Drg,
            (Drg, true) => self.push_post(Post::Cube)?,
            (LParen, false) => self.push_lparen()?,
            (LParen, true) => self.swap_y()?,
            (RParen, false) => self.push_rparen()?,
            (RParen, true) => self.push_post(Post::Inv)?,
            (Mr, false) => self.push_const_num(self.mem)?,
            (Mr, true) => {
                let v = self.eval_for_store()?;
                self.mem = fix12(v.f());
            }
            (MPlus, false) if self.mode == CalcMode::Sd => self.add_data()?,
            (MPlus, true) if self.mode == CalcMode::Sd => {
                self.stats.pop();
                self.shown = Shown::Val(Value::F(self.stat_n()));
            }
            (MPlus, sh) => {
                let v = self.eval_for_store()?.f();
                self.mem = fix12(if sh { self.mem - v } else { self.mem + v });
                if self.mem.abs() >= 1e100 {
                    self.mem = 0.0;
                    return Err(CalcError::Math);
                }
            }
            (Digit(d), true) => self.digit_shift(d)?,
            (Dot, true) => {
                let r = self.next_rand();
                self.push_const_num(r)?;
            }
            (AbC, false) => self.abc(),
            (AbC, true) => self.improper = !self.improper,
            (Dms, false) => self.dms_key(),
            (Dms, true) => self.backspace(),
        }
        Ok(())
    }

    fn digit_shift(&mut self, d: u8) -> CResult<()> {
        match d {
            0 => {
                let fmt = self.fmt;
                self.replace_value(|v| Value::F(round_to_format(v.f(), fmt)))
            }
            1 => self.push_bin(Bin::NPr),
            2 => self.push_bin(Bin::NCr),
            3 => self.push_post(Post::Fact),
            4..=9 if self.mode != CalcMode::Sd => Ok(()),
            4 => self.push_stat(|_, _, sxx, _| Ok(sxx)),
            5 => self.push_stat(|_, sx, _, _| Ok(sx)),
            6 => self.push_stat(|n, _, _, _| Ok(n)),
            7 => self.push_stat(|n, sx, _, _| {
                if n == 0.0 {
                    Err(CalcError::Math)
                } else {
                    Ok(sx / n)
                }
            }),
            8 => self.push_stat(|n, _, _, ss| {
                if n == 0.0 {
                    Err(CalcError::Math)
                } else {
                    Ok((ss / n).sqrt())
                }
            }),
            _ => self.push_stat(|n, _, _, ss| {
                if n < 2.0 {
                    Err(CalcError::Math)
                } else {
                    Ok((ss / (n - 1.0)).sqrt())
                }
            }),
        }
    }

    // ------------------------------------------------------------ menu / mode

    fn press_menu(&mut self, key: Key) -> CResult<()> {
        match (self.menu, key) {
            (_, Key::Ac | Key::On | Key::C) => self.menu = Menu::None,
            (Menu::Main(p), Key::Mode) => self.menu = Menu::Main((p + 1) % 4),
            (Menu::Main(p), Key::Digit(d)) => self.select_main(p, d),
            (Menu::Drg, Key::Digit(d @ 1..=3)) => {
                self.menu = Menu::None;
                let to = [Angle::Deg, Angle::Rad, Angle::Gra][d as usize - 1];
                let from = self.angle;
                self.replace_value(|v| Value::F(fix12(to.from_deg(from.to_deg(v.f())))))?;
            }
            (Menu::FixDigits, Key::Digit(d)) => {
                self.fmt = DispFmt::Fix(d);
                self.menu = Menu::None;
            }
            (Menu::SciDigits, Key::Digit(d)) => {
                self.fmt = DispFmt::Sci(d);
                self.menu = Menu::None;
            }
            (Menu::NormDigits, Key::Digit(d @ 1..=2)) => {
                self.fmt = DispFmt::Norm(d);
                self.menu = Menu::None;
            }
            _ => {}
        }
        Ok(())
    }

    fn select_main(&mut self, page: u8, d: u8) {
        self.menu = Menu::None;
        match (page, d) {
            (0, 1) => self.set_mode(CalcMode::Comp),
            (0, 2) => self.set_mode(CalcMode::Sd),
            (1, 1..=3) => {
                self.angle = [Angle::Deg, Angle::Rad, Angle::Gra][d as usize - 1];
                self.clear_expr();
            }
            (2, 1) => self.menu = Menu::FixDigits,
            (2, 2) => self.menu = Menu::SciDigits,
            (2, 3) => self.menu = Menu::NormDigits,
            (3, 1) => self.mem = 0.0,
            (3, 2) => self.reset_settings(),
            (3, 3) => {
                self.mem = 0.0;
                self.stats.clear();
                self.reset_settings();
            }
            _ => self.menu = Menu::Main(page),
        }
    }

    fn set_mode(&mut self, m: CalcMode) {
        self.mode = m;
        if m == CalcMode::Sd {
            self.stats.clear();
        }
        self.clear_expr();
    }

    fn reset_settings(&mut self) {
        self.mode = CalcMode::Comp;
        self.angle = Angle::Deg;
        self.fmt = DispFmt::Norm(1);
        self.improper = false;
        self.clear_expr();
    }

    // ------------------------------------------------------- token plumbing

    fn push(&mut self, t: Tok) -> CResult<()> {
        if self.toks.len() >= 80 {
            return Err(CalcError::Stack);
        }
        self.toks.push(t);
        Ok(())
    }

    fn current_value(&self) -> Value {
        match self.shown {
            Shown::Val(v) => v,
            Shown::Dms(x) => Value::F(x),
        }
    }

    /// Turn the number being typed into a token.
    fn commit(&mut self) -> CResult<()> {
        if let Some(e) = self.entry.take() {
            let v = e.value()?;
            self.push(Tok::Num(v))?;
            self.shown = Shown::Val(v);
            self.last_const = false;
        }
        Ok(())
    }

    fn push_const(&mut self, t: Tok, v: Value) -> CResult<()> {
        self.commit()?;
        self.push(t)?;
        self.shown = Shown::Val(v);
        self.last_const = true;
        Ok(())
    }

    fn push_const_num(&mut self, x: f64) -> CResult<()> {
        let v = Value::F(x);
        self.push_const(Tok::Num(v), v)
    }

    fn push_bin(&mut self, b: Bin) -> CResult<()> {
        self.commit()?;
        if self.toks.is_empty() {
            let v = self.current_value();
            self.push(Tok::Num(v))?;
        } else if matches!(self.toks.last(), Some(Tok::Bin(_))) {
            self.toks.pop();
        }
        self.last_const = false;
        self.push(Tok::Bin(b))
    }

    fn push_post(&mut self, p: Post) -> CResult<()> {
        self.commit()?;
        if self.toks.is_empty() {
            let v = self.current_value();
            self.push(Tok::Num(v))?;
        }
        self.push(Tok::Post(p))
    }

    fn push_pre(&mut self, f: Func) -> CResult<()> {
        self.commit()?;
        self.last_const = false;
        self.push(Tok::Pre(f))
    }

    fn open_parens(&self) -> usize {
        let mut d = 0usize;
        for t in &self.toks {
            match t {
                Tok::LP => d += 1,
                Tok::RP => d = d.saturating_sub(1),
                _ => {}
            }
        }
        d
    }

    fn push_lparen(&mut self) -> CResult<()> {
        self.commit()?;
        if self.open_parens() >= 6 {
            return Err(CalcError::Stack);
        }
        self.last_const = false;
        self.push(Tok::LP)
    }

    fn push_rparen(&mut self) -> CResult<()> {
        self.commit()?;
        if self.open_parens() > 0 {
            self.push(Tok::RP)?;
        }
        Ok(())
    }

    // ------------------------------------------------------------- evaluation

    fn run_eval(&mut self) -> CResult<Value> {
        let toks = std::mem::take(&mut self.toks);
        let mut ctx = Ctx { angle: self.angle, y: None };
        let r = evaluate(&toks, &mut ctx)?;
        if let Some(y) = ctx.y {
            self.y_reg = y;
        }
        self.ans = r;
        self.shown = Shown::Val(r);
        self.last_const = false;
        self.repeat = match toks.as_slice() {
            [Tok::Num(_), Tok::Bin(op @ (Bin::Add | Bin::Sub | Bin::Mul | Bin::Div | Bin::Pow)), Tok::Num(v)] => {
                Some((*op, *v))
            }
            _ => None,
        };
        Ok(r)
    }

    fn equals(&mut self) -> CResult<()> {
        self.commit()?;
        if self.toks.is_empty() {
            if let Some((op, v)) = self.repeat {
                let mut ctx = Ctx { angle: self.angle, y: None };
                let r = binop(op, self.ans, v, &mut ctx)?;
                self.ans = r;
                self.shown = Shown::Val(r);
                self.k = true;
            }
            return Ok(());
        }
        self.run_eval().map(|_| ())
    }

    /// Evaluate any pending expression and return the displayed value.
    fn eval_for_store(&mut self) -> CResult<Value> {
        self.commit()?;
        if !self.toks.is_empty() {
            self.run_eval()?;
        }
        Ok(self.current_value())
    }

    /// Apply `f` to the value just entered / displayed (RND, DRG►, X↔M).
    fn replace_value(&mut self, f: impl FnOnce(Value) -> Value) -> CResult<()> {
        self.commit()?;
        match self.toks.last_mut() {
            Some(Tok::Num(v)) => {
                *v = f(*v);
                self.shown = Shown::Val(*v);
            }
            Some(_) => {}
            None => {
                let v = f(self.current_value());
                self.ans = v;
                self.shown = Shown::Val(v);
            }
        }
        Ok(())
    }

    fn swap_y(&mut self) -> CResult<()> {
        let y = self.y_reg;
        let cur = self.current_value().f();
        self.replace_value(|_| Value::F(y))?;
        self.y_reg = cur;
        Ok(())
    }

    fn engineering(&mut self, up: bool, prev: Option<i32>) {
        if self.entry.is_some() || !self.toks.is_empty() {
            return;
        }
        let x = self.current_value().f();
        if x == 0.0 || matches!(self.shown, Shown::Dms(_)) {
            return;
        }
        let (_, x10) = digits_exp(x, 10);
        let base = x10.div_euclid(3) * 3;
        let e = match prev {
            Some(e) => e + if up { 3 } else { -3 },
            None if up => base,
            None => base - 3,
        };
        if (-99..=99).contains(&e) {
            self.eng = Some(e);
        }
    }

    fn next_rand(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        (x % 1000) as f64 / 1000.0
    }

    // ----------------------------------------------------------- entry editing

    fn new_entry(&mut self) -> &mut Entry {
        if self.entry.is_none() {
            if self.last_const {
                self.toks.pop();
                self.last_const = false;
            }
            self.entry = Some(Entry::default());
        }
        self.entry.as_mut().unwrap()
    }

    fn input_char(&mut self, c: char) -> CResult<()> {
        let e = self.new_entry();
        if let Some(x) = e.exp.as_mut() {
            if c != '.' && x.trim_start_matches('-').len() < 2 {
                x.push(c);
            }
            return Ok(());
        }
        if c == '.' {
            if e.kind == EKind::Frac || e.cur.contains('.') {
                return Ok(());
            }
            if e.cur.is_empty() {
                e.cur.push('0');
            }
            e.cur.push('.');
            return Ok(());
        }
        if e.cur.chars().filter(char::is_ascii_digit).count() >= 10 {
            return Ok(());
        }
        if e.cur == "0" {
            e.cur.clear();
        }
        e.cur.push(c);
        Ok(())
    }

    fn input_exp(&mut self) {
        let e = self.new_entry();
        if e.kind != EKind::Plain || e.exp.is_some() {
            return;
        }
        if e.cur.is_empty() {
            e.cur.push('1');
        }
        e.exp = Some(String::new());
    }

    fn negate_current(&mut self) {
        if let Some(e) = self.entry.as_mut() {
            match e.exp.as_mut() {
                Some(x) => {
                    if x.starts_with('-') {
                        x.remove(0);
                    } else {
                        x.insert(0, '-');
                    }
                }
                None => e.neg = !e.neg,
            }
            return;
        }
        if self.toks.is_empty() {
            let v = negate(self.current_value());
            self.ans = v;
            self.shown = Shown::Val(v);
            return;
        }
        if self.last_const {
            match self.toks.last_mut() {
                Some(Tok::Num(v)) => {
                    *v = negate(*v);
                    self.shown = Shown::Val(*v);
                    return;
                }
                Some(t @ Tok::Pi) => {
                    *t = Tok::Num(Value::F(-std::f64::consts::PI));
                    self.shown = Shown::Val(Value::F(-std::f64::consts::PI));
                    return;
                }
                _ => {}
            }
        }
        self.entry = Some(Entry { neg: true, ..Entry::default() });
    }

    fn backspace(&mut self) {
        let Some(e) = self.entry.as_mut() else { return };
        if let Some(x) = e.exp.as_mut() {
            if x.pop().is_none() {
                e.exp = None;
            }
        } else if e.cur.pop().is_none() {
            if let Some(s) = e.segs.pop() {
                e.cur = s;
                if e.segs.is_empty() {
                    e.kind = EKind::Plain;
                }
            } else {
                e.neg = false;
            }
        }
        if e.is_blank() {
            self.entry = None;
            self.shown = Shown::Val(Value::F(0.0));
        }
    }

    fn clear_entry(&mut self) {
        if self.entry.take().is_some() {
            self.shown = Shown::Val(Value::F(0.0));
        } else if !self.toks.is_empty() {
            self.toks.pop();
            self.last_const = false;
        } else {
            self.shown = Shown::Val(Value::F(0.0));
        }
    }

    fn abc(&mut self) {
        if let Some(e) = self.entry.as_mut() {
            let ok = e.kind != EKind::Dms
                && e.exp.is_none()
                && !e.cur.is_empty()
                && !e.cur.contains('.')
                && e.segs.len() < 2;
            if ok {
                let c = std::mem::take(&mut e.cur);
                e.segs.push(c);
                e.kind = EKind::Frac;
            }
            return;
        }
        if !self.toks.is_empty() {
            return;
        }
        let converted = match self.current_value() {
            Value::R(n, d) => Some(Value::F(n as f64 / d as f64)),
            Value::F(x) => to_fraction(x).and_then(|(n, d)| Value::rat(n as i128, d as i128)),
        };
        if let Some(v) = converted {
            self.ans = v;
            self.shown = Shown::Val(v);
        }
    }

    fn dms_key(&mut self) {
        if let Some(e) = self.entry.as_mut() {
            let ok = e.kind != EKind::Frac && e.exp.is_none() && !e.cur.is_empty() && e.segs.len() < 3;
            if ok {
                let c = std::mem::take(&mut e.cur);
                e.segs.push(c);
                e.kind = EKind::Dms;
            }
            return;
        }
        if self.toks.is_empty() {
            self.shown = Shown::Dms(self.current_value().f());
        }
    }

    // ------------------------------------------------------------ statistics

    fn stat_n(&self) -> f64 {
        self.stats.iter().map(|&(_, f)| f).sum()
    }

    fn push_stat(&mut self, f: impl Fn(f64, f64, f64, f64) -> CResult<f64>) -> CResult<()> {
        let n = self.stat_n();
        let sx: f64 = self.stats.iter().map(|&(x, f)| x * f).sum();
        let sxx: f64 = self.stats.iter().map(|&(x, f)| x * x * f).sum();
        let mean = if n > 0.0 { sx / n } else { 0.0 };
        let ss: f64 = self.stats.iter().map(|&(x, f)| f * (x - mean) * (x - mean)).sum();
        let v = f(n, sx, sxx, ss)?;
        let v = fix12(v);
        if v.abs() >= 1e100 {
            return Err(CalcError::Math);
        }
        self.push_const_num(v)
    }

    fn add_data(&mut self) -> CResult<()> {
        self.commit()?;
        let toks = std::mem::take(&mut self.toks);
        let eval = |t: &[Tok], angle| -> CResult<f64> {
            let mut ctx = Ctx { angle, y: None };
            evaluate(t, &mut ctx).map(|v| v.f())
        };
        let (x, f) = if toks.is_empty() {
            (self.current_value().f(), 1.0)
        } else {
            let mut depth = 0;
            let mut split = None;
            for (i, t) in toks.iter().enumerate() {
                match t {
                    Tok::LP => depth += 1,
                    Tok::RP => depth = i32::max(depth - 1, 0),
                    Tok::Bin(Bin::Mul) if depth == 0 => split = Some(i),
                    _ => {}
                }
            }
            match split {
                Some(i) => (eval(&toks[..i], self.angle)?, eval(&toks[i + 1..], self.angle)?),
                None => (eval(&toks, self.angle)?, 1.0),
            }
        };
        if f.fract() != 0.0 || !(1.0..100.0).contains(&f) {
            return Err(CalcError::Math);
        }
        self.stats.push((x, f));
        self.shown = Shown::Val(Value::F(self.stat_n()));
        self.last_const = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny DSL: digits/./+-*/= plus `S` (shift), `(`, `)`, `e` (EXP), `n` (+/-), `A`(AC),
    /// `s c t` sin/cos/tan, `q` (x²), `f` (a b/c), `M` (MODE), `m` (M+), `r` (MR).
    fn run(c: &mut Calc, s: &str) {
        for ch in s.chars() {
            let k = match ch {
                '0'..='9' => Key::Digit(ch as u8 - b'0'),
                '.' => Key::Dot,
                '+' => Key::Add,
                '-' => Key::Sub,
                '*' => Key::Mul,
                '/' => Key::Div,
                '=' => Key::Eq,
                'S' => Key::Shift,
                '(' => Key::LParen,
                ')' => Key::RParen,
                'e' => Key::Exp,
                'n' => Key::Neg,
                'A' => Key::Ac,
                's' => Key::Sin,
                'c' => Key::Cos,
                't' => Key::Tan,
                'q' => Key::Square,
                'f' => Key::AbC,
                'M' => Key::Mode,
                'm' => Key::MPlus,
                'r' => Key::Mr,
                'p' => Key::Pow,
                'd' => Key::Dms,
                'l' => Key::Log,
                ' ' => continue,
                _ => panic!("bad key {ch}"),
            };
            c.press(k);
        }
    }

    fn out(c: &Calc) -> String {
        let l = c.lcd();
        if let Some(t) = l.text {
            return t;
        }
        format!(
            "{}{}{}",
            if l.neg { "-" } else { "" },
            l.cells,
            l.exp.map(|e| format!(" e{e}")).unwrap_or_default()
        )
    }

    fn calc(s: &str) -> String {
        let mut c = Calc::new();
        run(&mut c, s);
        out(&c)
    }

    #[test]
    fn basics() {
        assert_eq!(calc("2+3*4="), "14");
        assert_eq!(calc("(2+3)*4="), "20");
        assert_eq!(calc("1/3="), "0.3333333333");
        assert_eq!(calc("1/0="), "Math ERROR");
        assert_eq!(calc("5*="), "Syntax ERROR");
        assert_eq!(calc("1.5e3="), "1500");
        assert_eq!(calc("1e10="), "1 e10");
        assert_eq!(calc("5n+2="), "-3");
        assert_eq!(calc("2p10="), "1024");
        assert_eq!(calc("2+3=*2="), "10");
    }

    #[test]
    fn functions() {
        assert_eq!(calc("s30="), "0.5");
        assert_eq!(calc("Ss1="), "90");
        assert_eq!(calc("Sq144="), "12");
        assert_eq!(calc("12q="), "144");
        assert_eq!(calc("5S2 2="), "10"); // nCr
        assert_eq!(calc("5S1 2="), "20"); // nPr
        assert_eq!(calc("5S3="), "120");
        assert_eq!(calc("200*15S==="), "30");
        assert_eq!(calc("3Sp27="), "3");
    }

    #[test]
    fn repeat_and_ans() {
        let mut c = Calc::new();
        run(&mut c, "2+3==");
        assert_eq!(out(&c), "8");
        assert!(c.lcd().ind.k);
    }

    #[test]
    fn fractions() {
        assert_eq!(calc("1f2+1f3="), "5J6");
        assert_eq!(calc("2f1f4="), "2J1J4");
        let mut c = Calc::new();
        run(&mut c, "2f1f4=");
        c.press(Key::Shift);
        c.press(Key::AbC);
        assert_eq!(out(&c), "9J4");
        c.press(Key::AbC);
        assert_eq!(out(&c), "2.25");
        c.press(Key::AbC);
        assert_eq!(out(&c), "9J4");
        assert_eq!(calc("1f0="), "Math ERROR");
    }

    #[test]
    fn dms() {
        assert_eq!(calc("12d30d45d="), "12.5125");
        let mut c = Calc::new();
        run(&mut c, "12.5125=");
        c.press(Key::Dms);
        assert_eq!(out(&c), "12°30°45°");
    }

    #[test]
    fn eng_and_modes() {
        let mut c = Calc::new();
        run(&mut c, "12345=");
        c.press(Key::Shift);
        c.press(Key::Div);
        assert_eq!(out(&c), "12.345 e03");
        c.press(Key::Shift);
        c.press(Key::Div);
        assert_eq!(out(&c), "0.012345 e06");
        c.press(Key::Shift);
        c.press(Key::Mul);
        assert_eq!(out(&c), "12.345 e03");
        // Fix 2
        let mut c = Calc::new();
        run(&mut c, "MMM12");
        run(&mut c, "2/3=");
        assert_eq!(out(&c), "0.67");
        assert!(c.lcd().ind.fix);
    }

    #[test]
    fn polar_and_swap() {
        let mut c = Calc::new();
        run(&mut c, "3S+4=");
        assert_eq!(out(&c), "5");
        run(&mut c, "S(");
        assert_eq!(out(&c), "53.13010235");
    }

    #[test]
    fn memory() {
        let mut c = Calc::new();
        run(&mut c, "5m");
        assert!(c.lcd().ind.m);
        run(&mut c, "2*r=");
        assert_eq!(out(&c), "10");
        run(&mut c, "3Sm");
        run(&mut c, "r=");
        assert_eq!(out(&c), "2");
    }

    #[test]
    fn statistics() {
        let mut c = Calc::new();
        run(&mut c, "M2"); // SD mode
        assert!(c.lcd().ind.sd);
        run(&mut c, "10m20m30m");
        assert_eq!(out(&c), "3");
        run(&mut c, "S7=");
        assert_eq!(out(&c), "20");
        run(&mut c, "S9=");
        assert_eq!(out(&c), "10");
        run(&mut c, "S8=");
        assert_eq!(out(&c), "8.164965809");
        run(&mut c, "S5=");
        assert_eq!(out(&c), "60");
        // frequency entry
        run(&mut c, "AAS"); // clear all, shift armed
        c.press(Key::Ac); // SAC
        run(&mut c, "5*3m");
        assert_eq!(out(&c), "3");
    }

    #[test]
    fn parens_depth() {
        assert_eq!(calc("((((((((1"), "Stack ERROR");
    }

    #[test]
    fn auto_close_and_neg_exp() {
        assert_eq!(calc("(2+3="), "5");
        assert_eq!(calc("1e5n="), "1 e-05");
        assert_eq!(calc("Sl3="), "1000"); // 10^x of 3
    }
}
