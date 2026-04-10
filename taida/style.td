// taida/style.td
//
// ANSI styling helpers -- Stylize, Color, ResetStyle.
//
// This is Layer B (ANSI facade) per TM_DESIGN.md:
//   - No syscalls
//   - No hidden side effects
//   - Returns ANSI escape strings only
//
// -- Color Palette (TM-2h) --

///@ Purpose: Basic 16-color palette (name constants)
///@ AI-Context: Pass to Stylize fg / bg arguments.
///@ Example:
///@   stdout(Stylize("hello", @(fg <= Color.red)))
///@ Since: a.5
Color <= @(
  black          <= "black"
  red            <= "red"
  green          <= "green"
  yellow         <= "yellow"
  blue           <= "blue"
  magenta        <= "magenta"
  cyan           <= "cyan"
  white          <= "white"
  bright_black   <= "bright_black"
  bright_red     <= "bright_red"
  bright_green   <= "bright_green"
  bright_yellow  <= "bright_yellow"
  bright_blue    <= "bright_blue"
  bright_magenta <= "bright_magenta"
  bright_cyan    <= "bright_cyan"
  bright_white   <= "bright_white"
)

// -- Internal: color name to SGR code mapping --

_fgCode name =
  | name == "black"          |> "30"
  | name == "red"            |> "31"
  | name == "green"          |> "32"
  | name == "yellow"         |> "33"
  | name == "blue"           |> "34"
  | name == "magenta"        |> "35"
  | name == "cyan"           |> "36"
  | name == "white"          |> "37"
  | name == "bright_black"   |> "90"
  | name == "bright_red"     |> "91"
  | name == "bright_green"   |> "92"
  | name == "bright_yellow"  |> "93"
  | name == "bright_blue"    |> "94"
  | name == "bright_magenta" |> "95"
  | name == "bright_cyan"    |> "96"
  | name == "bright_white"   |> "97"
  | _ |> Error(type <= "StylizeInvalidColor", message <= "unknown foreground color: " + name).throw()
=> :Str

_bgCode name =
  | name == "black"          |> "40"
  | name == "red"            |> "41"
  | name == "green"          |> "42"
  | name == "yellow"         |> "43"
  | name == "blue"           |> "44"
  | name == "magenta"        |> "45"
  | name == "cyan"           |> "46"
  | name == "white"          |> "47"
  | name == "bright_black"   |> "100"
  | name == "bright_red"     |> "101"
  | name == "bright_green"   |> "102"
  | name == "bright_yellow"  |> "103"
  | name == "bright_blue"    |> "104"
  | name == "bright_magenta" |> "105"
  | name == "bright_cyan"    |> "106"
  | name == "bright_white"   |> "107"
  | _ |> Error(type <= "StylizeInvalidColor", message <= "unknown background color: " + name).throw()
=> :Str

// Safe wrappers that guard against empty-string calls (avoids
// triggering error-throwing paths when no color is specified).
_safeFgCode name =
  | name == "" |> ""
  | _ |> _fgCode(name)
=> :Str

_safeBgCode name =
  | name == "" |> ""
  | _ |> _bgCode(name)
=> :Str

// Helper: join a code to the accumulator with ";" separator
_appendCode acc code =
  | acc == "" |> code
  | _ |> acc + ";" + code
=> :Str

// -- ResetStyle (TM-2h) --

///@ Purpose: Return ANSI reset sequence
///@ Returns: Str
///@ Example:
///@   stdout(ResetStyle())
///@ AI-SideEffects: none
///@ Since: a.5
ResetStyle =
  "\x1b[0m"
=> :Str

// -- Stylize (TM-2h) --

///@ Purpose: Apply color and decoration to text as ANSI string
///@ Params:
///@   text: Str
///@   opts: @(fg, bg, bold, dim, underline, italic)
///@ Returns: Str -- prefix + text + reset (or text as-is if no style)
///@ Throws:
///@   StylizeInvalidColor: unknown fg / bg color name
///@ Example:
///@   stdout(Stylize("hello", @(fg <= Color.red, bold <= true)))
///@ AI-SideEffects: none
///@ Since: a.5
Stylize text opts =
  // Pre-evaluate all function results to avoid the interpreter
  // condition-call premature-return limitation.
  _fgResult <= _safeFgCode(opts.fg)
  _bgResult <= _safeBgCode(opts.bg)
  _c1 <= _fgResult
  _c2pre <= _appendCode(_c1, _bgResult)
  _c2 <= | opts.bg == "" |> _c1
         | _ |> _c2pre
  _c3pre <= _appendCode(_c2, "1")
  _c3 <= | opts.bold == false |> _c2
         | _ |> _c3pre
  _c4pre <= _appendCode(_c3, "2")
  _c4 <= | opts.dim == false |> _c3
         | _ |> _c4pre
  _c5pre <= _appendCode(_c4, "3")
  _c5 <= | opts.italic == false |> _c4
         | _ |> _c5pre
  _c6pre <= _appendCode(_c5, "4")
  _c6 <= | opts.underline == false |> _c5
         | _ |> _c6pre
  | _c6 == "" |> text
  | _ |> "\x1b[" + _c6 + "m" + text + "\x1b[0m"
=> :Str

// -- Color256 / ColorRgb (TM-6e) --

///@ Purpose: 256-color index (0-255)
///@ AI-Context: Pass to Stylize256 fg / bg. index -1 means no color.
///@ Since: a.6
Color256 <= @(index <= 0)

///@ Purpose: RGB color (each component 0-255)
///@ AI-Context: Pass to StylizeRgb fg / bg. All -1 means no color.
///@ Since: a.6
ColorRgb <= @(r <= 0, g <= 0, b <= 0)

// -- Internal: 256-color SGR code builder --

_validate256 index label =
  | index == -1 |> ""
  | index < 0   |> Error(type <= "StylizeInvalidColor", message <= label + " color index out of range (0-255): " + index.toString()).throw()
  | index > 255 |> Error(type <= "StylizeInvalidColor", message <= label + " color index out of range (0-255): " + index.toString()).throw()
  | _ |> "ok"
=> :Str

_fg256Code index =
  "38;5;" + index.toString()
=> :Str

_bg256Code index =
  "48;5;" + index.toString()
=> :Str

_safeFg256Code index =
  | index == -1 |> ""
  | _ |> _fg256Code(index)
=> :Str

_safeBg256Code index =
  | index == -1 |> ""
  | _ |> _bg256Code(index)
=> :Str

// -- Internal: RGB SGR code builder --

_validateRgbComponent value label component =
  | value == -1 |> ""
  | value < 0   |> Error(type <= "StylizeInvalidColor", message <= label + " " + component + " out of range (0-255): " + value.toString()).throw()
  | value > 255 |> Error(type <= "StylizeInvalidColor", message <= label + " " + component + " out of range (0-255): " + value.toString()).throw()
  | _ |> "ok"
=> :Str

_isNoColorRgb color =
  | color.r == -1 |>
    | color.g == -1 |>
      | color.b == -1 |> true
      | _ |> false
    | _ |> false
  | _ |> false
=> :Bool

_fgRgbCode color =
  "38;2;" + color.r.toString() + ";" + color.g.toString() + ";" + color.b.toString()
=> :Str

_bgRgbCode color =
  "48;2;" + color.r.toString() + ";" + color.g.toString() + ";" + color.b.toString()
=> :Str

_safeFgRgbCode color =
  | _isNoColorRgb(color) == true |> ""
  | _ |> _fgRgbCode(color)
=> :Str

_safeBgRgbCode color =
  | _isNoColorRgb(color) == true |> ""
  | _ |> _bgRgbCode(color)
=> :Str

// Validate all RGB components for a color (skip if all -1)
_validateRgbFull color label =
  | _isNoColorRgb(color) == true |> ""
  | _ |> _validateRgbComponent(color.r, label, "r")
=> :Str

_validateRgbG color label prev =
  | _isNoColorRgb(color) == true |> ""
  | _ |> _validateRgbComponent(color.g, label, "g")
=> :Str

_validateRgbB color label prev =
  | _isNoColorRgb(color) == true |> ""
  | _ |> _validateRgbComponent(color.b, label, "b")
=> :Str

// -- Stylize256 (TM-6e) --

///@ Purpose: Apply 256-color styling to text
///@ Params:
///@   text: Str
///@   opts: @(fg <= Color256(index <= -1), bg <= Color256(index <= -1), bold, dim, underline, italic)
///@ Returns: Str
///@ Throws:
///@   StylizeInvalidColor: index out of 0-255 range
///@ Since: a.6
Stylize256 text opts =
  // Validate indices first (these throw on invalid values)
  _vfg <= _validate256(opts.fg.index, "foreground")
  _vbg <= _validate256(opts.bg.index, "background")
  // Pre-evaluate results
  _fgResult <= _safeFg256Code(opts.fg.index)
  _bgResult <= _safeBg256Code(opts.bg.index)
  _fgSkip <= opts.fg.index == -1
  _bgSkip <= opts.bg.index == -1
  // Build SGR parameter codes
  _c1 <= _fgResult
  _c2pre <= _appendCode(_c1, _bgResult)
  _c2 <= | _bgSkip == true |> _c1
         | _ |> _c2pre
  _c3pre <= _appendCode(_c2, "1")
  _c3 <= | opts.bold == false |> _c2
         | _ |> _c3pre
  _c4pre <= _appendCode(_c3, "2")
  _c4 <= | opts.dim == false |> _c3
         | _ |> _c4pre
  _c5pre <= _appendCode(_c4, "3")
  _c5 <= | opts.italic == false |> _c4
         | _ |> _c5pre
  _c6pre <= _appendCode(_c5, "4")
  _c6 <= | opts.underline == false |> _c5
         | _ |> _c6pre
  | _c6 == "" |> text
  | _ |> "\x1b[" + _c6 + "m" + text + "\x1b[0m"
=> :Str

// -- StylizeRgb (TM-6e) --

///@ Purpose: Apply RGB color styling to text
///@ Params:
///@   text: Str
///@   opts: @(fg <= ColorRgb(...), bg <= ColorRgb(...), bold, dim, underline, italic)
///@ Returns: Str
///@ Throws:
///@   StylizeInvalidColor: r/g/b out of 0-255 range
///@ Since: a.6
StylizeRgb text opts =
  // Validate RGB components
  _vfr <= _validateRgbFull(opts.fg, "foreground")
  _vfg2 <= _validateRgbG(opts.fg, "foreground", _vfr)
  _vfb <= _validateRgbB(opts.fg, "foreground", _vfg2)
  _vbr <= _validateRgbFull(opts.bg, "background")
  _vbg2 <= _validateRgbG(opts.bg, "background", _vbr)
  _vbb <= _validateRgbB(opts.bg, "background", _vbg2)
  // Pre-evaluate results
  _fgSkip <= _isNoColorRgb(opts.fg)
  _bgSkip <= _isNoColorRgb(opts.bg)
  _fgResult <= _safeFgRgbCode(opts.fg)
  _bgResult <= _safeBgRgbCode(opts.bg)
  // Build SGR parameter codes
  _c1 <= _fgResult
  _c2pre <= _appendCode(_c1, _bgResult)
  _c2 <= | _bgSkip == true |> _c1
         | _ |> _c2pre
  _c3pre <= _appendCode(_c2, "1")
  _c3 <= | opts.bold == false |> _c2
         | _ |> _c3pre
  _c4pre <= _appendCode(_c3, "2")
  _c4 <= | opts.dim == false |> _c3
         | _ |> _c4pre
  _c5pre <= _appendCode(_c4, "3")
  _c5 <= | opts.italic == false |> _c4
         | _ |> _c5pre
  _c6pre <= _appendCode(_c5, "4")
  _c6 <= | opts.underline == false |> _c5
         | _ |> _c6pre
  | _c6 == "" |> text
  | _ |> "\x1b[" + _c6 + "m" + text + "\x1b[0m"
=> :Str

<<< @(Color, ResetStyle, Stylize, Color256, ColorRgb, Stylize256, StylizeRgb)
