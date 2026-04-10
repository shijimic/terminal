// smoke_test.td -- Facade smoke test for taida-lang/terminal
//
// This file is loaded by the Taida interpreter to verify that all
// facade .td files parse correctly, exports are resolvable, and
// basic function calls produce the expected return values.
//
// Run: taida examples/smoke_test.td
//   (requires the package to be installed or symlinked into .taida/deps/)

>>> taida-lang/terminal => @(KeyKind, Color, ResetStyle, ClearScreen, ClearLine, AltScreenEnter, AltScreenLeave, CursorMoveTo, CursorHide, CursorShow, MouseTrackingEnter, MouseTrackingLeave, Stylize, Color256, ColorRgb, Stylize256, StylizeRgb, WidthMode, MeasureGrapheme, DisplayWidth, NormalizeCellText, TruncateWidth, PadWidth, Cell, ScreenBuffer, DiffOpKind, DiffOp, BufferNew, PromptMode, PromptOptions, CompletionState, LineEditorAction, LineEditorState, LineEditorNew, LineEditorStep, LineEditorRender, SpinnerState, SpinnerNext, SpinnerRender, ProgressOptions, ProgressBar, StatusLine, EventKind, MouseKind)

// -- Type packs --
stdout("KeyKind.Enter:" + KeyKind.Enter.toString())
stdout("KeyKind.Unknown:" + KeyKind.Unknown.toString())
stdout("Color.red:" + Color.red)
stdout("Color.bright_white:" + Color.bright_white)
stdout("WidthMode.Wide:" + WidthMode.Wide.toString())
stdout("WidthMode.Zero:" + WidthMode.Zero.toString())
stdout("DiffOpKind.MoveTo:" + DiffOpKind.MoveTo.toString())
stdout("PromptMode.Password:" + PromptMode.Password.toString())
stdout("LineEditorAction.Submitted:" + LineEditorAction.Submitted.toString())
stdout("LineEditorAction.Cancelled:" + LineEditorAction.Cancelled.toString())
stdout("EventKind.Resize:" + EventKind.Resize.toString())
stdout("MouseKind.ScrollDown:" + MouseKind.ScrollDown.toString())

// -- ANSI facade functions --
stdout("ResetStyle.len:" + ResetStyle().length().toString())
stdout("ClearScreen.len:" + ClearScreen().length().toString())
stdout("ClearLine.len:" + ClearLine().length().toString())
stdout("AltScreenEnter.len:" + AltScreenEnter().length().toString())
stdout("AltScreenLeave.len:" + AltScreenLeave().length().toString())
stdout("CursorMoveTo.len:" + CursorMoveTo(10, 5).length().toString())
stdout("CursorHide.len:" + CursorHide().length().toString())
stdout("CursorShow.len:" + CursorShow().length().toString())
stdout("MouseTrackingEnter.len:" + MouseTrackingEnter().length().toString())
stdout("MouseTrackingLeave.len:" + MouseTrackingLeave().length().toString())

// -- Width functions --
stdout("MeasureGrapheme.A:" + MeasureGrapheme("A").width.toString())
stdout("DisplayWidth.hello:" + DisplayWidth("hello").toString())
stdout("DisplayWidth.empty:" + DisplayWidth("").toString())
stdout("NormalizeCellText.empty:" + NormalizeCellText("").length().toString())
stdout("TruncateWidth.hello3:" + TruncateWidth("hello", 3))
stdout("PadWidth.hi5:" + PadWidth("hi", 5).length().toString())

// -- Stylize --
s1 <= Stylize("x", @(fg <= "red", bg <= "", bold <= false, dim <= false, underline <= false, italic <= false))
stdout("Stylize.red.len:" + s1.length().toString())
s2 <= Stylize("plain", @(fg <= "", bg <= "", bold <= false, dim <= false, underline <= false, italic <= false))
stdout("Stylize.plain:" + s2)

// -- Line editor --
state <= LineEditorNew(@(prompt <= "> ", initial <= "hello", placeholder <= "", mode <= 0, history <= @["old"], completion <= @()))
stdout("LineEditorNew.text:" + state.text)
stdout("LineEditorNew.cursor:" + state.cursor.toString())

// -- Spinner --
sp <= SpinnerState
stdout("SpinnerState.frame:" + sp.frame.toString())
sp2 <= SpinnerNext(sp)
stdout("SpinnerNext.frame:" + sp2.frame.toString())
sr <= SpinnerRender(sp)
stdout("SpinnerRender.len:" + sr.length().toString())

// -- Progress bar --
pb <= ProgressBar(50, 100, ProgressOptions)
stdout("ProgressBar.len:" + pb.length().toString())

// -- Status line --
sl <= StatusLine("left", "right", 20)
stdout("StatusLine.len:" + sl.length().toString())

stdout("PASS:all_smoke_tests")
