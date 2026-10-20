//! Number → LCD cell formatting (Norm / Fix / Sci / ENG / fractions / DMS).
//!
//! Cell strings use `0-9`, `.` (attached to the previous digit cell),
//! `J` (fraction separator) and `°` (D°M′S″ separator).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispFmt {
    /// Norm 1 or Norm 2.
    Norm(u8),
    /// Fix n decimals.
    Fix(u8),
    /// Sci n significant digits (0 = 10).
    Sci(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Formatted {
    pub neg: bool,
    pub cells: String,
    pub exp: Option<i32>,
}

const MAX_CELLS: usize = 10;

/// `n` significant digits of |x| and the decimal exponent of the first digit.
pub fn digits_exp(x: f64, n: usize) -> (String, i32) {
    let s = format!("{:.*e}", n - 1, x.abs());
    let (m, e) = s.split_once('e').unwrap();
    (m.replace('.', ""), e.parse().unwrap())
}

fn strip_zeros(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Lay out `d1.d2d3… × 10^exp` as a plain decimal string.
fn plain(digits: &str, exp: i32) -> String {
    let l = digits.len() as i32;
    let s = if exp >= 0 {
        let int_len = exp + 1;
        if l <= int_len {
            format!("{}{}", digits, "0".repeat((int_len - l) as usize))
        } else {
            format!("{}.{}", &digits[..int_len as usize], &digits[int_len as usize..])
        }
    } else {
        format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
    };
    strip_zeros(s)
}

fn cell_count(s: &str) -> usize {
    s.chars().filter(|&c| c != '.').count()
}

fn limit_cells(s: String) -> String {
    let mut out = String::new();
    let mut n = 0;
    for c in s.chars() {
        if c != '.' {
            if n == MAX_CELLS {
                break;
            }
            n += 1;
        }
        out.push(c);
    }
    out
}

fn sci_mantissa(digits: &str, keep_zeros: bool) -> String {
    let s = if digits.len() > 1 {
        format!("{}.{}", &digits[..1], &digits[1..])
    } else {
        digits.to_string()
    };
    if keep_zeros {
        s
    } else {
        strip_zeros(s)
    }
}

pub fn format_f64(x: f64, fmt: DispFmt) -> Formatted {
    let neg = x < 0.0;
    if x == 0.0 {
        return match fmt {
            DispFmt::Norm(_) => Formatted { neg: false, cells: "0".into(), exp: None },
            DispFmt::Fix(n) => Formatted {
                neg: false,
                cells: if n == 0 { "0".into() } else { format!("0.{}", "0".repeat(n as usize)) },
                exp: None,
            },
            DispFmt::Sci(n) => {
                let sig = if n == 0 { 10 } else { n as usize };
                Formatted { neg: false, cells: sci_mantissa(&"0".repeat(sig), true), exp: Some(0) }
            }
        };
    }
    match fmt {
        DispFmt::Norm(k) => norm(x, neg, k),
        DispFmt::Sci(n) => {
            let sig = if n == 0 { 10 } else { n as usize };
            let (d, e) = digits_exp(x, sig);
            Formatted { neg, cells: sci_mantissa(&d, true), exp: Some(e) }
        }
        DispFmt::Fix(n) => {
            let s = format!("{:.*}", n as usize, x.abs());
            if cell_count(&s) > MAX_CELLS {
                return norm_sci(x, neg);
            }
            let zero = s.chars().all(|c| c == '0' || c == '.');
            Formatted { neg: neg && !zero, cells: s, exp: None }
        }
    }
}

fn norm_sci(x: f64, neg: bool) -> Formatted {
    let (d, e) = digits_exp(x, 10);
    Formatted { neg, cells: sci_mantissa(&d, false), exp: Some(e) }
}

fn norm(x: f64, neg: bool, k: u8) -> Formatted {
    let (d, e) = digits_exp(x, 10);
    let low = if k == 2 { -9 } else { -2 };
    if e < low || e >= 10 {
        norm_sci(x, neg)
    } else {
        Formatted { neg, cells: plain(&d, e), exp: None }
    }
}

/// Engineering display: mantissa × 10^e where e is a multiple of 3.
pub fn format_eng(x: f64, e: i32) -> Formatted {
    let (d, x10) = digits_exp(x, 10);
    Formatted {
        neg: x < 0.0,
        cells: limit_cells(plain(&d, x10 - e)),
        exp: Some(e),
    }
}

/// Round `x` the way the display would (RND key).
pub fn round_to_format(x: f64, fmt: DispFmt) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let s = match fmt {
        DispFmt::Norm(_) => format!("{:.9e}", x),
        DispFmt::Fix(n) => format!("{:.*}", n as usize, x),
        DispFmt::Sci(n) => format!("{:.*e}", if n == 0 { 9 } else { n as usize - 1 }, x),
    };
    s.parse().unwrap_or(x)
}

/// Fraction `n/d` (d > 1) as `w J n J d` or improper `n J d`; `None` if > 10 cells.
pub fn format_frac(n: i64, d: i64, improper: bool) -> Option<Formatted> {
    let a = n.unsigned_abs();
    let d = d as u64;
    let cells = if improper || a < d {
        format!("{a}J{d}")
    } else {
        format!("{}J{}J{}", a / d, a % d, d)
    };
    (cell_count(&cells) <= MAX_CELLS).then_some(Formatted { neg: n < 0, cells, exp: None })
}

/// Degrees / minutes / seconds display.
pub fn format_dms(x: f64) -> Formatted {
    let a = x.abs();
    for dec in [2usize, 1, 0] {
        let scale = 10f64.powi(dec as i32);
        let ts = (a * 3600.0 * scale).round() / scale;
        let d = (ts / 3600.0).floor();
        let rem = ts - d * 3600.0;
        let m = (rem / 60.0).floor();
        let s = rem - m * 60.0;
        let s_txt = strip_zeros(format!("{:.*}", dec, s));
        let cells = format!("{}°{}°{}°", d, m, s_txt);
        if cell_count(&cells) <= MAX_CELLS || dec == 0 {
            return Formatted { neg: x < 0.0, cells: limit_cells(cells), exp: None };
        }
    }
    unreachable!()
}

/// Approximate a decimal by a fraction with denominator ≤ 9999.
pub fn to_fraction(x: f64) -> Option<(i64, i64)> {
    let a = x.abs();
    if !x.is_finite() || a >= 1e9 {
        return None;
    }
    if a == 0.0 {
        return Some((0, 1));
    }
    let (mut h0, mut k0, mut h1, mut k1) = (0i64, 1i64, 1i64, 0i64);
    let mut r = a;
    for _ in 0..30 {
        let ai = r.floor();
        let (h2, k2) = (ai as i64 * h1 + h0, ai as i64 * k1 + k0);
        if k2 > 9999 {
            break;
        }
        (h0, k0, h1, k1) = (h1, k1, h2, k2);
        let frac = r - ai;
        if frac < 1e-12 {
            break;
        }
        r = 1.0 / frac;
    }
    if k1 == 0 || (a - h1 as f64 / k1 as f64).abs() > 1e-9 * a {
        return None;
    }
    Some((if x < 0.0 { -h1 } else { h1 }, k1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(f: Formatted) -> String {
        format!(
            "{}{}{}",
            if f.neg { "-" } else { "" },
            f.cells,
            f.exp.map(|e| format!(" e{e}")).unwrap_or_default()
        )
    }

    #[test]
    fn norm_format() {
        let n = DispFmt::Norm(1);
        assert_eq!(s(format_f64(1.0 / 3.0, n)), "0.3333333333");
        assert_eq!(s(format_f64(14.0, n)), "14");
        assert_eq!(s(format_f64(-1500.0, n)), "-1500");
        assert_eq!(s(format_f64(1e10, n)), "1 e10");
        assert_eq!(s(format_f64(0.001234, n)), "1.234 e-3");
        assert_eq!(s(format_f64(0.001234, DispFmt::Norm(2))), "0.001234");
        assert_eq!(s(format_f64(1234567.891234, n)), "1234567.891");
    }

    #[test]
    fn fix_sci() {
        assert_eq!(s(format_f64(2.0 / 3.0, DispFmt::Fix(2))), "0.67");
        assert_eq!(s(format_f64(2.0, DispFmt::Fix(2))), "2.00");
        assert_eq!(s(format_f64(12345.678, DispFmt::Sci(3))), "1.23 e4");
        assert_eq!(s(format_f64(12345.678, DispFmt::Sci(0))), "1.234567800 e4");
    }

    #[test]
    fn eng_frac_dms() {
        assert_eq!(s(format_eng(12345.0, 3)), "12.345 e3");
        assert_eq!(s(format_eng(12345.0, 0)), "12345 e0");
        assert_eq!(s(format_eng(12345.0, 6)), "0.012345 e6");
        assert_eq!(s(format_frac(11, 4, false).unwrap()), "2J3J4");
        assert_eq!(s(format_frac(11, 4, true).unwrap()), "11J4");
        assert_eq!(s(format_dms(12.5125)), "12°30°45°");
        assert_eq!(to_fraction(0.75), Some((3, 4)));
        assert_eq!(to_fraction(std::f64::consts::PI), None);
    }
}
