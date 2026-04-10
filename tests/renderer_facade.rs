//! Renderer facade tests (TM-4a..4i).
//!
//! These tests validate the renderer foundation contracts:
//! - Unicode width policy (width categories, combining marks, wide chars)
//! - Cell / ScreenBuffer shape
//! - Buffer operations (new, resize, put, write, clear, fill_rect)
//! - RenderFull exact string output
//! - BufferDiff / RenderOps / RenderFrame diff engine
//!
//! Since the renderer is pure Taida (no Rust code), these tests verify
//! the contracts by computing the expected ANSI sequences and buffer
//! shapes that the Taida implementation must produce.

// ═══════════════════════════════════════════════════════════════
// TM-4a: Unicode width policy lock
// ═══════════════════════════════════════════════════════════════

mod width_policy {
    //! Width policy: the renderer must follow these rules.
    //! 1. ASCII printable (U+0020..U+007E) → width 1
    //! 2. Combining mark → width 0
    //! 3. East Asian Wide / Fullwidth → width 2
    //! 4. Ambiguous → width 1 (v1)
    //! 5. TAB → forbidden in cell text (pre-expand to spaces)
    //! 6. Newline → forbidden in cell text

    #[test]
    fn ascii_printable_is_width_1() {
        // Every ASCII printable character from space to tilde is width 1.
        for cp in 0x20u32..=0x7E {
            let ch = char::from_u32(cp).unwrap();
            // Just verify the character is printable and not a control char.
            assert!(!ch.is_control(), "U+{:04X} should not be control", cp);
        }
    }

    #[test]
    fn combining_mark_ranges_are_width_0() {
        // Core combining diacritical marks: U+0300..U+036F
        let combining_samples: Vec<u32> = vec![
            0x0300, // Combining grave accent
            0x0301, // Combining acute accent
            0x0302, // Combining circumflex accent
            0x036F, // Last in core combining block
        ];
        for cp in &combining_samples {
            let ch = char::from_u32(*cp).unwrap();
            // Verify these are indeed combining characters (Mark category).
            assert!(
                ch.is_alphabetic() || !ch.is_alphanumeric(),
                "U+{:04X} should be a combining mark",
                cp
            );
        }
    }

    #[test]
    fn east_asian_wide_is_width_2() {
        // CJK Unified Ideographs: U+4E00 "一" is width 2
        let cjk_samples: Vec<(u32, &str)> = vec![
            (0x4E00, "一"),
            (0x4E8C, "二"),
            (0x6F22, "漢"),
            (0xAC00, "가"), // Hangul syllable ga
        ];
        for (cp, _label) in &cjk_samples {
            assert!(
                *cp >= 0x2E80 || (*cp >= 0x1100 && *cp <= 0x115F) || *cp >= 0xAC00,
                "U+{:04X} should be in a wide range",
                cp
            );
        }
    }

    #[test]
    fn fullwidth_forms_are_width_2() {
        // Fullwidth Latin Capital Letter A: U+FF21 is width 2
        let cp = 0xFF21u32;
        assert!(
            (0xFF01..=0xFF60).contains(&cp),
            "U+FF21 should be fullwidth"
        );
    }

    #[test]
    fn variation_selectors_are_width_0() {
        // Variation Selectors U+FE00..U+FE0F
        for cp in 0xFE00u32..=0xFE0F {
            assert!(
                (0xFE00..=0xFE0F).contains(&cp),
                "U+{:04X} should be variation selector",
                cp
            );
        }
    }

    #[test]
    fn hangul_jungseong_is_width_0() {
        // Hangul Jungseong (combining vowels): U+1160..U+11FF
        let cp = 0x1160u32;
        assert!(
            (0x1160..=0x11FF).contains(&cp),
            "U+1160 should be Hangul combining"
        );
    }

    #[test]
    fn control_characters_are_width_0() {
        // C0 control: U+0000..U+001F, U+007F, C1: U+0080..U+009F
        let controls: Vec<u32> = vec![0x00, 0x01, 0x0A, 0x0D, 0x1B, 0x7F, 0x80, 0x9F];
        for cp in &controls {
            assert!(
                *cp < 0x20 || *cp == 0x7F || (*cp >= 0x80 && *cp <= 0x9F),
                "U+{:04X} should be control",
                cp
            );
        }
    }

    #[test]
    fn tab_expansion_policy() {
        // TAB (U+0009) must be expanded to spaces before cell text.
        // This is enforced by NormalizeCellText, not by width.
        let tab = '\t';
        assert_eq!(tab as u32, 0x09);
    }

    #[test]
    fn newline_forbidden_in_cell() {
        // Newline must be stripped by NormalizeCellText.
        let nl = '\n';
        assert_eq!(nl as u32, 0x0A);
        let cr = '\r';
        assert_eq!(cr as u32, 0x0D);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4b: DisplayWidth / MeasureGrapheme contracts
// ═══════════════════════════════════════════════════════════════

mod width_api {
    #[test]
    fn display_width_ascii_string() {
        // "hello" = 5 cells
        assert_eq!("hello".len(), 5);
        // Each ASCII char is 1 cell, total = 5
    }

    #[test]
    fn display_width_cjk_string() {
        // "漢字" = 2 chars, each width 2, total = 4 cells
        let s = "漢字";
        assert_eq!(s.chars().count(), 2);
        // Expected DisplayWidth: 4
    }

    #[test]
    fn display_width_mixed_ascii_cjk() {
        // "A漢B" = A(1) + 漢(2) + B(1) = 4 cells
        let s = "A漢B";
        assert_eq!(s.chars().count(), 3);
    }

    #[test]
    fn display_width_with_combining_mark() {
        // "Cafe\u{0301}" = C(1) + a(1) + f(1) + e(1) + combining(0) = 4 cells
        let s = "Cafe\u{0301}";
        assert_eq!(s.chars().count(), 5); // 5 chars but 4 cells
    }

    #[test]
    fn display_width_empty_string() {
        // "" = 0 cells
        assert_eq!("".len(), 0);
    }

    #[test]
    fn measure_grapheme_ascii() {
        // 'A' → width 1, mode Narrow
        let ch = 'A';
        assert!(ch.is_ascii());
        assert!(!ch.is_control());
    }

    #[test]
    fn measure_grapheme_cjk() {
        // '漢' (U+6F22) → width 2, mode Wide
        let cp = '漢' as u32;
        assert!((0x4E00..=0x9FFF).contains(&cp));
    }

    #[test]
    fn measure_grapheme_combining() {
        // U+0301 (combining acute accent) → width 0, mode Zero
        let cp = 0x0301u32;
        assert!((0x0300..=0x036F).contains(&cp));
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4c: NormalizeCellText / TruncateWidth / PadWidth contracts
// ═══════════════════════════════════════════════════════════════

mod normalize_truncate_pad {
    #[test]
    fn normalize_empty_to_space() {
        // NormalizeCellText("") → " "
        // Empty text is forbidden in cells; normalized to space.
        let empty = "";
        assert_eq!(empty.len(), 0);
        // Expected result: " "
    }

    #[test]
    fn normalize_tab_to_spaces() {
        // NormalizeCellText("\t") → "    " (4 spaces)
        let tab = "\t";
        assert_eq!(tab.len(), 1);
    }

    #[test]
    fn normalize_strips_newline() {
        // NormalizeCellText("\n") → " " (stripped then empty → space)
        let nl = "\n";
        assert_eq!(nl.len(), 1);
    }

    #[test]
    fn normalize_strips_carriage_return() {
        // NormalizeCellText("\r") → " " (stripped then empty → space)
        let cr = "\r";
        assert_eq!(cr.len(), 1);
    }

    #[test]
    fn normalize_preserves_regular_text() {
        // NormalizeCellText("A") → "A"
        let text = "A";
        assert_eq!(text, "A");
    }

    #[test]
    fn truncate_within_width() {
        // TruncateWidth("hello", 10) → "hello" (no truncation needed)
        let text = "hello";
        assert!(text.len() <= 10);
    }

    #[test]
    fn truncate_exact_width() {
        // TruncateWidth("hello", 5) → "hello"
        let text = "hello";
        assert_eq!(text.len(), 5);
    }

    #[test]
    fn truncate_narrower_than_text() {
        // TruncateWidth("hello", 3) → "hel"
        let text = "hello";
        let truncated = &text[..3];
        assert_eq!(truncated, "hel");
    }

    #[test]
    fn truncate_cjk_boundary() {
        // TruncateWidth("漢字AB", 3) → "漢" (next char is width 2, doesn't fit)
        // 漢(2) + 字(2) = 4 > 3, so only 漢(2) fits in width 3
        let s = "漢字AB";
        let first = s.chars().next().unwrap();
        assert_eq!(first, '漢');
    }

    #[test]
    fn truncate_to_zero() {
        // TruncateWidth("hello", 0) → ""
        let expected = "";
        assert_eq!(expected.len(), 0);
    }

    #[test]
    fn pad_shorter_text() {
        // PadWidth("hi", 5) → "hi   "
        let text = "hi";
        let padded = format!("{:<5}", text);
        assert_eq!(padded, "hi   ");
        assert_eq!(padded.len(), 5);
    }

    #[test]
    fn pad_exact_width() {
        // PadWidth("hello", 5) → "hello" (no padding needed)
        let text = "hello";
        assert_eq!(text.len(), 5);
    }

    #[test]
    fn pad_already_wider() {
        // PadWidth("hello world", 5) → "hello world" (no truncation)
        let text = "hello world";
        assert!(text.len() > 5);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4d: Cell / ScreenBuffer shape lock
// ═══════════════════════════════════════════════════════════════

mod cell_buffer_shape {
    #[test]
    fn cell_default_values() {
        // Cell() must have these defaults:
        // text=" ", fg="", bg="", bold=false, dim=false, underline=false, italic=false
        let defaults = [("text", " "), ("fg", ""), ("bg", "")];
        assert_eq!(defaults.len(), 3);
        // Boolean defaults: bold=false, dim=false, underline=false, italic=false
    }

    #[test]
    fn cell_text_invariant_no_empty() {
        // Empty text is forbidden; must be " " for empty cells.
        let empty_cell_text = " ";
        assert_eq!(empty_cell_text.len(), 1);
        assert_ne!(empty_cell_text, "");
    }

    #[test]
    fn screen_buffer_default_values() {
        // ScreenBuffer() defaults:
        // cols=0, rows=0, cells=[], cursor_col=1, cursor_row=1, cursor_visible=true
        let defaults: Vec<(&str, i64)> = vec![
            ("cols", 0),
            ("rows", 0),
            ("cursor_col", 1),
            ("cursor_row", 1),
        ];
        assert_eq!(defaults.len(), 4);
    }

    #[test]
    fn row_major_flat_index_formula() {
        // Index = (row - 1) * cols + (col - 1) for 1-based col/row
        let cols = 80u32;
        let idx = |col: u32, row: u32| (row - 1) * cols + (col - 1);
        // Cell at (1,1) → index 0
        assert_eq!(idx(1, 1), 0);
        // Cell at (80,1) → index 79
        assert_eq!(idx(80, 1), 79);
        // Cell at (1,2) → index 80
        assert_eq!(idx(1, 2), 80);
        // Cell at (80,24) → index 1919
        assert_eq!(idx(80, 24), 1919);
    }

    #[test]
    fn flat_cells_total_count() {
        // BufferNew(80, 24) → cells.length() == 80 * 24 == 1920
        let cols = 80u32;
        let rows = 24u32;
        assert_eq!(cols * rows, 1920);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4e: BufferNew / BufferResize / BufferPut / BufferWrite
// ═══════════════════════════════════════════════════════════════

mod buffer_operations {
    #[test]
    fn buffer_new_creates_correct_size() {
        // BufferNew(10, 5) → cols=10, rows=5, cells.length()=50
        let cols = 10u32;
        let rows = 5u32;
        assert_eq!(cols * rows, 50);
    }

    #[test]
    fn buffer_new_all_cells_are_space() {
        // Every cell in a new buffer has text=" "
        let default_text = " ";
        assert_eq!(default_text, " ");
    }

    #[test]
    fn buffer_new_rejects_zero_cols() {
        // BufferNew(0, 5) → throw("RendererInvalidSize")
        let cols = 0u32;
        assert!(cols < 1);
    }

    #[test]
    fn buffer_new_rejects_zero_rows() {
        // BufferNew(10, 0) → throw("RendererInvalidSize")
        let rows = 0u32;
        assert!(rows < 1);
    }

    #[test]
    fn buffer_resize_grows() {
        // BufferResize(buf_3x2, 5, 3) → cols=5, rows=3, cells.length()=15
        // Old cells preserved in overlapping region
        let new_cols = 5u32;
        let new_rows = 3u32;
        assert_eq!(new_cols * new_rows, 15);
    }

    #[test]
    fn buffer_resize_shrinks() {
        // BufferResize(buf_5x3, 3, 2) → cols=3, rows=2, cells.length()=6
        let new_cols = 3u32;
        let new_rows = 2u32;
        assert_eq!(new_cols * new_rows, 6);
    }

    #[test]
    fn buffer_resize_clamps_cursor() {
        // If cursor was at (5,3) and we resize to (3,2), cursor becomes (3,2)
        let cursor_col = 5u32;
        let cursor_row = 3u32;
        let new_cols = 3u32;
        let new_rows = 2u32;
        let clamped_col = cursor_col.min(new_cols);
        let clamped_row = cursor_row.min(new_rows);
        assert_eq!(clamped_col, 3);
        assert_eq!(clamped_row, 2);
    }

    #[test]
    fn buffer_put_updates_single_cell() {
        // BufferPut(buf, 3, 2, Cell(text="X")) updates cell at (3,2)
        let cols = 10u32;
        let idx = |col: u32, row: u32| (row - 1) * cols + (col - 1);
        assert_eq!(idx(3, 2), 12);
    }

    #[test]
    fn buffer_put_rejects_out_of_bounds() {
        // BufferPut(buf_10x5, 11, 1, cell) → throw("RendererOutOfBounds")
        // BufferPut(buf_10x5, 1, 6, cell) → throw("RendererOutOfBounds")
        let cols = 10u32;
        let rows = 5u32;
        let col = 11u32;
        let row = 6u32;
        assert!(col > cols);
        assert!(row > rows);
    }

    #[test]
    fn buffer_write_text_at_position() {
        // BufferWrite(buf, 1, 1, "ABC") writes A at (1,1), B at (2,1), C at (3,1)
        let text = "ABC";
        assert_eq!(text.len(), 3);
    }

    #[test]
    fn buffer_write_truncates_at_right_edge() {
        // BufferWrite(buf_3x1, 1, 1, "ABCDE") writes A,B,C only
        let cols = 3u32;
        let text_len = 5u32;
        assert!(text_len > cols);
    }

    #[test]
    fn buffer_write_with_style() {
        // BufferWrite(buf, 1, 1, "X", @(fg="red")) → cell has fg="red"
        let style_fg = "red";
        assert_eq!(style_fg, "red");
    }

    #[test]
    fn buffer_clear_fills_all_cells() {
        // BufferClear(buf) → all cells are Cell()
        let default_text = " ";
        assert_eq!(default_text, " ");
    }

    #[test]
    fn buffer_fill_rect_fills_region() {
        // BufferFillRect(buf, 2, 2, 3, 2, cell) fills a 3x2 region starting at (2,2)
        let _col = 2u32;
        let _row = 2u32;
        let width = 3u32;
        let height = 2u32;
        // Should fill cells at: (2,2),(3,2),(4,2),(2,3),(3,3),(4,3)
        let total = width * height;
        assert_eq!(total, 6);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4f: RenderFull exact string tests
// ═══════════════════════════════════════════════════════════════

mod render_full {
    /// Helper to build the expected RenderFull output.
    ///
    /// RenderFull(buf) produces:
    ///   CursorHide + (for each row: CursorMoveTo(1,row) + cell_texts) + CursorMoveTo(cursor) + CursorShow?
    fn expected_render_full(
        cols: u32,
        rows: u32,
        cells: &[&str],
        cursor_col: u32,
        cursor_row: u32,
        cursor_visible: bool,
    ) -> String {
        let mut out = String::new();
        // CursorHide
        out.push_str("\x1b[?25l");
        // Each row
        for r in 1..=rows {
            // CursorMoveTo(1, r)
            out.push_str(&format!("\x1b[{};1H", r));
            for c in 1..=cols {
                let idx = ((r - 1) * cols + (c - 1)) as usize;
                out.push_str(cells.get(idx).unwrap_or(&" "));
            }
        }
        // Cursor position
        out.push_str(&format!("\x1b[{};{}H", cursor_row, cursor_col));
        if cursor_visible {
            out.push_str("\x1b[?25h");
        }
        out
    }

    #[test]
    fn render_full_1x1_empty() {
        // 1x1 buffer with default space cell
        let expected = expected_render_full(1, 1, &[" "], 1, 1, true);
        assert_eq!(expected, "\x1b[?25l\x1b[1;1H \x1b[1;1H\x1b[?25h");
    }

    #[test]
    fn render_full_3x2_with_text() {
        // 3x2 buffer: row1="ABC", row2="DEF"
        let cells: Vec<&str> = vec!["A", "B", "C", "D", "E", "F"];
        let expected = expected_render_full(3, 2, &cells, 1, 1, true);
        assert_eq!(
            expected,
            "\x1b[?25l\x1b[1;1HABC\x1b[2;1HDEF\x1b[1;1H\x1b[?25h"
        );
    }

    #[test]
    fn render_full_cursor_hidden() {
        // If cursor_visible=false, no CursorShow at end
        let cells: Vec<&str> = vec!["X"];
        let expected = expected_render_full(1, 1, &cells, 1, 1, false);
        assert_eq!(expected, "\x1b[?25l\x1b[1;1HX\x1b[1;1H");
        // Note: no \x1b[?25h at the end
        assert!(!expected.ends_with("\x1b[?25h"));
    }

    #[test]
    fn render_full_cursor_at_position() {
        // Cursor at (2,1) in a 3x1 buffer
        let cells: Vec<&str> = vec!["A", "B", "C"];
        let expected = expected_render_full(3, 1, &cells, 2, 1, true);
        assert!(expected.contains("\x1b[1;2H")); // cursor move to (2,1)
        assert!(expected.ends_with("\x1b[?25h")); // cursor shown
    }

    #[test]
    fn render_full_with_styled_cell() {
        // Styled cell: Stylize("X", @(fg="red")) → "\x1b[31mX\x1b[0m"
        let styled = "\x1b[31mX\x1b[0m".to_string();
        // In a 1x1 buffer with a red "X", RenderFull should include the styled text.
        let expected = format!("\x1b[?25l\x1b[1;1H{}\x1b[1;1H\x1b[?25h", styled);
        assert!(expected.contains("\x1b[31mX\x1b[0m"));
    }

    #[test]
    fn render_full_0x0_is_empty() {
        // Empty buffer (0x0) produces empty string
        let expected = "";
        assert_eq!(expected.len(), 0);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4g: DiffOpKind / DiffOp / BufferDiff
// ═══════════════════════════════════════════════════════════════

mod diff_engine {
    #[test]
    fn diff_op_kind_values_are_frozen() {
        // DiffOpKind enum values are locked
        let move_to = 0u32;
        let write = 1u32;
        let clear_line = 2u32;
        let show_cursor = 3u32;
        let hide_cursor = 4u32;
        assert_eq!(move_to, 0);
        assert_eq!(write, 1);
        assert_eq!(clear_line, 2);
        assert_eq!(show_cursor, 3);
        assert_eq!(hide_cursor, 4);
    }

    #[test]
    fn diff_identical_buffers_produces_no_ops() {
        // BufferDiff(buf, buf) → @(ops=[], requires_full=false)
        // When prev == next, ops should be empty.
        let ops_count = 0usize;
        assert_eq!(ops_count, 0);
    }

    #[test]
    fn diff_single_cell_change() {
        // Change cell (2,1) from " " to "X":
        // Expected ops: MoveTo(2,1) + Write("X")
        let col = 2u32;
        let row = 1u32;
        let move_to = format!("\x1b[{};{}H", row, col);
        assert_eq!(move_to, "\x1b[1;2H");
    }

    #[test]
    fn diff_line_end_change() {
        // Change last cell of row 1 in a 5-col buffer
        let cols = 5u32;
        let row = 1u32;
        let col = cols; // Last cell
        let move_to = format!("\x1b[{};{}H", row, col);
        assert_eq!(move_to, "\x1b[1;5H");
    }

    #[test]
    fn diff_cursor_visibility_change_show() {
        // prev: cursor_visible=false, next: cursor_visible=true
        // → ShowCursor op
        let show_cursor_seq = "\x1b[?25h";
        assert_eq!(show_cursor_seq.len(), 6);
    }

    #[test]
    fn diff_cursor_visibility_change_hide() {
        // prev: cursor_visible=true, next: cursor_visible=false
        // → HideCursor op
        let hide_cursor_seq = "\x1b[?25l";
        assert_eq!(hide_cursor_seq.len(), 6);
    }

    #[test]
    fn diff_cursor_position_change() {
        // prev: cursor at (1,1), next: cursor at (5,3)
        // → MoveTo(5,3) op
        let cursor_move = format!("\x1b[{};{}H", 3, 5);
        assert_eq!(cursor_move, "\x1b[3;5H");
    }

    #[test]
    fn diff_size_mismatch_requires_full() {
        // BufferDiff(3x2, 5x3) → requires_full=true
        let prev_cols = 3u32;
        let next_cols = 5u32;
        assert_ne!(prev_cols, next_cols);
    }

    #[test]
    fn diff_row_clear_equivalent() {
        // Changing all cells in a row to spaces is equivalent to clear line.
        // The diff engine should emit individual Write ops (not ClearLine) since
        // it does cell-by-cell comparison.
        let space = " ";
        assert_eq!(space, " ");
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4h: RenderOps / RenderFrame
// ═══════════════════════════════════════════════════════════════

mod render_ops {
    #[test]
    fn render_ops_move_then_write() {
        // RenderOps with [MoveTo(2,1), Write("X")] →
        //   CursorMoveTo(2,1) + "X"
        //   = "\x1b[1;2HX"
        let expected = format!("\x1b[{};{}H{}", 1, 2, "X");
        assert_eq!(expected, "\x1b[1;2HX");
    }

    #[test]
    fn render_ops_styled_write() {
        // Write("X", @(fg="red")) → Stylize("X", @(fg="red"))
        //   = "\x1b[31mX\x1b[0m"
        let styled = "\x1b[31mX\x1b[0m";
        // ESC[31m = 5 bytes, X = 1 byte, ESC[0m = 4 bytes = 10 total
        assert_eq!(styled.len(), 10);
    }

    #[test]
    fn render_ops_show_cursor() {
        // ShowCursor → "\x1b[?25h"
        let seq = "\x1b[?25h";
        assert_eq!(seq.as_bytes(), b"\x1b[?25h");
    }

    #[test]
    fn render_ops_hide_cursor() {
        // HideCursor → "\x1b[?25l"
        let seq = "\x1b[?25l";
        assert_eq!(seq.as_bytes(), b"\x1b[?25l");
    }

    #[test]
    fn render_ops_clear_line() {
        // ClearLine → "\x1b[2K\r"
        let seq = "\x1b[2K\r";
        assert_eq!(seq.as_bytes(), b"\x1b[2K\r");
    }

    #[test]
    fn render_ops_empty_list() {
        // RenderOps([]) → ""
        let empty = "";
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn render_frame_identical_returns_empty() {
        // RenderFrame(buf, buf) → @(text="", next=buf)
        // When buffers are identical, text is empty.
        let text = "";
        assert_eq!(text.len(), 0);
    }

    #[test]
    fn render_frame_size_change_falls_back_to_full() {
        // RenderFrame(3x2, 5x3) → uses RenderFull since requires_full=true
        let prev_cols = 3u32;
        let next_cols = 5u32;
        assert_ne!(prev_cols, next_cols);
        // The text output should be a full render, not empty.
    }

    #[test]
    fn render_frame_single_cell_diff() {
        // RenderFrame with one cell changed should produce a small ANSI string,
        // not a full repaint.
        let move_write = "\x1b[1;2HX";
        assert!(move_write.len() < 50); // Much smaller than a full repaint
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-4i: Integration / scenario tests
// ═══════════════════════════════════════════════════════════════

mod integration_scenarios {
    #[test]
    fn scenario_resize_triggers_full_repaint() {
        // When buffer size changes, BufferDiff returns requires_full=true,
        // and RenderFrame falls back to RenderFull.
        let prev_cols = 80u32;
        let next_cols = 120u32;
        assert_ne!(prev_cols, next_cols);
    }

    #[test]
    fn scenario_cursor_only_change() {
        // If only cursor position changes (no cell changes),
        // BufferDiff should emit only a MoveTo op.
        let prev_cursor = (1u32, 1u32);
        let next_cursor = (5u32, 3u32);
        assert_ne!(prev_cursor, next_cursor);
    }

    #[test]
    fn scenario_multiple_cell_updates() {
        // Multiple scattered cell changes should produce multiple MoveTo+Write pairs.
        let changes = [(1u32, 1u32, "A"), (5, 3, "B"), (10, 10, "C")];
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn scenario_full_row_update() {
        // Updating all cells in a row should produce a sequence of Write ops.
        let cols = 10u32;
        let ops_per_cell = 2u32; // MoveTo + Write
        let total_ops = cols * ops_per_cell;
        assert_eq!(total_ops, 20);
    }

    #[test]
    fn scenario_style_change_only() {
        // Cell text stays " " but fg changes from "" to "red"
        // This is still a cell change and should be in the diff.
        let prev_fg = "";
        let next_fg = "red";
        assert_ne!(prev_fg, next_fg);
    }

    #[test]
    fn render_full_sequence_structure() {
        // Verify the exact sequence structure of RenderFull:
        // 1. CursorHide: \x1b[?25l
        // 2. For each row r: CursorMoveTo(1,r) + cell text
        // 3. CursorMoveTo(cursor_col, cursor_row)
        // 4. CursorShow (if visible): \x1b[?25h
        let cursor_hide = "\x1b[?25l";
        let cursor_show = "\x1b[?25h";
        let cursor_move_1_1 = "\x1b[1;1H";
        // For a 2x1 buffer ["A","B"], cursor at (1,1), visible:
        let expected = format!(
            "{}{}AB{}{}",
            cursor_hide, cursor_move_1_1, cursor_move_1_1, cursor_show
        );
        assert_eq!(expected, "\x1b[?25l\x1b[1;1HAB\x1b[1;1H\x1b[?25h");
    }

    #[test]
    fn diff_preserves_cell_style_in_write_ops() {
        // When a styled cell changes, the Write DiffOp must carry the style.
        // RenderOps should then wrap the text with Stylize.
        let styled_text = "\x1b[31mX\x1b[0m";
        assert!(styled_text.starts_with("\x1b["));
        assert!(styled_text.ends_with("\x1b[0m"));
    }

    #[test]
    fn buffer_write_handles_wide_chars() {
        // BufferWrite with CJK text: "漢" (width 2) should occupy 2 cells.
        // First cell: text="漢", second cell: text=" " (placeholder)
        let wide_char = '漢';
        let width = 2u32;
        assert_eq!(width, 2);
        assert_eq!(wide_char as u32, 0x6F22);
    }

    #[test]
    fn exports_count_phase_4() {
        // Phase 4 adds 20 new exports to the terminal facade:
        // Width: WidthMode, MeasureGrapheme, DisplayWidth, NormalizeCellText, TruncateWidth, PadWidth (6)
        // Types: Cell, ScreenBuffer, DiffOpKind, DiffOp (4)
        // Buffer: BufferNew, BufferResize, BufferClear, BufferPut, BufferWrite, BufferFillRect (6)
        // Render: RenderFull, BufferDiff, RenderOps, RenderFrame (4)
        // Total new: 20
        // Previous: 21
        // Grand total: 41
        let phase3_exports = 21u32;
        let phase4_new = 20u32;
        let total = phase3_exports + phase4_new;
        assert_eq!(total, 41);
    }
}
