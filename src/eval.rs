//! Values, tokens and the expression evaluator (priority parser).

use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalcError {
    Math,
    Syntax,
    Stack,
}

pub type CResult<T> = Result<T, CalcError>;

/// A calculator value: either a decimal or an exact fraction (n/d, d > 0, reduced).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    F(f64),
    R(i64, i64),
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Value {
    pub fn f(self) -> f64 {
        match self {
            Value::F(x) => x,
            Value::R(n, d) => n as f64 / d as f64,
        }
    }

    /// Reduced fraction; `None` if it does not fit or d == 0.
    pub fn rat(n: i128, d: i128) -> Option<Value> {
        if d == 0 {
            return None;
        }
        let g = gcd(n, d).max(1);
        let (mut n, mut d) = (n / g, d / g);
        if d < 0 {
            n = -n;
            d = -d;
        }
        Some(Value::R(i64::try_from(n).ok()?, i64::try_from(d).ok()?))
    }
}

/// Round to 12 significant digits like the real calculator's internal precision.
pub fn fix12(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    format!("{:.11e}", x).parse().unwrap_or(x)
}

fn check(x: f64) -> CResult<Value> {
    if !x.is_finite() || x.abs() >= 1e100 {
        return Err(CalcError::Math);
    }
    let x = fix12(x);
    Ok(Value::F(if x.abs() < 1e-99 { 0.0 } else { x }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Angle {
    Deg,
    Rad,
    Gra,
}

impl Angle {
    pub fn to_deg(self, x: f64) -> f64 {
        match self {
            Angle::Deg => x,
            Angle::Rad => x * 180.0 / PI,
            Angle::Gra => x * 0.9,
        }
    }
    pub fn from_deg(self, d: f64) -> f64 {
        match self {
            Angle::Deg => d,
            Angle::Rad => d * PI / 180.0,
            Angle::Gra => d / 0.9,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Root,
    NPr,
    NCr,
    ToPol,
    ToRec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Log,
    Ln,
    Pow10,
    ExpE,
    Sqrt,
    Cbrt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Post {
    Sq,
    Cube,
    Inv,
    Fact,
    Percent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tok {
    Num(Value),
    Pi,
    Bin(Bin),
    Pre(Func),
    Post(Post),
    LP,
    RP,
}

pub struct Ctx {
    pub angle: Angle,
    /// Second result of R→P / P→R.
    pub y: Option<f64>,
}

/// Evaluate a token list. Missing `)` are closed implicitly.
pub fn evaluate(toks: &[Tok], ctx: &mut Ctx) -> CResult<Value> {
    let mut depth = 0usize;
    for t in toks {
        match t {
            Tok::LP => {
                depth += 1;
                if depth > 6 {
                    return Err(CalcError::Stack);
                }
            }
            Tok::RP => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    if toks.len() > 79 {
        return Err(CalcError::Stack);
    }
    let mut p = Parser { toks, pos: 0, ctx };
    let v = p.expr()?;
    if p.pos != toks.len() {
        return Err(CalcError::Syntax);
    }
    Ok(v)
}

struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
    ctx: &'a mut Ctx,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Tok> {
        self.toks.get(self.pos).copied()
    }

    fn eat_bin(&mut self, ops: &[Bin]) -> Option<Bin> {
        if let Some(Tok::Bin(b)) = self.peek() {
            if ops.contains(&b) {
                self.pos += 1;
                return Some(b);
            }
        }
        None
    }

    fn expr(&mut self) -> CResult<Value> {
        let mut l = self.addsub()?;
        while let Some(op) = self.eat_bin(&[Bin::ToPol, Bin::ToRec]) {
            let r = self.addsub()?;
            l = binop(op, l, r, self.ctx)?;
        }
        Ok(l)
    }

    fn addsub(&mut self) -> CResult<Value> {
        let mut l = self.muldiv()?;
        while let Some(op) = self.eat_bin(&[Bin::Add, Bin::Sub]) {
            let r = self.muldiv()?;
            l = binop(op, l, r, self.ctx)?;
        }
        Ok(l)
    }

    fn muldiv(&mut self) -> CResult<Value> {
        let mut l = self.perm()?;
        while let Some(op) = self.eat_bin(&[Bin::Mul, Bin::Div]) {
            let r = self.perm()?;
            l = binop(op, l, r, self.ctx)?;
        }
        Ok(l)
    }

    fn perm(&mut self) -> CResult<Value> {
        let mut l = self.implicit()?;
        while let Some(op) = self.eat_bin(&[Bin::NPr, Bin::NCr]) {
            let r = self.implicit()?;
            l = binop(op, l, r, self.ctx)?;
        }
        Ok(l)
    }

    fn implicit(&mut self) -> CResult<Value> {
        let mut l = self.power()?;
        while matches!(
            self.peek(),
            Some(Tok::Num(_) | Tok::Pi | Tok::LP | Tok::Pre(_))
        ) {
            let r = self.power()?;
            l = binop(Bin::Mul, l, r, self.ctx)?;
        }
        Ok(l)
    }

    /// `^` and `ˣ√` are right-associative.
    fn power(&mut self) -> CResult<Value> {
        let base = self.postfix()?;
        if let Some(op) = self.eat_bin(&[Bin::Pow, Bin::Root]) {
            let rhs = self.power()?;
            return binop(op, base, rhs, self.ctx);
        }
        Ok(base)
    }

    fn postfix(&mut self) -> CResult<Value> {
        let mut v = self.primary()?;
        while let Some(Tok::Post(p)) = self.peek() {
            self.pos += 1;
            v = postop(p, v)?;
        }
        Ok(v)
    }

    fn primary(&mut self) -> CResult<Value> {
        let t = self.peek().ok_or(CalcError::Syntax)?;
        self.pos += 1;
        match t {
            Tok::Num(v) => Ok(v),
            Tok::Pi => Ok(Value::F(PI)),
            Tok::LP => {
                let v = self.expr()?;
                if matches!(self.peek(), Some(Tok::RP)) {
                    self.pos += 1;
                }
                Ok(v)
            }
            Tok::Pre(f) => {
                let x = self.postfix()?;
                func(f, x, self.ctx.angle)
            }
            Tok::Bin(Bin::Sub) => {
                let v = self.postfix()?;
                binop(Bin::Sub, Value::F(0.0), v, self.ctx)
            }
            _ => Err(CalcError::Syntax),
        }
    }
}

fn postop(p: Post, v: Value) -> CResult<Value> {
    match p {
        Post::Sq => binop_arith(Bin::Mul, v, v),
        Post::Cube => {
            let s = binop_arith(Bin::Mul, v, v)?;
            binop_arith(Bin::Mul, s, v)
        }
        Post::Inv => binop_arith(Bin::Div, Value::R(1, 1), v),
        Post::Percent => check(v.f() / 100.0),
        Post::Fact => {
            let x = v.f();
            if x < 0.0 || x.fract() != 0.0 || x > 69.0 {
                return Err(CalcError::Math);
            }
            let mut r = 1.0f64;
            for i in 2..=(x as u32) {
                r *= i as f64;
            }
            check(r)
        }
    }
}

fn reduce_trig_arg(x: f64, angle: Angle) -> Option<i64> {
    // Some(k) when the argument is an exact multiple of 90 degrees.
    let k = angle.to_deg(x) / 90.0;
    ((k - k.round()).abs() < 1e-10).then(|| k.round() as i64)
}

fn func(f: Func, v: Value, angle: Angle) -> CResult<Value> {
    let x = v.f();
    let r = match f {
        Func::Sin => match reduce_trig_arg(x, angle) {
            Some(k) => [0.0, 1.0, 0.0, -1.0][k.rem_euclid(4) as usize],
            None => angle.to_deg(x).to_radians().sin(),
        },
        Func::Cos => match reduce_trig_arg(x, angle) {
            Some(k) => [1.0, 0.0, -1.0, 0.0][k.rem_euclid(4) as usize],
            None => angle.to_deg(x).to_radians().cos(),
        },
        Func::Tan => match reduce_trig_arg(x, angle) {
            Some(k) if k.rem_euclid(2) == 0 => 0.0,
            Some(_) => return Err(CalcError::Math),
            None => angle.to_deg(x).to_radians().tan(),
        },
        Func::Asin => {
            if x.abs() > 1.0 {
                return Err(CalcError::Math);
            }
            angle.from_deg(x.asin().to_degrees())
        }
        Func::Acos => {
            if x.abs() > 1.0 {
                return Err(CalcError::Math);
            }
            angle.from_deg(x.acos().to_degrees())
        }
        Func::Atan => angle.from_deg(x.atan().to_degrees()),
        Func::Sinh => x.sinh(),
        Func::Cosh => x.cosh(),
        Func::Tanh => x.tanh(),
        Func::Asinh => x.asinh(),
        Func::Acosh => {
            if x < 1.0 {
                return Err(CalcError::Math);
            }
            x.acosh()
        }
        Func::Atanh => {
            if x.abs() >= 1.0 {
                return Err(CalcError::Math);
            }
            x.atanh()
        }
        Func::Log => {
            if x <= 0.0 {
                return Err(CalcError::Math);
            }
            x.log10()
        }
        Func::Ln => {
            if x <= 0.0 {
                return Err(CalcError::Math);
            }
            x.ln()
        }
        Func::Pow10 => 10f64.powf(x),
        Func::ExpE => x.exp(),
        Func::Sqrt => {
            if x < 0.0 {
                return Err(CalcError::Math);
            }
            x.sqrt()
        }
        Func::Cbrt => x.cbrt(),
    };
    check(r)
}

/// + − × ÷ on fractions stay exact, otherwise decimal.
fn binop_arith(op: Bin, a: Value, b: Value) -> CResult<Value> {
    if let (Value::R(an, ad), Value::R(bn, bd)) = (a, b) {
        let (an, ad, bn, bd) = (an as i128, ad as i128, bn as i128, bd as i128);
        let r = match op {
            Bin::Add => Value::rat(an * bd + bn * ad, ad * bd),
            Bin::Sub => Value::rat(an * bd - bn * ad, ad * bd),
            Bin::Mul => Value::rat(an * bn, ad * bd),
            Bin::Div => {
                if bn == 0 {
                    return Err(CalcError::Math);
                }
                Value::rat(an * bd, ad * bn)
            }
            _ => None,
        };
        if let Some(r) = r {
            return Ok(r);
        }
    }
    let (x, y) = (a.f(), b.f());
    check(match op {
        Bin::Add => x + y,
        Bin::Sub => x - y,
        Bin::Mul => x * y,
        Bin::Div => {
            if y == 0.0 {
                return Err(CalcError::Math);
            }
            x / y
        }
        _ => unreachable!(),
    })
}

fn power(a: Value, b: Value) -> CResult<Value> {
    if let (Value::R(..), Value::R(e, 1)) = (a, b) {
        if e.abs() <= 32 {
            let mut acc = Value::R(1, 1);
            let mut ok = true;
            for _ in 0..e.abs() {
                match binop_arith(Bin::Mul, acc, a) {
                    Ok(v @ Value::R(..)) => acc = v,
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                return if e >= 0 {
                    Ok(acc)
                } else {
                    binop_arith(Bin::Div, Value::R(1, 1), acc)
                };
            }
        }
    }
    let (x, y) = (a.f(), b.f());
    if x == 0.0 {
        return if y > 0.0 { Ok(Value::F(0.0)) } else { Err(CalcError::Math) };
    }
    if x < 0.0 && y.fract() != 0.0 {
        return Err(CalcError::Math);
    }
    check(x.powf(y))
}

fn as_int(v: Value) -> CResult<f64> {
    let x = v.f();
    if x.fract() != 0.0 || !(0.0..1e10).contains(&x) {
        return Err(CalcError::Math);
    }
    Ok(x)
}

pub fn binop(op: Bin, a: Value, b: Value, ctx: &mut Ctx) -> CResult<Value> {
    match op {
        Bin::Add | Bin::Sub | Bin::Mul | Bin::Div => binop_arith(op, a, b),
        Bin::Pow => power(a, b),
        Bin::Root => {
            let (deg, x) = (a.f(), b.f());
            if deg == 0.0 {
                return Err(CalcError::Math);
            }
            if x < 0.0 {
                let odd = deg.fract() == 0.0 && deg.rem_euclid(2.0) == 1.0;
                if !odd {
                    return Err(CalcError::Math);
                }
                return check(-(-x).powf(1.0 / deg));
            }
            if x == 0.0 {
                return if deg > 0.0 { Ok(Value::F(0.0)) } else { Err(CalcError::Math) };
            }
            check(x.powf(1.0 / deg))
        }
        Bin::NPr | Bin::NCr => {
            let (n, r) = (as_int(a)?, as_int(b)?);
            if r > n {
                return Err(CalcError::Math);
            }
            let k = if op == Bin::NCr { r.min(n - r) } else { r };
            let mut acc = 1.0f64;
            let mut i = 0.0;
            while i < k {
                acc *= n - i;
                if op == Bin::NCr {
                    acc /= i + 1.0;
                }
                if acc >= 1e100 {
                    return Err(CalcError::Math);
                }
                i += 1.0;
            }
            check(acc.round())
        }
        Bin::ToPol => {
            let (x, y) = (a.f(), b.f());
            let r = x.hypot(y);
            let th = ctx.angle.from_deg(y.atan2(x).to_degrees());
            ctx.y = Some(fix12(th));
            check(r)
        }
        Bin::ToRec => {
            let (r, th) = (a.f(), b.f());
            let (s, c) = match (
                func(Func::Sin, Value::F(th), ctx.angle)?,
                func(Func::Cos, Value::F(th), ctx.angle)?,
            ) {
                (s, c) => (s.f(), c.f()),
            };
            ctx.y = Some(fix12(r * s));
            check(r * c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: f64) -> Tok {
        Tok::Num(Value::F(x))
    }

    fn ev(t: &[Tok]) -> CResult<f64> {
        let mut c = Ctx { angle: Angle::Deg, y: None };
        evaluate(t, &mut c).map(|v| v.f())
    }

    #[test]
    fn priority() {
        use Bin::*;
        assert_eq!(ev(&[n(2.0), Tok::Bin(Add), n(3.0), Tok::Bin(Mul), n(4.0)]), Ok(14.0));
        assert_eq!(
            ev(&[Tok::LP, n(2.0), Tok::Bin(Add), n(3.0), Tok::RP, Tok::Bin(Mul), n(4.0)]),
            Ok(20.0)
        );
        // 2^3^2 = 512 (right-assoc)
        assert_eq!(ev(&[n(2.0), Tok::Bin(Pow), n(3.0), Tok::Bin(Pow), n(2.0)]), Ok(512.0));
    }

    #[test]
    fn trig_exact() {
        assert_eq!(ev(&[Tok::Pre(Func::Sin), n(30.0)]), Ok(0.5));
        assert_eq!(ev(&[Tok::Pre(Func::Sin), n(180.0)]), Ok(0.0));
        assert_eq!(ev(&[Tok::Pre(Func::Tan), n(90.0)]), Err(CalcError::Math));
        assert_eq!(ev(&[Tok::Pre(Func::Asin), n(1.0)]), Ok(90.0));
    }

    #[test]
    fn implicit_and_errors() {
        use Bin::*;
        assert_eq!(ev(&[n(2.0), Tok::Pi]).map(|x| (x * 1e6).round()), Ok(6283185.0));
        assert_eq!(ev(&[n(1.0), Tok::Bin(Div), n(0.0)]), Err(CalcError::Math));
        assert_eq!(ev(&[n(5.0), Tok::Bin(Mul)]), Err(CalcError::Syntax));
        assert_eq!(ev(&[n(5.0), Tok::Bin(NCr), n(2.0)]), Ok(10.0));
        assert_eq!(ev(&[n(5.0), Tok::Bin(NPr), n(2.0)]), Ok(20.0));
        assert_eq!(ev(&[n(5.0), Tok::Post(Post::Fact)]), Ok(120.0));
        assert_eq!(ev(&[n(3.0), Tok::Bin(Root), n(27.0)]), Ok(3.0));
    }

    #[test]
    fn fractions_stay_exact() {
        let t = [
            Tok::Num(Value::R(1, 2)),
            Tok::Bin(Bin::Add),
            Tok::Num(Value::R(1, 3)),
        ];
        let mut c = Ctx { angle: Angle::Deg, y: None };
        assert_eq!(evaluate(&t, &mut c), Ok(Value::R(5, 6)));
    }

    #[test]
    fn float_noise_removed() {
        use Bin::*;
        assert_eq!(ev(&[n(0.1), Tok::Bin(Add), n(0.2), Tok::Bin(Sub), n(0.3)]), Ok(0.0));
    }
}
