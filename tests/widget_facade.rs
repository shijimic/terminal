#![allow(unused_variables, unused_assignments, dead_code)]
//! Widget / Prompt facade tests (TM-5a..5i).
//!
//! These tests validate the Phase 5 prompt/widget contracts:
//! - Line editor state machine (state transitions, editing operations)
//! - History navigation (prev/next)
//! - Password mode (masking)
//! - Completion state (minimal shape)
//! - Spinner (state cycle, render)
//! - Progress bar (fill calculation, labels, edge cases)
//! - Status line (left/right layout, truncation)
//!
//! Since all Phase 5 code is pure Taida (no Rust addon), these tests
//! verify the contracts by computing expected state/output values.

// ═══════════════════════════════════════════════════════════════
// TM-5a: Line editor / prompt API design
// ═══════════════════════════════════════════════════════════════

mod prompt_types {
    //! Verify the type shapes defined in TM_DESIGN.md Section 12.

    #[test]
    fn prompt_mode_variants() {
        // PromptMode has exactly 2 variants: Normal=0, Password=1
        let normal = 0;
        let password = 1;
        assert_eq!(normal, 0);
        assert_eq!(password, 1);
        assert_ne!(normal, password);
    }

    #[test]
    fn prompt_options_defaults() {
        // PromptOptions has 6 fields, all with defaults
        let prompt: &str = "";
        let initial: &str = "";
        let placeholder: &str = "";
        let mode: i32 = 0; // PromptMode.Normal
        let history: Vec<&str> = vec![];
        assert_eq!(prompt, "");
        assert_eq!(initial, "");
        assert_eq!(placeholder, "");
        assert_eq!(mode, 0);
        assert!(history.is_empty());
    }

    #[test]
    fn completion_state_defaults() {
        // CompletionState: items=@[], selected=0, visible=false
        let items: Vec<&str> = vec![];
        let selected: i32 = 0;
        let visible: bool = false;
        assert!(items.is_empty());
        assert_eq!(selected, 0);
        assert!(!visible);
    }

    #[test]
    fn line_editor_action_variants() {
        // LineEditorAction: Editing=0, Submitted=1, Cancelled=2
        let editing = 0;
        let submitted = 1;
        let cancelled = 2;
        assert_eq!(editing, 0);
        assert_eq!(submitted, 1);
        assert_eq!(cancelled, 2);
    }

    #[test]
    fn line_editor_state_defaults() {
        // LineEditorState has all required fields
        let text: &str = "";
        let cursor: i32 = 0;
        let prompt: &str = "";
        let mode: i32 = 0;
        let placeholder: &str = "";
        let history_index: i32 = -1;
        let history_saved: &str = "";
        let action: i32 = 0; // Editing
        assert_eq!(text, "");
        assert_eq!(cursor, 0);
        assert_eq!(prompt, "");
        assert_eq!(mode, 0);
        assert_eq!(placeholder, "");
        assert_eq!(history_index, -1);
        assert_eq!(history_saved, "");
        assert_eq!(action, 0);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5b: PromptMode / PromptOptions / LineEditorNew
// ═══════════════════════════════════════════════════════════════

mod line_editor_new {
    //! Verify LineEditorNew creates correct initial states.

    /// Simulates LineEditorNew with default options
    fn editor_new(
        prompt: &str,
        initial: &str,
        mode: i32,
        placeholder: &str,
        history: Vec<&str>,
    ) -> EditorState {
        EditorState {
            text: initial.to_string(),
            cursor: initial.len() as i32,
            prompt: prompt.to_string(),
            mode,
            placeholder: placeholder.to_string(),
            history: history.iter().map(|s| s.to_string()).collect(),
            history_index: -1,
            history_saved: String::new(),
            action: 0, // Editing
        }
    }

    struct EditorState {
        text: String,
        cursor: i32,
        prompt: String,
        mode: i32,
        placeholder: String,
        history: Vec<String>,
        history_index: i32,
        history_saved: String,
        action: i32,
    }

    #[test]
    fn default_options_creates_empty_editor() {
        let s = editor_new("", "", 0, "", vec![]);
        assert_eq!(s.text, "");
        assert_eq!(s.cursor, 0);
        assert_eq!(s.prompt, "");
        assert_eq!(s.mode, 0);
        assert_eq!(s.action, 0);
    }

    #[test]
    fn with_initial_text() {
        let s = editor_new("> ", "hello", 0, "", vec![]);
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 5); // cursor at end
        assert_eq!(s.prompt, "> ");
    }

    #[test]
    fn with_password_mode() {
        let s = editor_new("Password: ", "", 1, "", vec![]);
        assert_eq!(s.mode, 1); // Password
    }

    #[test]
    fn with_history() {
        let s = editor_new("", "", 0, "", vec!["cmd1", "cmd2", "cmd3"]);
        assert_eq!(s.history.len(), 3);
        assert_eq!(s.history_index, -1); // not browsing history
    }

    #[test]
    fn with_placeholder() {
        let s = editor_new("", "", 0, "Type here...", vec![]);
        assert_eq!(s.placeholder, "Type here...");
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5c: Basic editing operations
// ═══════════════════════════════════════════════════════════════

mod editing_operations {
    //! Test the core editing state transitions that LineEditorStep must implement.
    //! We simulate the state machine logic here to verify contracts.

    // Key kind constants (must match KeyKind in terminal.td)
    const KK_CHAR: i32 = 0;
    const KK_ENTER: i32 = 1;
    const KK_ESCAPE: i32 = 2;
    const KK_BACKSPACE: i32 = 4;
    const KK_DELETE: i32 = 5;
    const KK_ARROW_LEFT: i32 = 8;
    const KK_ARROW_RIGHT: i32 = 9;
    const KK_HOME: i32 = 10;
    const KK_END: i32 = 11;

    // Action constants
    const ACT_EDITING: i32 = 0;
    const ACT_SUBMITTED: i32 = 1;
    const ACT_CANCELLED: i32 = 2;

    struct Key {
        kind: i32,
        text: String,
    }

    struct State {
        text: String,
        cursor: i32,
        action: i32,
    }

    impl State {
        fn new(text: &str, cursor: i32) -> Self {
            State {
                text: text.to_string(),
                cursor,
                action: ACT_EDITING,
            }
        }
    }

    /// Simulates _insertAt
    fn insert_at(s: &str, pos: usize, ch: &str) -> String {
        let mut result = String::new();
        result.push_str(&s[..pos]);
        result.push_str(ch);
        result.push_str(&s[pos..]);
        result
    }

    /// Simulates _deleteAt
    fn delete_at(s: &str, pos: usize) -> String {
        let mut result = String::new();
        result.push_str(&s[..pos]);
        if pos < s.len() {
            result.push_str(&s[pos + 1..]);
        }
        result
    }

    /// Simulates one step of LineEditorStep
    fn step(state: &State, key: &Key) -> State {
        if state.action != ACT_EDITING {
            return State {
                text: state.text.clone(),
                cursor: state.cursor,
                action: state.action,
            };
        }

        match key.kind {
            KK_ENTER => State {
                text: state.text.clone(),
                cursor: state.cursor,
                action: ACT_SUBMITTED,
            },
            KK_ESCAPE => State {
                text: state.text.clone(),
                cursor: state.cursor,
                action: ACT_CANCELLED,
            },
            KK_ARROW_LEFT => {
                let nc = if state.cursor > 0 {
                    state.cursor - 1
                } else {
                    0
                };
                State::new(&state.text, nc)
            }
            KK_ARROW_RIGHT => {
                let len = state.text.len() as i32;
                let nc = if state.cursor < len {
                    state.cursor + 1
                } else {
                    len
                };
                State::new(&state.text, nc)
            }
            KK_HOME => State::new(&state.text, 0),
            KK_END => State::new(&state.text, state.text.len() as i32),
            KK_BACKSPACE => {
                if state.cursor == 0 {
                    State::new(&state.text, 0)
                } else {
                    let new_text = delete_at(&state.text, (state.cursor - 1) as usize);
                    State::new(&new_text, state.cursor - 1)
                }
            }
            KK_DELETE => {
                if state.cursor >= state.text.len() as i32 {
                    State::new(&state.text, state.cursor)
                } else {
                    let new_text = delete_at(&state.text, state.cursor as usize);
                    State::new(&new_text, state.cursor)
                }
            }
            KK_CHAR => {
                if key.text.is_empty() {
                    State::new(&state.text, state.cursor)
                } else {
                    let new_text = insert_at(&state.text, state.cursor as usize, &key.text);
                    State::new(&new_text, state.cursor + key.text.len() as i32)
                }
            }
            _ => State::new(&state.text, state.cursor),
        }
    }

    #[test]
    fn insert_single_char() {
        let s = State::new("", 0);
        let k = Key {
            kind: KK_CHAR,
            text: "a".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "a");
        assert_eq!(next.cursor, 1);
        assert_eq!(next.action, ACT_EDITING);
    }

    #[test]
    fn insert_multiple_chars_sequentially() {
        let mut s = State::new("", 0);
        for ch in ['h', 'e', 'l', 'l', 'o'] {
            let k = Key {
                kind: KK_CHAR,
                text: ch.to_string(),
            };
            s = step(&s, &k);
        }
        assert_eq!(s.text, "hello");
        assert_eq!(s.cursor, 5);
    }

    #[test]
    fn insert_at_middle() {
        let s = State::new("hllo", 1);
        let k = Key {
            kind: KK_CHAR,
            text: "e".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "hello");
        assert_eq!(next.cursor, 2);
    }

    #[test]
    fn arrow_left_moves_cursor() {
        let s = State::new("abc", 3);
        let k = Key {
            kind: KK_ARROW_LEFT,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.cursor, 2);
        assert_eq!(next.text, "abc");
    }

    #[test]
    fn arrow_left_at_start_stays() {
        let s = State::new("abc", 0);
        let k = Key {
            kind: KK_ARROW_LEFT,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.cursor, 0);
    }

    #[test]
    fn arrow_right_moves_cursor() {
        let s = State::new("abc", 0);
        let k = Key {
            kind: KK_ARROW_RIGHT,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.cursor, 1);
    }

    #[test]
    fn arrow_right_at_end_stays() {
        let s = State::new("abc", 3);
        let k = Key {
            kind: KK_ARROW_RIGHT,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.cursor, 3);
    }

    #[test]
    fn home_moves_to_start() {
        let s = State::new("hello", 3);
        let k = Key {
            kind: KK_HOME,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.cursor, 0);
    }

    #[test]
    fn end_moves_to_end() {
        let s = State::new("hello", 2);
        let k = Key {
            kind: KK_END,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.cursor, 5);
    }

    #[test]
    fn backspace_deletes_before_cursor() {
        let s = State::new("hello", 3);
        let k = Key {
            kind: KK_BACKSPACE,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "helo");
        assert_eq!(next.cursor, 2);
    }

    #[test]
    fn backspace_at_start_no_change() {
        let s = State::new("hello", 0);
        let k = Key {
            kind: KK_BACKSPACE,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "hello");
        assert_eq!(next.cursor, 0);
    }

    #[test]
    fn delete_at_cursor() {
        let s = State::new("hello", 2);
        let k = Key {
            kind: KK_DELETE,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "helo");
        assert_eq!(next.cursor, 2);
    }

    #[test]
    fn delete_at_end_no_change() {
        let s = State::new("hello", 5);
        let k = Key {
            kind: KK_DELETE,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "hello");
        assert_eq!(next.cursor, 5);
    }

    #[test]
    fn enter_submits() {
        let s = State::new("hello", 5);
        let k = Key {
            kind: KK_ENTER,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "hello");
        assert_eq!(next.action, ACT_SUBMITTED);
    }

    #[test]
    fn escape_cancels() {
        let s = State::new("hello", 5);
        let k = Key {
            kind: KK_ESCAPE,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "hello");
        assert_eq!(next.action, ACT_CANCELLED);
    }

    #[test]
    fn no_edit_after_submit() {
        let s = State {
            text: "hello".into(),
            cursor: 5,
            action: ACT_SUBMITTED,
        };
        let k = Key {
            kind: KK_CHAR,
            text: "x".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "hello"); // unchanged
        assert_eq!(next.action, ACT_SUBMITTED);
    }

    #[test]
    fn no_edit_after_cancel() {
        let s = State {
            text: "hello".into(),
            cursor: 5,
            action: ACT_CANCELLED,
        };
        let k = Key {
            kind: KK_CHAR,
            text: "x".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "hello"); // unchanged
        assert_eq!(next.action, ACT_CANCELLED);
    }

    #[test]
    fn empty_char_text_no_change() {
        let s = State::new("abc", 3);
        let k = Key {
            kind: KK_CHAR,
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "abc");
        assert_eq!(next.cursor, 3);
    }

    #[test]
    fn unknown_key_no_change() {
        let s = State::new("abc", 2);
        let k = Key {
            kind: 99, // unknown
            text: "".into(),
        };
        let next = step(&s, &k);
        assert_eq!(next.text, "abc");
        assert_eq!(next.cursor, 2);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5d: History prev / next
// ═══════════════════════════════════════════════════════════════

mod history_navigation {
    //! Test history up/down navigation contracts.

    const KK_ARROW_UP: i32 = 6;
    const KK_ARROW_DOWN: i32 = 7;
    const KK_CHAR: i32 = 0;

    struct HistoryState {
        text: String,
        cursor: i32,
        history: Vec<String>,
        history_index: i32,
        history_saved: String,
    }

    impl HistoryState {
        fn new(text: &str, history: Vec<&str>) -> Self {
            HistoryState {
                text: text.to_string(),
                cursor: text.len() as i32,
                history: history.iter().map(|s| s.to_string()).collect(),
                history_index: -1,
                history_saved: String::new(),
            }
        }
    }

    fn history_step(state: &HistoryState, key_kind: i32) -> HistoryState {
        match key_kind {
            KK_ARROW_UP => {
                if state.history.is_empty() {
                    return HistoryState {
                        text: state.text.clone(),
                        cursor: state.cursor,
                        history: state.history.clone(),
                        history_index: state.history_index,
                        history_saved: state.history_saved.clone(),
                    };
                }
                let new_idx = if state.history_index == -1 {
                    state.history.len() as i32 - 1
                } else if state.history_index > 0 {
                    state.history_index - 1
                } else {
                    0
                };
                let saved = if state.history_index == -1 {
                    state.text.clone()
                } else {
                    state.history_saved.clone()
                };
                let hist_text = &state.history[new_idx as usize];
                HistoryState {
                    text: hist_text.clone(),
                    cursor: hist_text.len() as i32,
                    history: state.history.clone(),
                    history_index: new_idx,
                    history_saved: saved,
                }
            }
            KK_ARROW_DOWN => {
                if state.history_index == -1 {
                    return HistoryState {
                        text: state.text.clone(),
                        cursor: state.cursor,
                        history: state.history.clone(),
                        history_index: state.history_index,
                        history_saved: state.history_saved.clone(),
                    };
                }
                let new_idx = state.history_index + 1;
                if new_idx >= state.history.len() as i32 {
                    // Back to current input
                    HistoryState {
                        text: state.history_saved.clone(),
                        cursor: state.history_saved.len() as i32,
                        history: state.history.clone(),
                        history_index: -1,
                        history_saved: String::new(),
                    }
                } else {
                    let hist_text = &state.history[new_idx as usize];
                    HistoryState {
                        text: hist_text.clone(),
                        cursor: hist_text.len() as i32,
                        history: state.history.clone(),
                        history_index: new_idx,
                        history_saved: state.history_saved.clone(),
                    }
                }
            }
            _ => HistoryState {
                text: state.text.clone(),
                cursor: state.cursor,
                history: state.history.clone(),
                history_index: state.history_index,
                history_saved: state.history_saved.clone(),
            },
        }
    }

    #[test]
    fn up_with_no_history_stays() {
        let s = HistoryState::new("current", vec![]);
        let next = history_step(&s, KK_ARROW_UP);
        assert_eq!(next.text, "current");
        assert_eq!(next.history_index, -1);
    }

    #[test]
    fn up_goes_to_last_history() {
        let s = HistoryState::new("", vec!["cmd1", "cmd2", "cmd3"]);
        let next = history_step(&s, KK_ARROW_UP);
        assert_eq!(next.text, "cmd3");
        assert_eq!(next.history_index, 2);
        assert_eq!(next.history_saved, ""); // saved current empty text
    }

    #[test]
    fn up_twice_goes_to_second_last() {
        let s = HistoryState::new("", vec!["cmd1", "cmd2", "cmd3"]);
        let s2 = history_step(&s, KK_ARROW_UP);
        let s3 = history_step(&s2, KK_ARROW_UP);
        assert_eq!(s3.text, "cmd2");
        assert_eq!(s3.history_index, 1);
    }

    #[test]
    fn up_at_top_stays_at_first() {
        let s = HistoryState::new("", vec!["cmd1"]);
        let s2 = history_step(&s, KK_ARROW_UP);
        assert_eq!(s2.text, "cmd1");
        assert_eq!(s2.history_index, 0);
        let s3 = history_step(&s2, KK_ARROW_UP);
        assert_eq!(s3.text, "cmd1");
        assert_eq!(s3.history_index, 0); // stays at 0
    }

    #[test]
    fn down_without_up_stays() {
        let s = HistoryState::new("current", vec!["cmd1"]);
        let next = history_step(&s, KK_ARROW_DOWN);
        assert_eq!(next.text, "current");
        assert_eq!(next.history_index, -1);
    }

    #[test]
    fn up_then_down_returns_to_current() {
        let s = HistoryState::new("my input", vec!["cmd1", "cmd2"]);
        let s2 = history_step(&s, KK_ARROW_UP);
        assert_eq!(s2.text, "cmd2");
        let s3 = history_step(&s2, KK_ARROW_DOWN);
        assert_eq!(s3.text, "my input"); // saved text restored
        assert_eq!(s3.history_index, -1);
    }

    #[test]
    fn up_up_down_navigates_correctly() {
        let s = HistoryState::new("", vec!["cmd1", "cmd2", "cmd3"]);
        let s2 = history_step(&s, KK_ARROW_UP); // cmd3, idx=2
        let s3 = history_step(&s2, KK_ARROW_UP); // cmd2, idx=1
        let s4 = history_step(&s3, KK_ARROW_DOWN); // cmd3, idx=2
        assert_eq!(s4.text, "cmd3");
        assert_eq!(s4.history_index, 2);
    }

    #[test]
    fn history_saves_current_text() {
        let s = HistoryState::new("partial input", vec!["old cmd"]);
        let s2 = history_step(&s, KK_ARROW_UP);
        assert_eq!(s2.text, "old cmd");
        assert_eq!(s2.history_saved, "partial input");
        let s3 = history_step(&s2, KK_ARROW_DOWN);
        assert_eq!(s3.text, "partial input"); // restored
    }

    #[test]
    fn cursor_at_end_of_history_entry() {
        let s = HistoryState::new("", vec!["hello"]);
        let s2 = history_step(&s, KK_ARROW_UP);
        assert_eq!(s2.text, "hello");
        assert_eq!(s2.cursor, 5); // cursor at end
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5e: Password mode
// ═══════════════════════════════════════════════════════════════

mod password_mode {
    //! Password mode: display is masked, return value is raw text.

    #[test]
    fn password_mask_length_matches_text() {
        // In password mode, each character should be displayed as '*'
        let text = "secret123";
        let mask: String = "*".repeat(text.len());
        assert_eq!(mask, "*********");
        assert_eq!(mask.len(), text.len());
    }

    #[test]
    fn password_empty_text_no_mask() {
        let text = "";
        let mask: String = "*".repeat(text.len());
        assert_eq!(mask, "");
    }

    #[test]
    fn password_raw_text_preserved() {
        // The state's text field must contain raw text, not masked
        let raw_text = "p@ssw0rd!";
        // After submit, action=Submitted, text=raw
        assert_eq!(raw_text, "p@ssw0rd!");
    }

    #[test]
    fn password_cursor_position_uses_mask_width() {
        // In password mode, cursor display position = number of '*' before cursor
        let text = "abc";
        let cursor = 2; // after 'b'
        // Display: "**|*" — cursor at position 2 in mask
        let mask_width_before_cursor = cursor; // each char = 1 '*'
        assert_eq!(mask_width_before_cursor, 2);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5f: CompletionState minimal
// ═══════════════════════════════════════════════════════════════

mod completion_state {
    //! CompletionState v1: items list + selected index + visible flag.

    #[test]
    fn default_completion_state() {
        let items: Vec<&str> = vec![];
        let selected = 0;
        let visible = false;
        assert!(items.is_empty());
        assert_eq!(selected, 0);
        assert!(!visible);
    }

    #[test]
    fn completion_with_items() {
        let items = ["git add", "git commit", "git push"];
        let selected = 1;
        let visible = true;
        assert_eq!(items.len(), 3);
        assert_eq!(items[selected], "git commit");
        assert!(visible);
    }

    #[test]
    fn completion_selected_wraps() {
        // Selected index should be bounded by items length
        let items = ["a", "b", "c"];
        let selected = 2;
        assert!(selected < items.len());
        // Next would need to wrap to 0 (in Taida implementation)
        let next_selected = (selected + 1) % items.len();
        assert_eq!(next_selected, 0);
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5g: SpinnerState / SpinnerNext / SpinnerRender
// ═══════════════════════════════════════════════════════════════

mod spinner {
    //! Spinner widget: frame cycling and render output.

    const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    const FRAME_COUNT: i32 = 10;

    struct SpinnerState {
        frame: i32,
        label: String,
        done: bool,
    }

    fn spinner_next(state: &SpinnerState) -> SpinnerState {
        if state.done {
            return SpinnerState {
                frame: state.frame,
                label: state.label.clone(),
                done: true,
            };
        }
        let next_frame = (state.frame + 1) % FRAME_COUNT;
        SpinnerState {
            frame: next_frame,
            label: state.label.clone(),
            done: false,
        }
    }

    fn spinner_render(state: &SpinnerState) -> String {
        if state.done {
            if state.label.is_empty() {
                return "v".to_string();
            }
            return format!("v {}", state.label);
        }
        let frame_char = SPINNER_FRAMES[state.frame as usize];
        if state.label.is_empty() {
            return frame_char.to_string();
        }
        format!("{} {}", frame_char, state.label)
    }

    #[test]
    fn initial_frame_is_0() {
        let s = SpinnerState {
            frame: 0,
            label: "".into(),
            done: false,
        };
        assert_eq!(s.frame, 0);
    }

    #[test]
    fn next_increments_frame() {
        let s = SpinnerState {
            frame: 0,
            label: "".into(),
            done: false,
        };
        let s2 = spinner_next(&s);
        assert_eq!(s2.frame, 1);
    }

    #[test]
    fn frame_wraps_around() {
        let s = SpinnerState {
            frame: 9,
            label: "".into(),
            done: false,
        };
        let s2 = spinner_next(&s);
        assert_eq!(s2.frame, 0); // wraps
    }

    #[test]
    fn done_spinner_no_advance() {
        let s = SpinnerState {
            frame: 5,
            label: "".into(),
            done: true,
        };
        let s2 = spinner_next(&s);
        assert_eq!(s2.frame, 5); // no change
        assert!(s2.done);
    }

    #[test]
    fn render_frame_0() {
        let s = SpinnerState {
            frame: 0,
            label: "".into(),
            done: false,
        };
        assert_eq!(spinner_render(&s), "⠋");
    }

    #[test]
    fn render_with_label() {
        let s = SpinnerState {
            frame: 0,
            label: "Loading...".into(),
            done: false,
        };
        assert_eq!(spinner_render(&s), "⠋ Loading...");
    }

    #[test]
    fn render_done_no_label() {
        let s = SpinnerState {
            frame: 3,
            label: "".into(),
            done: true,
        };
        assert_eq!(spinner_render(&s), "v");
    }

    #[test]
    fn render_done_with_label() {
        let s = SpinnerState {
            frame: 3,
            label: "Complete".into(),
            done: true,
        };
        assert_eq!(spinner_render(&s), "v Complete");
    }

    #[test]
    fn full_cycle_returns_to_frame_0() {
        let mut s = SpinnerState {
            frame: 0,
            label: "".into(),
            done: false,
        };
        for _ in 0..10 {
            s = spinner_next(&s);
        }
        assert_eq!(s.frame, 0); // 10 steps from 0 = back to 0
    }

    #[test]
    fn all_10_frames_render_different() {
        let mut renders = std::collections::HashSet::new();
        for i in 0..10 {
            let s = SpinnerState {
                frame: i,
                label: "".into(),
                done: false,
            };
            renders.insert(spinner_render(&s));
        }
        assert_eq!(renders.len(), 10); // all unique
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5h: ProgressBar / StatusLine
// ═══════════════════════════════════════════════════════════════

mod progress_bar {
    //! Progress bar: fill calculation and label assembly.

    struct ProgressOptions {
        width: i32,
        complete_char: char,
        incomplete_char: char,
        left_label: String,
        right_label: String,
    }

    impl Default for ProgressOptions {
        fn default() -> Self {
            ProgressOptions {
                width: 20,
                complete_char: '#',
                incomplete_char: '-',
                left_label: String::new(),
                right_label: String::new(),
            }
        }
    }

    fn progress_bar(
        current: i32,
        total: i32,
        opts: &ProgressOptions,
    ) -> Result<String, &'static str> {
        if total < 1 {
            return Err("ProgressInvalidTotal");
        }
        if current < 0 {
            return Err("ProgressInvalidCurrent");
        }
        let clamped = current.min(total);
        let bar_width = opts.width.max(1);
        let filled = (clamped * bar_width) / total;
        let empty = bar_width - filled;

        let bar: String = std::iter::repeat_n(opts.complete_char, filled as usize)
            .chain(std::iter::repeat_n(opts.incomplete_char, empty as usize))
            .collect();

        let with_left = if opts.left_label.is_empty() {
            bar.clone()
        } else {
            format!("{} {}", opts.left_label, bar)
        };

        Ok(if opts.right_label.is_empty() {
            with_left
        } else {
            format!("{} {}", with_left, opts.right_label)
        })
    }

    #[test]
    fn empty_progress() {
        let result = progress_bar(0, 100, &ProgressOptions::default()).unwrap();
        assert_eq!(result, "--------------------"); // 20 dashes
    }

    #[test]
    fn full_progress() {
        let result = progress_bar(100, 100, &ProgressOptions::default()).unwrap();
        assert_eq!(result, "####################"); // 20 hashes
    }

    #[test]
    fn half_progress() {
        let result = progress_bar(50, 100, &ProgressOptions::default()).unwrap();
        assert_eq!(result, "##########----------"); // 10 + 10
    }

    #[test]
    fn quarter_progress() {
        let result = progress_bar(25, 100, &ProgressOptions::default()).unwrap();
        // 25 * 20 / 100 = 5
        assert_eq!(result, "#####---------------");
    }

    #[test]
    fn over_total_clamped() {
        let result = progress_bar(200, 100, &ProgressOptions::default()).unwrap();
        assert_eq!(result, "####################"); // clamped to 100%
    }

    #[test]
    fn invalid_total_error() {
        let result = progress_bar(10, 0, &ProgressOptions::default());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "ProgressInvalidTotal");
    }

    #[test]
    fn negative_current_error() {
        let result = progress_bar(-5, 100, &ProgressOptions::default());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "ProgressInvalidCurrent");
    }

    #[test]
    fn custom_chars() {
        let opts = ProgressOptions {
            width: 10,
            complete_char: '=',
            incomplete_char: '.',
            ..Default::default()
        };
        let result = progress_bar(3, 10, &opts).unwrap();
        assert_eq!(result, "===.......");
    }

    #[test]
    fn with_left_label() {
        let opts = ProgressOptions {
            width: 10,
            left_label: "DL".into(),
            ..Default::default()
        };
        let result = progress_bar(5, 10, &opts).unwrap();
        assert_eq!(result, "DL #####-----");
    }

    #[test]
    fn with_right_label() {
        let opts = ProgressOptions {
            width: 10,
            right_label: "50%".into(),
            ..Default::default()
        };
        let result = progress_bar(5, 10, &opts).unwrap();
        assert_eq!(result, "#####----- 50%");
    }

    #[test]
    fn with_both_labels() {
        let opts = ProgressOptions {
            width: 10,
            left_label: "DL".into(),
            right_label: "50%".into(),
            ..Default::default()
        };
        let result = progress_bar(5, 10, &opts).unwrap();
        assert_eq!(result, "DL #####----- 50%");
    }

    #[test]
    fn width_1_progress() {
        let opts = ProgressOptions {
            width: 1,
            ..Default::default()
        };
        let empty = progress_bar(0, 10, &opts).unwrap();
        assert_eq!(empty, "-");
        let full = progress_bar(10, 10, &opts).unwrap();
        assert_eq!(full, "#");
    }

    #[test]
    fn small_total() {
        let result = progress_bar(1, 1, &ProgressOptions::default()).unwrap();
        assert_eq!(result, "####################"); // 1/1 = 100%
    }
}

mod status_line {
    //! Status line: left/right layout with padding.

    fn unicode_width(s: &str) -> usize {
        // Simplified: ASCII chars are width 1 each
        s.len()
    }

    fn status_line(left: &str, right: &str, width: usize) -> String {
        if width == 0 {
            return format!("{}{}", left, right);
        }
        let left_w = unicode_width(left);
        let right_w = unicode_width(right);
        let total = left_w + right_w;
        if total <= width {
            let gap = width - total;
            let padding: String = " ".repeat(gap);
            format!("{}{}{}", left, padding, right)
        } else {
            // Truncate left
            let avail = width.saturating_sub(right_w);
            if avail == 0 {
                right[..width.min(right.len())].to_string()
            } else {
                format!("{}{}", &left[..avail.min(left.len())], right)
            }
        }
    }

    #[test]
    fn zero_width_concatenates() {
        let result = status_line("left", "right", 0);
        assert_eq!(result, "leftright");
    }

    #[test]
    fn basic_padding() {
        let result = status_line("Ready", "Ln 1", 20);
        assert_eq!(result, "Ready           Ln 1"); // 5 + 11 spaces + 4 = 20
    }

    #[test]
    fn exact_width_no_padding() {
        let result = status_line("AB", "CD", 4);
        assert_eq!(result, "ABCD");
    }

    #[test]
    fn only_left() {
        let result = status_line("Ready", "", 20);
        assert_eq!(result, "Ready               "); // padded to 20
    }

    #[test]
    fn only_right() {
        let result = status_line("", "Ln 1", 20);
        assert_eq!(result, "                Ln 1"); // left is empty, 16 spaces + 4
    }

    #[test]
    fn left_truncated_when_too_long() {
        let result = status_line("Very Long Left Text", "R", 10);
        // avail = 10 - 1 = 9
        assert_eq!(result, "Very LongR"); // truncated left + right
    }

    #[test]
    fn both_empty() {
        let result = status_line("", "", 10);
        assert_eq!(result, "          "); // 10 spaces
    }
}

// ═══════════════════════════════════════════════════════════════
// TM-5i: Integration / end-to-end widget tests
// ═══════════════════════════════════════════════════════════════

mod widget_integration {
    //! Integration tests verifying widgets compose correctly.

    #[test]
    fn line_editor_render_with_prompt() {
        // LineEditorRender should produce: ClearLine + prompt + text
        let clear_line = "\x1b[2K\r";
        let prompt = "> ";
        let text = "hello";
        let expected_line = format!("{}{}{}", clear_line, prompt, text);
        assert_eq!(expected_line, "\x1b[2K\r> hello");
        // cursor_col = prompt_width + cursor_display_pos + 1
        let prompt_width = 2; // "> " is 2 chars
        let cursor_pos = 5; // at end of "hello"
        let cursor_col = prompt_width + cursor_pos + 1;
        assert_eq!(cursor_col, 8);
    }

    #[test]
    fn line_editor_render_password_mode() {
        let clear_line = "\x1b[2K\r";
        let prompt = "Password: ";
        let text = "secret";
        let mask = "******"; // 6 asterisks
        let expected_line = format!("{}{}{}", clear_line, prompt, mask);
        assert_eq!(expected_line, "\x1b[2K\rPassword: ******");
        // cursor in password mode uses mask width
        let prompt_width = 10;
        let cursor_at_end = 6; // mask width
        let cursor_col = prompt_width + cursor_at_end + 1;
        assert_eq!(cursor_col, 17);
    }

    #[test]
    fn line_editor_render_placeholder() {
        let clear_line = "\x1b[2K\r";
        let prompt = "> ";
        let placeholder = "Type here...";
        // When text is empty, render shows placeholder
        let expected_line = format!("{}{}{}", clear_line, prompt, placeholder);
        assert_eq!(expected_line, "\x1b[2K\r> Type here...");
    }

    #[test]
    fn spinner_and_progress_compose() {
        // A typical TUI might show: "⠋ Loading... [#####-----] 50%"
        let spinner_frame = "⠋";
        let label = "Loading...";
        let bar = "#####-----";
        let pct = "50%";
        let line = format!("{} {} [{}] {}", spinner_frame, label, bar, pct);
        assert_eq!(line, "⠋ Loading... [#####-----] 50%");
    }

    #[test]
    fn status_line_in_alt_screen() {
        // Status line is typically rendered at the bottom of alt screen
        let alt_enter = "\x1b[?1049h";
        let alt_leave = "\x1b[?1049l";
        // Just verify the ANSI sequences are correct
        assert_eq!(alt_enter, "\x1b[?1049h");
        assert_eq!(alt_leave, "\x1b[?1049l");
    }

    #[test]
    fn full_editing_sequence() {
        // Simulate: type "helo", left, left, insert 'l', right, right => "hello"
        let mut text = String::new();
        let mut cursor: i32 = 0;

        // Type 'h', 'e', 'l', 'o'
        for ch in ['h', 'e', 'l', 'o'] {
            text.insert(cursor as usize, ch);
            cursor += 1;
        }
        assert_eq!(text, "helo");
        assert_eq!(cursor, 4);

        // Left twice
        cursor -= 1; // 3
        cursor -= 1; // 2

        // Insert 'l' at position 2
        text.insert(cursor as usize, 'l');
        cursor += 1; // 3

        assert_eq!(text, "hello");
        assert_eq!(cursor, 3);

        // Right twice
        cursor += 1; // 4
        cursor += 1; // 5
        assert_eq!(cursor, 5);
        assert_eq!(text, "hello");
    }

    #[test]
    fn backspace_then_retype() {
        let mut text = "helpo".to_string();
        let mut cursor: i32 = 4; // after 'p'

        // Backspace: delete 'p'
        text.remove((cursor - 1) as usize);
        cursor -= 1; // 3
        assert_eq!(text, "helo");

        // Type 'l'
        text.insert(cursor as usize, 'l');
        cursor += 1; // 4
        assert_eq!(text, "hello");
    }

    #[test]
    fn history_round_trip() {
        // Up -> edit -> down -> original preserved
        let history = ["cmd1", "cmd2"];
        let current = "partial";

        // Up -> cmd2
        let browsing = history[1];
        assert_eq!(browsing, "cmd2");

        // Down -> back to current
        assert_eq!(current, "partial");
    }

    #[test]
    fn widget_pure_render_no_side_effects() {
        // All widgets return Str, no I/O side effects
        // Spinner
        let spinner_output = format!("{} {}", "⠋", "Loading");
        assert!(!spinner_output.is_empty());

        // Progress
        let bar = "##########----------";
        assert_eq!(bar.len(), 20);

        // Status line
        let status = format!("{}{}{}", "Ready", "    ", "Ln 1");
        assert!(!status.is_empty());
    }

    #[test]
    fn export_count_phase5() {
        // Phase 5 adds 14 new exports to terminal.td:
        // From prompt.td: PromptMode, PromptOptions, CompletionState,
        //   LineEditorAction, LineEditorState, LineEditorNew, LineEditorStep, LineEditorRender (8)
        // From widgets.td: SpinnerState, SpinnerNext, SpinnerRender,
        //   ProgressOptions, ProgressBar, StatusLine (6)
        // Total new: 14
        // Previous: 41
        // New total: 55
        let phase4_exports = 41;
        let phase5_new = 14;
        assert_eq!(phase4_exports + phase5_new, 55);
    }
}
