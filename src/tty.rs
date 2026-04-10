//! `taida-lang/terminal` — `IsTerminal[]()` implementation.
//!
//! This module exposes the internal `isatty` probe as a public addon
//! entry so Taida code can guard `ReadKey[]()` / `TerminalSize[]()`
//! without speculative failure handling.

use taida_addon::bridge::{HostValueBuilder, borrow_arg};
use taida_addon::{TaidaAddonErrorV1, TaidaAddonStatus, TaidaAddonValueV1, TaidaHostV1};

/// `IsTerminal[]()` error codes.
pub mod err {
    /// The caller passed a stream name other than `stdin`, `stdout`,
    /// or `stderr`.
    pub const IS_TERMINAL_INVALID_STREAM: u32 = 2101;
    /// The addon could not allocate the return Bool on the host.
    pub const IS_TERMINAL_BUILD_VALUE: u32 = 2102;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamKind {
    Stdin,
    Stdout,
    Stderr,
}

fn parse_stream_kind(name: &str) -> Option<StreamKind> {
    match name {
        "stdin" => Some(StreamKind::Stdin),
        "stdout" => Some(StreamKind::Stdout),
        "stderr" => Some(StreamKind::Stderr),
        _ => None,
    }
}

fn is_terminal_stream(stream: StreamKind) -> bool {
    let fd = match stream {
        StreamKind::Stdin => libc::STDIN_FILENO,
        StreamKind::Stdout => libc::STDOUT_FILENO,
        StreamKind::Stderr => libc::STDERR_FILENO,
    };
    // SAFETY: `isatty` reads process-owned fd metadata and does not
    // retain pointers.
    unsafe { libc::isatty(fd) == 1 }
}

pub fn is_terminal_impl(
    host_ptr: *const TaidaHostV1,
    args_ptr: *const TaidaAddonValueV1,
    args_len: u32,
    out_value: *mut *mut TaidaAddonValueV1,
    out_error: *mut *mut TaidaAddonErrorV1,
) -> TaidaAddonStatus {
    if args_len != 1 {
        return TaidaAddonStatus::ArityMismatch;
    }
    if host_ptr.is_null() {
        return TaidaAddonStatus::InvalidState;
    }

    let builder = match unsafe { HostValueBuilder::from_raw(host_ptr) } {
        Some(b) => b,
        None => return TaidaAddonStatus::InvalidState,
    };

    let arg = match unsafe { borrow_arg(args_ptr, args_len, 0) } {
        Some(v) => v,
        None => return TaidaAddonStatus::NullPointer,
    };
    let Some(stream_name) = arg.as_str() else {
        return TaidaAddonStatus::UnsupportedValue;
    };
    let Some(stream) = parse_stream_kind(stream_name) else {
        let err = builder.error(
            err::IS_TERMINAL_INVALID_STREAM,
            &format!(
                "IsTerminalInvalidStream: expected stdin|stdout|stderr, got {}",
                stream_name
            ),
        );
        if !out_error.is_null() {
            unsafe { *out_error = err };
        }
        return TaidaAddonStatus::Error;
    };

    let value = builder.bool(is_terminal_stream(stream));
    if value.is_null() {
        let err = builder.error(
            err::IS_TERMINAL_BUILD_VALUE,
            "IsTerminalBuildValue: failed to allocate Bool return value",
        );
        if !out_error.is_null() {
            unsafe { *out_error = err };
        }
        return TaidaAddonStatus::Error;
    }
    if !out_value.is_null() {
        unsafe { *out_value = value };
    }
    TaidaAddonStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_terminal_error_codes_are_frozen() {
        assert_eq!(err::IS_TERMINAL_INVALID_STREAM, 2101);
        assert_eq!(err::IS_TERMINAL_BUILD_VALUE, 2102);
    }

    #[test]
    fn parse_stream_kind_accepts_only_three_streams() {
        assert_eq!(parse_stream_kind("stdin"), Some(StreamKind::Stdin));
        assert_eq!(parse_stream_kind("stdout"), Some(StreamKind::Stdout));
        assert_eq!(parse_stream_kind("stderr"), Some(StreamKind::Stderr));
        assert_eq!(parse_stream_kind("STDOUT"), None);
        assert_eq!(parse_stream_kind("tty"), None);
        assert_eq!(parse_stream_kind(""), None);
    }

    #[test]
    fn is_terminal_stream_matches_libc_for_standard_fds() {
        assert_eq!(is_terminal_stream(StreamKind::Stdin), unsafe {
            libc::isatty(libc::STDIN_FILENO) == 1
        });
        assert_eq!(is_terminal_stream(StreamKind::Stdout), unsafe {
            libc::isatty(libc::STDOUT_FILENO) == 1
        });
        assert_eq!(is_terminal_stream(StreamKind::Stderr), unsafe {
            libc::isatty(libc::STDERR_FILENO) == 1
        });
    }

    #[test]
    fn is_terminal_impl_arity_mismatch_when_args_missing() {
        let status = is_terminal_impl(
            core::ptr::null(),
            core::ptr::null(),
            0,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!(status, TaidaAddonStatus::ArityMismatch);
    }

    #[test]
    fn is_terminal_impl_invalid_state_when_host_null() {
        let status = is_terminal_impl(
            core::ptr::null(),
            core::ptr::null(),
            1,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!(status, TaidaAddonStatus::InvalidState);
    }
}
