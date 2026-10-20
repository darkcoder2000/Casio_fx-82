# Casio fx-82 SOLAR II clone

A Rust + [egui](https://github.com/emilk/egui) clone of the Casio fx-82 SOLAR II scientific calculator
(Windows, Linux, macOS). See [REQUIREMENTS.md](REQUIREMENTS.md) for the feature list and key map.

```
cargo run --release
cargo test
```

Linux build dependencies (Debian/Ubuntu): `libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev libgl1-mesa-dev`.

Layout: `src/eval.rs` (values, priority parser, functions), `src/format.rs` (Norm/Fix/Sci/ENG/fraction/DMS formatting),
`src/calc.rs` (key state machine, modes, memory, statistics), `src/ui.rs` (LCD + keypad drawing, keyboard input).
Click the keys or use the keyboard shortcuts listed in the requirements (section 9).
