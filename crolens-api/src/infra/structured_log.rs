//! Structured logging module for CroLens API
//!
//! Provides JSON-formatted logs with trace_id and context for observability.

use serde::Serialize;
use worker::console_log;

/// Log levels for structured logging
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

/// Structured log entry
#[derive(Debug, Serialize)]
pub struct LogEntry<'a> {
    pub level: LogLevel,
    pub trace_id: &'a str,
    pub message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_size: Option<usize>,
    pub timestamp_ms: i64,
}

impl<'a> LogEntry<'a> {
    pub fn new(level: LogLevel, trace_id: &'a str, message: &'a str) -> Self {
        Self {
            level,
            trace_id,
            message,
            tool: None,
            api_key: None,
            client_ip: None,
            latency_ms: None,
            status: None,
            error_code: None,
            error_message: None,
            request_size: None,
            timestamp_ms: crate::types::now_ms(),
        }
    }

    pub fn with_tool(mut self, tool: &'a str) -> Self {
        self.tool = Some(tool);
        self
    }

    pub fn with_api_key(mut self, api_key: &'a str) -> Self {
        // Mask API key for security: show only prefix
        if api_key.len() > 10 {
            self.api_key = Some(&api_key[..10]);
        } else {
            self.api_key = Some(api_key);
        }
        self
    }

    pub fn with_client_ip(mut self, ip: &'a str) -> Self {
        self.client_ip = Some(ip);
        self
    }

    pub fn with_latency(mut self, latency_ms: i64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    pub fn with_status(mut self, status: &'a str) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_error(mut self, code: i32, message: &'a str) -> Self {
        self.error_code = Some(code);
        self.error_message = Some(message);
        self
    }

    pub fn with_request_size(mut self, size: usize) -> Self {
        self.request_size = Some(size);
        self
    }

    /// Output the log entry as JSON
    pub fn emit(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            console_log!("{}", json);
        }
    }
}

/// Request context for logging
pub struct RequestContext<'a> {
    pub trace_id: &'a str,
    pub api_key: Option<&'a str>,
    pub client_ip: &'a str,
    pub start_ms: i64,
}

impl<'a> RequestContext<'a> {
    pub fn new(
        trace_id: &'a str,
        api_key: Option<&'a str>,
        client_ip: &'a str,
        start_ms: i64,
    ) -> Self {
        Self {
            trace_id,
            api_key,
            client_ip,
            start_ms,
        }
    }

    /// Log the start of a request
    pub fn log_request_start(&self, tool: &str) {
        let mut entry = LogEntry::new(LogLevel::Info, self.trace_id, "request_start")
            .with_tool(tool)
            .with_client_ip(self.client_ip);

        if let Some(key) = self.api_key {
            entry = entry.with_api_key(key);
        }

        entry.emit();
    }

    /// Log the completion of a request
    pub fn log_request_complete(&self, tool: &str, status: &str) {
        let latency = crate::types::now_ms().saturating_sub(self.start_ms);
        let mut entry = LogEntry::new(LogLevel::Info, self.trace_id, "request_complete")
            .with_tool(tool)
            .with_client_ip(self.client_ip)
            .with_latency(latency)
            .with_status(status);

        if let Some(key) = self.api_key {
            entry = entry.with_api_key(key);
        }

        entry.emit();
    }

    /// Log a request error
    pub fn log_request_error(&self, tool: &str, error_code: i32, error_message: &str) {
        let latency = crate::types::now_ms().saturating_sub(self.start_ms);
        let mut entry = LogEntry::new(LogLevel::Error, self.trace_id, "request_error")
            .with_tool(tool)
            .with_client_ip(self.client_ip)
            .with_latency(latency)
            .with_status("error")
            .with_error(error_code, error_message);

        if let Some(key) = self.api_key {
            entry = entry.with_api_key(key);
        }

        entry.emit();
    }
}

/// Convenience macros for structured logging
#[macro_export]
macro_rules! log_info {
    ($trace_id:expr, $message:expr) => {
        $crate::infra::log::LogEntry::new(
            $crate::infra::log::LogLevel::Info,
            $trace_id,
            $message,
        )
        .emit()
    };
    ($trace_id:expr, $message:expr, $($field:ident = $value:expr),* $(,)?) => {{
        let mut entry = $crate::infra::log::LogEntry::new(
            $crate::infra::log::LogLevel::Info,
            $trace_id,
            $message,
        );
        $(
            entry = entry.$field($value);
        )*
        entry.emit()
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($trace_id:expr, $message:expr) => {
        $crate::infra::log::LogEntry::new(
            $crate::infra::log::LogLevel::Warn,
            $trace_id,
            $message,
        )
        .emit()
    };
}

#[macro_export]
macro_rules! log_error {
    ($trace_id:expr, $message:expr) => {
        $crate::infra::log::LogEntry::new(
            $crate::infra::log::LogLevel::Error,
            $trace_id,
            $message,
        )
        .emit()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            level: LogLevel::Info,
            trace_id: "abc123",
            message: "test message",
            tool: Some("get_account_summary"),
            api_key: Some("cl_sk_test"),
            client_ip: Some("203.0.113.1"),
            latency_ms: Some(150),
            status: Some("success"),
            error_code: None,
            error_message: None,
            request_size: Some(256),
            timestamp_ms: 1700000000000,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"trace_id\":\"abc123\""));
        assert!(json.contains("\"tool\":\"get_account_summary\""));
        assert!(json.contains("\"latency_ms\":150"));
        // Error fields should be omitted
        assert!(!json.contains("error_code"));
        assert!(!json.contains("error_message"));
    }

    #[test]
    fn test_log_entry_builder() {
        // Create entry directly without using ::new() to avoid now_ms() call
        let mut entry = LogEntry {
            level: LogLevel::Error,
            trace_id: "trace1",
            message: "something failed",
            tool: None,
            api_key: None,
            client_ip: None,
            latency_ms: None,
            status: None,
            error_code: None,
            error_message: None,
            request_size: None,
            timestamp_ms: 1700000000000,
        };
        entry = entry.with_tool("decode_transaction").with_error(-32602, "Invalid params");

        assert_eq!(entry.level.as_str(), "error");
        assert_eq!(entry.tool, Some("decode_transaction"));
        assert_eq!(entry.error_code, Some(-32602));
        assert_eq!(entry.error_message, Some("Invalid params"));
    }

    #[test]
    fn test_api_key_masking() {
        // Create entry directly without using ::new() to avoid now_ms() call
        let mut entry = LogEntry {
            level: LogLevel::Info,
            trace_id: "t1",
            message: "test",
            tool: None,
            api_key: None,
            client_ip: None,
            latency_ms: None,
            status: None,
            error_code: None,
            error_message: None,
            request_size: None,
            timestamp_ms: 1700000000000,
        };
        entry = entry.with_api_key("cl_sk_verylongapikey123456");

        // Only first 10 chars should be stored
        assert_eq!(entry.api_key, Some("cl_sk_very"));
    }

    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }
}
