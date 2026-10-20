# Casio fx-82 SOLAR II Clone — Requirements

Status: draft v1 · Source of truth for the key legends: `CasioFront.jpg`

## 1. Goal and scope

A desktop clone of the Casio **fx-82 SOLAR II** scientific calculator, written in **Rust** with **egui/eframe**, running on **Windows, Linux and macOS** from one code base.

The clone reproduces the look (LCD, indicators, key layout, SHIFT legends) and the behaviour of the one-line, non-"natural-display" fx-82 family: numbers are entered in algebraic (infix) order, the LCD shows one number at a time (10 digits + 2-digit exponent), and the expression is evaluated with operator priorities when `=` is pressed.

Out of scope: solar-cell simulation, battery behaviour, sound, formula replay (there are no cursor keys), regression modes, base-n, complex numbers, physical-accuracy of the 12-digit BCD engine beyond what is stated in §6.

## 2. Platforms and non-functional requirements

| ID | Requirement |
|----|-------------|
| NF-1 | Builds with stable Rust (edition 2021) via `cargo build --release` on Windows 10/11, Linux (X11 and Wayland) and macOS (Intel and Apple Silicon). |
| NF-2 | No platform-specific code in the application logic; only `eframe`/`egui` is used for windowing and rendering. |
| NF-3 | The calculator engine (parsing, evaluation, formatting, state machine) is UI-independent, lives in its own modules and is covered by unit tests. |
| NF-4 | The UI is drawn in a fixed design coordinate space and scaled uniformly to the window, so the calculator keeps its aspect ratio at any window size. |
| NF-5 | Every key can be clicked with the mouse; the common keys also work from the physical keyboard (§9). |
| NF-6 | Start-up in < 1 s, key press → LCD update within one frame. |
| NF-7 | CI builds and tests on all three operating systems. |

## 3. Display (LCD)

| ID | Requirement |
|----|-------------|
| D-1 | Seven-segment style LCD showing a **sign cell + 10 digit cells** (decimal point attached to a digit cell) and a **2-digit exponent** (with its own minus sign) at the right, smaller and raised. |
| D-2 | Annunciators along the top: `SHIFT`, `M`, `K`, `HYP`, `DEG`/`RAD`/`GRA` (exactly one lit), `FIX`, `SCI`, `SD`. Unlit annunciators are drawn faintly, as on the real LCD. |
| D-3 | `SHIFT` lit after pressing SHIFT until the next key. `M` lit while the independent memory ≠ 0. `K` lit while the constant-repeat function is used (§7.6). `HYP` lit after pressing `hyp` until the next function key. `SCI` lit in Sci mode, `FIX` in Fix mode, `SD` in statistics mode. |
| D-4 | Numbers being typed are shown as typed (leading `-`, `.`, `E` exponent, fraction separators, degree separators). |
| D-5 | Errors replace the number with `Math ERROR`, `Syntax ERROR` or `Stack ERROR`; only `AC`, `C` or `ON` leave the error state. |
| D-6 | MODE menus are shown as text on the LCD (§5). |
| D-7 | After 10 minutes without input the display switches off (blank); `ON` wakes it up and all state (memory, statistics, settings) is retained. |

## 4. Key map

Layout is 3 rows of 6 small keys followed by 4 rows of 5 large keys. "Shift" is the function printed above the key (gold legend), reached by `SHIFT` then the key.

| Row | Key | Primary | SHIFT |
|-----|-----|---------|-------|
| 1 | SHIFT | arm shift | — |
| 1 | MODE | mode / setup menu | — |
| 1 | x² | square | √ square root |
| 1 | log | log₁₀ | 10ˣ |
| 1 | ln | ln | eˣ |
| 1 | ON | wake / clear | — |
| 2 | a b/c | enter fraction / toggle fraction ⇄ decimal | d/c (mixed ⇄ improper) |
| 2 | ° ' " | enter / convert degrees-minutes-seconds | ← (delete last typed character) |
| 2 | hyp | hyperbolic prefix | — |
| 2 | sin | sin | sin⁻¹ |
| 2 | cos | cos | cos⁻¹ |
| 2 | tan | tan | tan⁻¹ |
| 3 | +/− | change sign | ∛ cube root |
| 3 | ► | DRG► angle-unit conversion menu (see A-1) | x³ cube |
| 3 | ( | open parenthesis | X↔Y swap displayed value with Y register |
| 3 | ) | close parenthesis | 1/x reciprocal |
| 3 | xʸ | power | ˣ√y x-th root (degree first) |
| 3 | MR | recall memory M | Min store into M |
| 4 | 7 | digit | x̄ mean |
| 4 | 8 | digit | σn population standard deviation |
| 4 | 9 | digit | σn−1 sample standard deviation |
| 4 | C | clear last entry / error | X↔M swap displayed value with M |
| 4 | AC | clear all (expression, display) | SAC clear statistics data |
| 5 | 4 | digit | Σx² |
| 5 | 5 | digit | Σx |
| 5 | 6 | digit | n (number of data) |
| 5 | × | multiply | ←ENG engineering notation, exponent −3 |
| 5 | ÷ | divide | ENG engineering notation, exponent +3 |
| 6 | 1 | digit | nPr |
| 6 | 2 | digit | nCr |
| 6 | 3 | digit | x! |
| 6 | + | add | R→P rectangular to polar |
| 6 | − | subtract | P→R polar to rectangular |
| 7 | 0 | digit | RND round to display format |
| 7 | • | decimal point | RAN# random number 0.000–0.999 |
| 7 | EXP | enter ×10ⁿ exponent | π |
| 7 | = | evaluate | % percent |
| 7 | M+ | add to memory M (in SD mode: **DATA** entry) | M− subtract from M (in SD mode: **DEL** delete last data) |

Colour scheme: dark keys, the `C` and `AC` keys blue, shift legends gold, white labels for primary functions, white/grey bezel, solar cell strip above the LCD, "CASIO fx-82 SOLAR II" branding.

### Interpretation notes (not unambiguous in the photo)

* **A-1** `►` has no primary legend. It is implemented as the classic Casio `DRG►`: a small menu `1:DEG 2:RAD 3:GRA` converts the displayed value from the current angle unit to the chosen one (the angle *mode* is not changed).
* **A-2** `SAC` (SHIFT+AC) is implemented as "clear statistics data" (Scl).
* **A-3** SHIFT+C `X↔M` swaps the displayed value with the memory M.
* **A-4** The `6` under the parenthesis keys is the nesting depth: at most **6 levels** of open parentheses.
* **A-5** `ENG` legends on `×` and `÷` are the ENG / ←ENG pair.

## 5. Modes and setup (MODE key)

Pressing `MODE` opens a paged menu on the LCD; pressing `MODE` again goes to the next page (wraps), a digit key selects an entry, `AC` leaves the menu without change.

| Page | Entries |
|------|---------|
| 1 | `1:COMP` normal calculation · `2:SD` single-variable statistics (clears data) |
| 2 | Angle unit `1:Deg` · `2:Rad` · `3:Gra` |
| 3 | Display format `1:Fix` · `2:Sci` · `3:Norm` |
| 4 | Clear `1:Mcl` memory M · `2:Mode` reset all settings to default · `3:All` memory, statistics, settings |

| ID | Requirement |
|----|-------------|
| M-1 | Selecting **Fix** asks for the number of decimal places 0–9; **Sci** asks for significant digits 1–9, `0` = 10; **Norm** asks for `1` or `2`. |
| M-2 | Defaults after reset: COMP, Deg, Norm 1. |
| M-3 | Changing mode clears the pending expression. |

## 6. Number representation and display formats

| ID | Requirement |
|----|-------------|
| N-1 | Entry: up to 10 mantissa digits, optional decimal point, optional `EXP` exponent of 2 digits (−99…99, sign via `+/−` while in exponent). Surplus digits are ignored. |
| N-2 | Result range ±1×10⁻⁹⁹ … ±9.999999999×10⁹⁹. Overflow → `Math ERROR`; underflow → 0. |
| N-3 | Internal results are rounded to 12 significant digits after each operation (so `0.1+0.2−0.3` is `0` and `sin 180` is exactly `0` in Deg). |
| N-4 | **Norm 1**: scientific display when the value is < 10⁻² or ≥ 10¹⁰. **Norm 2**: when < 10⁻⁹ or ≥ 10¹⁰. Mantissa is rounded to 10 digits; trailing zeros are dropped. |
| N-5 | **Fix n**: n decimals, trailing zeros kept; falls back to scientific if the integer part would not fit. |
| N-6 | **Sci n**: n significant digits (0 → 10), mantissa + exponent. |
| N-7 | **ENG**: re-displays the current value as mantissa × 10^(multiple of 3); repeated presses move the exponent by 3; any other key cancels the shifted display. |
| N-8 | **RND**: replaces the value by the value rounded according to the current display format. |
| N-9 | **RAN#**: pseudo-random number with 3 decimals, 0.000 ≤ r ≤ 0.999. |

### 6.1 Fractions (`a b/c`)

* **F-1** Typed as `a b/c` → `n⌐d` or `w⌐n⌐d` (mixed). The separator is drawn as the Casio "┘" glyph.
* **F-2** `+ − × ÷` on two fraction values gives a fraction result (reduced); integer powers of a fraction stay a fraction. Mixing a fraction with a decimal gives a decimal. `x²`, `x³`, `1/x` keep fractions.
* **F-3** A fraction result is shown as mixed number; SHIFT+`a b/c` (`d/c`) toggles mixed ⇄ improper. If the display (digits + separators) would exceed 10 cells the result is shown as a decimal.
* **F-4** Pressing `a b/c` on a displayed result converts fraction ⇄ decimal (a decimal is converted when a representation with a denominator ≤ 9999 matches).
* **F-5** Denominator 0 → `Math ERROR`.

### 6.2 Degrees–minutes–seconds (`° ' "`)

* **S-1** Typing `12 ° ' " 30 ° ' " 45 ° ' "` is a number (12°30′45″ = 12.5125). Used like any other number.
* **S-2** Pressing `° ' "` on a displayed result converts it to D°M′S″ display (max 10 cells, seconds rounded).

## 7. Calculation engine

### 7.1 Expression entry
Algebraic, infix. Prefix functions (`sin`, `√`, `log`, …) are pressed before their operand, postfix functions (`x²`, `x³`, `1/x`, `x!`, `%`) after it. Missing closing parentheses are closed automatically on `=`.

### 7.2 Operator priority (highest first)
1. Fractions, DMS
2. Prefix functions: sin cos tan (and inverses/hyperbolics), log, ln, 10ˣ, eˣ, √, ∛; postfix x², x³, x⁻¹, x!, %
3. `^` and `ˣ√` (right-associative)
4. Implicit multiplication before π, `(`, a function (`2π`, `2sin30`, `3(4+5)`)
5. nPr, nCr
6. `×`, `÷`
7. `+`, `−`
8. R→P, P→R

### 7.3 Functions

| ID | Function | Notes |
|----|----------|-------|
| E-1 | `sin cos tan` | Angle unit per mode; exact results at multiples of 90°; `tan 90` → Math ERROR |
| E-2 | `sin⁻¹ cos⁻¹ tan⁻¹` | Result in current angle unit; domain \|x\| ≤ 1 for sin⁻¹/cos⁻¹ |
| E-3 | `hyp` + `sin cos tan` and with SHIFT | sinh cosh tanh asinh acosh atanh; domains checked (acosh x ≥ 1, atanh \|x\| < 1) |
| E-4 | `log`, `ln` | x > 0 |
| E-5 | `10ˣ`, `eˣ` | overflow → Math ERROR |
| E-6 | `√`, `∛`, `x²`, `x³`, `1/x` | √ needs x ≥ 0; 1/0 → Math ERROR; ∛ accepts negatives |
| E-7 | `xʸ`, `ˣ√y` | 0⁰ and 0^negative → Math ERROR; negative base needs integer exponent; odd integer root of negative allowed |
| E-8 | `x!` | integer 0…69 |
| E-9 | `nPr`, `nCr` | integers, 0 ≤ r ≤ n < 10¹⁰ |
| E-10 | `%` | postfix, divides by 100 (`200×15%` = 30) |
| E-11 | `π` | 3.14159265358979 |
| E-12 | `R→P` / `P→R` | `x R→P y =` shows r, `P→R`: `r P→R θ =` shows x; the second result (θ resp. y) goes to the Y register and is shown with SHIFT `X↔Y` |
| E-13 | `Pol/Rec angle` | θ uses the current angle unit |

### 7.4 Results and chaining
* `=` stores the result as **Ans**. Pressing a binary operator right after `=` continues with Ans; pressing a number or function starts a new calculation.
* A result is shown until the next key press.

### 7.5 Parentheses
Up to 6 nested levels; more → `Stack ERROR`.

### 7.6 Constant repeat (`K`)
After a simple `a ∘ b =` (∘ ∈ + − × ÷ ^), pressing `=` again repeats `∘ b` on the previous result; `K` is lit while this is used.

### 7.7 Errors
`Math ERROR` – division by zero, domain error, overflow. `Syntax ERROR` – malformed expression (e.g. `5 × =`, operator without operand). `Stack ERROR` – more than 6 parenthesis levels (or expression > 79 steps).
`AC`/`C`/`ON` clear the error.

## 8. Memory and statistics

| ID | Requirement |
|----|-------------|
| MEM-1 | One independent memory **M** (COMP mode): `M+` adds the displayed value / evaluated expression, SHIFT+`M+` (`M−`) subtracts, `MR` recalls, SHIFT+`MR` (`Min`) stores. Persists through AC, ON, mode change; cleared with `MODE → Clr → Mcl`. |
| MEM-2 | Y register used by R→P / P→R and `X↔Y`. |
| STAT-1 | **SD mode**: `x DATA` (`M+`) stores a data point and shows `n`. `x × f DATA` stores `x` with integer frequency f (1…99). SHIFT+DATA (`DEL`) removes the last entered data point. |
| STAT-2 | Statistics values (SHIFT+): `x̄` (7), `σn` (8), `σn−1` (9), `Σx²` (4), `Σx` (5), `n` (6). Only available in SD mode; usable inside expressions. `σn−1` with n = 1, or any value with n = 0 where undefined → Math ERROR. |
| STAT-3 | SHIFT+AC (`SAC`) clears the data. Entering SD mode via MODE clears the data. |

## 9. Keyboard shortcuts (convenience, not on the real device)

`0-9 . + - * / ( ) ^ =`, `Enter` → `=`, `Backspace` → `←` (SHIFT+`° ' "`), `Esc`/`Delete` → `AC`, `e` → `EXP`, `_` → `+/−`, `p` → π, `s c t` → sin cos tan, `h` → hyp, `l` → log, `n` → ln, `q` → x², `r` → √, `f` → `a b/c`, `d` → `° ' "`, `!` → x!, `%` → %, `m` → MODE, `S` → SHIFT.

## 10. Acceptance criteria (excerpt)

| Input | Expected |
|-------|----------|
| `2 + 3 × 4 =` | `14` |
| `( 2 + 3 ) × 4 =` | `20` |
| `1 ÷ 3 =` | `0.3333333333` |
| `2 xʸ 10 =` | `1024` |
| `3 SHIFT ˣ√y 27 =` | `3` |
| `sin 30 =` (Deg) | `0.5` |
| `SHIFT sin⁻¹ 1 =` (Deg) | `90` |
| `SHIFT √ 144 =` | `12` |
| `5 SHIFT nCr 2 =` | `10`, `5 SHIFT nPr 2 =` → `20` |
| `5 SHIFT x! =` | `120` |
| `1 a b/c 2 + 1 a b/c 3 =` | `5⌐6` |
| `2 a b/c 1 a b/c 4 =` then SHIFT d/c | `2⌐1⌐4` → `9⌐4` |
| `1 ÷ 0 =` | `Math ERROR` |
| `1.5 EXP 3 =` | `1500` |
| `1 EXP 10 =` | `10000000000` is too wide → mantissa `1` with exponent field `10` |
| `12345 SHIFT ÷` (ENG) | `12.345` with exponent `03` |
| `3 SHIFT R→P 4 =` | `5`, then SHIFT `X↔Y` → `53.13010235` (Deg) |
| `MODE MODE MODE 1 2` then `2 ÷ 3 =` (Fix, 2 decimals) | `0.67` |
| SD: `10 DATA 20 DATA 30 DATA`, SHIFT `x̄` | `20`, SHIFT `σn−1` → `10` |
| `200 × 15 SHIFT % ` | `30` |
