# Module: terminal.td

## Exports

- `IsTerminal`
- `TerminalSize`
- `ReadKey`
- `KeyKind`
- `RawModeEnter`
- `RawModeLeave`
- `ClearScreen`
- `ClearLine`
- `AltScreenEnter`
- `AltScreenLeave`
- `CursorMoveTo`
- `CursorHide`
- `CursorShow`
- `Stylize`
- `Color`
- `ResetStyle`

## Bindings

### KeyKind

> キー種別を表す列挙パック（28バリアント）

**Example**:

```taida
key <= ReadKey[]()
key.kind |== KeyKind.Enter => stdout("Enter pressed")
key.kind |== KeyKind.Char  => stdout(key.text)
```

**AI-Context**:
ReadKey の戻り値 `kind` フィールドと比較して使う。
タグ値（Int）は v1 ABI で凍結済み。追加・並び替えは ABI bump が必要。

### IsTerminal

> 指定ストリームが TTY かどうかを判定する

**Params**:
- `stream`: `"stdin"` | `"stdout"` | `"stderr"`

**Returns**: `Bool`

**Throws**:
- `IsTerminalInvalidStream`: stream が `"stdin"` / `"stdout"` / `"stderr"` 以外の場合

**Example**:

```taida
interactive <= IsTerminal[]("stdin")
stdout(interactive.toString())
```

**Since**: a.4

**AI-SideEffects**:
- `isatty` システムコールを発行する（読み取り専用、副作用なし）

### TerminalSize

> ターミナルのカラム数・行数を取得する

**Returns**: @(cols: Int, rows: Int) — 両方 >= 1

**Throws**:
- TerminalSizeNotATty: stdout が TTY でない場合
- TerminalSizeIoctl: ioctl(TIOCGWINSZ) が失敗した場合

**Example**:

```taida
size <= TerminalSize[]()
stdout(size.cols)
stdout(size.rows)
```

**Since**: a.1

**AI-SideEffects**:
- ioctl システムコールを発行する（読み取り専用、副作用なし）

### ReadKey

> キーボードから1キー分の入力を読み取る（raw モード）

**Returns**: @(kind: KeyKind, text: Str, ctrl: Bool, alt: Bool, shift: Bool)

**Throws**:
- ReadKeyNotATty: stdin が TTY でない場合
- ReadKeyRawMode: raw モードの開始/終了に失敗した場合
- ReadKeyEof: EOF を検出した場合
- ReadKeyInterrupted: シグナル割り込みが発生した場合

**Example**:

```taida
key <= ReadKey[]()
key.kind |== KeyKind.Escape => stdout("Escaped!")
```

**Since**: a.1

**AI-Context**:
ブロッキング呼び出し。1キー読み取り後に raw モードを解除して返る。

**AI-SideEffects**:
- stdin を一時的に raw モードに変更し、RAII で自動復元する
- standalone raw mode 中 (`RawModeEnter` 済み) は mode 操作をスキップし read のみ実行

### RawModeEnter

> stdin を raw モードに切り替える

**Returns**: `@()` (empty pack)

**Throws**:
- `RawModeNotATty`: stdin が TTY でない場合
- `RawModeAlreadyActive`: 既に raw モードの場合
- `RawModeEnterFailed`: termios 操作に失敗した場合

**Example**:

```taida
RawModeEnter[]()
key <= ReadKey[]()
RawModeLeave[]()
```

**Since**: a.5

**AI-Context**:
TUI アプリでは `RawModeEnter` -> `ReadKey` x N -> `RawModeLeave` のパターンで使う。
raw モード中の `ReadKey` は自身の enter/leave をスキップする。

**AI-SideEffects**:
- stdin の termios を変更する。`RawModeLeave` で復元必須。

### RawModeLeave

> stdin を raw モードから復元する

**Returns**: `@()` (empty pack)

**Throws**:
- `RawModeNotActive`: raw モードでない状態で呼んだ場合
- `RawModeLeaveFailed`: termios 復元に失敗した場合

**Example**:

```taida
RawModeEnter[]()
key <= ReadKey[]()
RawModeLeave[]()
```

**Since**: a.5

**AI-SideEffects**:
- stdin の termios を復元する

### ClearScreen

> 画面全体をクリアし、カーソルを左上 (1,1) へ移動する ANSI シーケンスを返す

**Returns**: `Str` -- `"\x1b[2J\x1b[H"`

**Example**:

```taida
stdout(ClearScreen[]())
```

**Since**: a.5

### ClearLine

> 現在行をクリアし、カーソルを行頭へ移動する ANSI シーケンスを返す

**Returns**: `Str` -- `"\x1b[2K\r"`

**Example**:

```taida
stdout(ClearLine[]())
```

**Since**: a.5

### AltScreenEnter

> alternate screen buffer に切り替える ANSI シーケンスを返す

**Returns**: `Str` -- `"\x1b[?1049h"`

**Example**:

```taida
stdout(AltScreenEnter[]())
```

**Since**: a.5

### AltScreenLeave

> main screen buffer に復帰する ANSI シーケンスを返す

**Returns**: `Str` -- `"\x1b[?1049l"`

**Example**:

```taida
stdout(AltScreenLeave[]())
```

**Since**: a.5

### CursorMoveTo

> カーソルを指定位置 (col, row) へ移動する ANSI シーケンスを返す

**Params**:
- `col`: `Int` -- 1-based カラム位置
- `row`: `Int` -- 1-based 行位置

**Returns**: `Str` -- `"\x1b[{row};{col}H"`

**Throws**:
- `CursorMoveInvalidPosition`: col < 1 または row < 1

**Example**:

```taida
stdout(CursorMoveTo[](10, 5))
```

**Since**: a.5

### CursorHide

> カーソルを非表示にする ANSI シーケンスを返す

**Returns**: `Str` -- `"\x1b[?25l"`

**Example**:

```taida
stdout(CursorHide[]())
```

**Since**: a.5

### CursorShow

> カーソルを表示する ANSI シーケンスを返す

**Returns**: `Str` -- `"\x1b[?25h"`

**Example**:

```taida
stdout(CursorShow[]())
```

**Since**: a.5

### Stylize

> テキストに色・装飾を適用した ANSI 文字列を返す

**Params**:
- `text`: `Str` -- 装飾するテキスト
- `opts`: `@(fg, bg, bold, dim, underline, italic)` -- スタイルオプション

**Returns**: `Str` -- `"\x1b[{codes}m{text}\x1b[0m"` (スタイル指定が空なら text そのまま)

**Throws**:
- `StylizeInvalidColor`: fg / bg に未知の色名が指定された場合

**Example**:

```taida
stdout(Stylize[]("hello", @(fg <= Color.red, bold <= true)))
```

**Since**: a.5

### Color

> 基本 16 色のカラーパレット

**Fields**:
`black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`,
`bright_black`, `bright_red`, `bright_green`, `bright_yellow`,
`bright_blue`, `bright_magenta`, `bright_cyan`, `bright_white`

**Example**:

```taida
stdout(Stylize[]("error", @(fg <= Color.red)))
stdout(Stylize[]("ok", @(fg <= Color.green, bold <= true)))
```

**Since**: a.5

### ResetStyle

> 全スタイルをリセットする ANSI シーケンスを返す

**Returns**: `Str` -- `"\x1b[0m"`

**Example**:

```taida
stdout(ResetStyle[]())
```

**Since**: a.5
