// FlowDesk log parser.
//
// Parses the C++ core's stdout lines into structured events. Mirrors the
// legacy GUI's `checkConnected` and `checkFingerprint` logic (see
// MainWindow.cpp:407 and MainWindow.cpp:429). See docs/design/tauri-gui.md §3.3.
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

use regex::Regex;
use std::sync::OnceLock;

/// Lifecycle state inferred from a log line. Own type (kept here, not in
/// supervisor) to avoid an import cycle between the two modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedState {
    Connected,
    Error,
}

/// Events emitted by parsing one or more log lines.
#[derive(Debug, Clone)]
pub enum LogEvent {
    /// The core transitioned to a new lifecycle state.
    StateChange(DerivedState),
    /// A TLS peer fingerprint was seen — user must decide whether to trust.
    FingerprintPrompt { sha1: String, sha256: String },
}

/// Parsed representation of a single log line.
pub struct ParsedLine<'a> {
    pub level: Option<&'a str>,
    pub message: &'a str,
    /// Derived events (state changes / fingerprints) from this line.
    pub events_buf: Vec<LogEvent>,
}

impl<'a> ParsedLine<'a> {
    pub fn level_str(&self) -> &'static str {
        // Normalize to a small set for the frontend.
        match self.level.map(str::to_ascii_uppercase).as_deref() {
            Some("FATAL") | Some("ERROR") => "ERROR",
            Some("WARNING") => "WARNING",
            Some("NOTE") => "NOTE",
            Some("INFO") => "INFO",
            Some("DEBUG") | Some("DEBUG1") | Some("DEBUG2") => "DEBUG",
            Some(s) if s.starts_with("DEBUG") => "DEBUG",
            _ => "INFO",
        }
    }

    pub fn message(&self) -> &str {
        self.message
    }

    pub fn events(&self) -> &[LogEvent] {
        &self.events_buf
    }
}

/// Log line shape: `[YYYY-MM-DDTHH:MM:SS] LEVEL: message`
static LOG_RE: OnceLock<Regex> = OnceLock::new();
/// Fingerprint shape: `peer fingerprint (SHA1): XX:.. (SHA256): XX:..`
static FINGERPRINT_RE: OnceLock<Regex> = OnceLock::new();

fn log_re() -> &'static Regex {
    LOG_RE.get_or_init(|| {
        Regex::new(
            r"^\[(?P<ts>[^\]]+)\]\s+(?P<level>[A-Z0-9]+):\s(?P<msg>.*)$",
        )
        .expect("static regex")
    })
}

fn fingerprint_re() -> &'static Regex {
    FINGERPRINT_RE.get_or_init(|| {
        Regex::new(r"peer fingerprint \(SHA1\):\s*(?P<sha1>[0-9A-Fa-f:]+)\s*\(SHA256\):\s*(?P<sha256>[0-9A-Fa-f:]+)")
            .expect("static regex")
    })
}

/// Parse one raw stdout line into level + message + derived events.
pub fn parse_log_line(raw: &str) -> ParsedLine<'_> {
    let mut events_buf = Vec::new();

    // First, try the structured form.
    let (level, message) = if let Some(caps) = log_re().captures(raw) {
        let level = caps.name("level").map(|m| m.as_str());
        let msg = caps.name("msg").map(|m| m.as_str()).unwrap_or(raw);
        (level, msg)
    } else {
        // Unstructured (e.g. a CLOG_PRINT line with no timestamp prefix).
        (None, raw)
    };

    // Derive events.
    let lower_msg = message.to_ascii_lowercase();
    if lower_msg.contains("started server")
        || lower_msg.contains("connected to server")
        || lower_msg.contains("server status: active")
    {
        events_buf.push(LogEvent::StateChange(DerivedState::Connected));
    } else if lower_msg.contains("cannot listen for clients")
        || lower_msg.contains("failed to connect to server")
        || lower_msg.contains("cannot read configuration")
    {
        events_buf.push(LogEvent::StateChange(DerivedState::Error));
    }

    if let Some(caps) = fingerprint_re().captures(message) {
        events_buf.push(LogEvent::FingerprintPrompt {
            sha1: caps.name("sha1").unwrap().as_str().to_string(),
            sha256: caps.name("sha256").unwrap().as_str().to_string(),
        });
    }

    ParsedLine {
        level,
        message,
        events_buf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_line() {
        let raw = "[2026-06-23T12:00:00] NOTE: started server, waiting for clients";
        let p = parse_log_line(raw);
        assert_eq!(p.level, Some("NOTE"));
        assert!(p.message.contains("started server"));
        assert!(matches!(
            p.events_buf.first(),
            Some(LogEvent::StateChange(DerivedState::Connected))
        ));
    }

    #[test]
    fn parses_unstructured_print_line() {
        let raw = "started server (ipv4), waiting for clients";
        let p = parse_log_line(raw);
        assert!(p.level.is_none());
        assert!(!p.events_buf.is_empty());
    }

    #[test]
    fn parses_fingerprint_line() {
        let raw = "[2026-06-23T12:00:00] NOTE: peer fingerprint (SHA1): AA:BB (SHA256): CC:DD";
        let p = parse_log_line(raw);
        assert!(p
            .events_buf
            .iter()
            .any(|e| matches!(e, LogEvent::FingerprintPrompt { .. })));
    }

    #[test]
    fn detects_port_in_use() {
        let raw = "[2026-06-23T12:00:00] ERROR: cannot listen for clients: address already in use";
        let p = parse_log_line(raw);
        assert!(matches!(
            p.events_buf.first(),
            Some(LogEvent::StateChange(DerivedState::Error))
        ));
    }
}
