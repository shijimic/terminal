# taida-lang/terminal

Taida Lang official terminal package -- TTY detection, size query, key input, raw mode control, screen/cursor manipulation, and ANSI styling via a Rust native addon + pure Taida facade.

## Usage

```taida
>>> taida-lang/terminal => @(
  IsTerminal, TerminalSize, ReadKey, KeyKind,
  RawModeEnter, RawModeLeave,
  ClearScreen, ClearLine, AltScreenEnter, AltScreenLeave,
  CursorMoveTo, CursorHide, CursorShow,
  Stylize, Color, ResetStyle
)

interactive <= IsTerminal[]("stdin")
stdout(interactive.toString())

// Query terminal dimensions
size <= TerminalSize[]()
stdout(size.cols)
stdout(size.rows)

// Read a single key press (raw mode)
key <= ReadKey[]()
stdout(key.kind)
stdout(key.text)

// Persistent raw mode for TUI apps
RawModeEnter[]()
key <= ReadKey[]()
RawModeLeave[]()

// Screen/Cursor control (returns ANSI strings)
stdout(ClearScreen[]())
stdout(CursorMoveTo[](10, 5))
stdout(CursorHide[]())

// Styled text
stdout(Stylize[]("hello", @(fg <= Color.red, bold <= true)))
stdout(ResetStyle[]())
```

### Exports

| Symbol | Layer | Description |
|--------|-------|-------------|
| `IsTerminal` | addon | Returns `Bool` for `"stdin"`, `"stdout"`, or `"stderr"` |
| `TerminalSize` | addon | Returns `@(cols: Int, rows: Int)`, both >= 1 |
| `ReadKey` | addon | Returns `@(kind: KeyKind, text: Str, ctrl: Bool, alt: Bool, shift: Bool)` |
| `KeyKind` | pack | 28-variant enum: `Char`, `Enter`, `Escape`, `Tab`, `Backspace`, `Delete`, `ArrowUp`/`Down`/`Left`/`Right`, `Home`, `End`, `PageUp`/`Down`, `Insert`, `F1`-`F12`, `Unknown` |
| `RawModeEnter` | addon | Enter raw mode on stdin. Returns `@()` |
| `RawModeLeave` | addon | Leave raw mode on stdin. Returns `@()` |
| `ClearScreen` | facade | Returns `"\x1b[2J\x1b[H"` |
| `ClearLine` | facade | Returns `"\x1b[2K\r"` |
| `AltScreenEnter` | facade | Returns `"\x1b[?1049h"` |
| `AltScreenLeave` | facade | Returns `"\x1b[?1049l"` |
| `CursorMoveTo` | facade | Returns `"\x1b[{row};{col}H"` (1-based) |
| `CursorHide` | facade | Returns `"\x1b[?25l"` |
| `CursorShow` | facade | Returns `"\x1b[?25h"` |
| `Stylize` | facade | Returns `"\x1b[{codes}m{text}\x1b[0m"` or text as-is |
| `Color` | pack | 16-color palette (basic 8 + bright 8) |
| `ResetStyle` | facade | Returns `"\x1b[0m"` |

### Error variants

Functions throw deterministic Taida errors (no silent fallbacks):

- `IsTerminalInvalidStream`
- `ReadKeyNotATty` / `ReadKeyRawMode` / `ReadKeyEof` / `ReadKeyInterrupted`
- `TerminalSizeNotATty` / `TerminalSizeIoctl`
- `RawModeNotATty` / `RawModeAlreadyActive` / `RawModeNotActive` / `RawModeEnterFailed` / `RawModeLeaveFailed`
- `CursorMoveInvalidPosition`
- `StylizeInvalidColor`

## Development

### Prerequisites

- Rust toolchain (edition 2024)
- A local checkout of [taida-lang/taida](https://github.com/taida-lang/taida) (for `taida-addon` crate)

### Build

```bash
cargo build --release --lib
```

### Test

```bash
cargo test
```

### Local development with taida-addon override

The `taida-addon` dependency resolves from the `taida-lang/taida` git repository.
To use a local checkout instead, create `.cargo/config.toml`:

```toml
[patch."https://github.com/taida-lang/taida.git"]
taida-addon = { path = "../taida/crates/addon-rs" }
```

### Publish

```bash
taida publish --target rust-addon
```

## License

MIT
