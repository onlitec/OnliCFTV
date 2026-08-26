use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use chrono::Utc;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[derive(Clone)]
pub struct LogStore {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
}

impl LogStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub fn log(&self, level: &str, target: &str, message: &str) {
        let sanitized = sanitize_credentials(message);
        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            target: target.to_string(),
            message: sanitized,
        };

        let mut lock = self.entries.lock().unwrap();
        if lock.len() >= self.max_entries {
            lock.pop_front();
        }
        lock.push_back(entry);
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        let lock = self.entries.lock().unwrap();
        lock.iter().cloned().collect()
    }

    pub fn clear(&self) {
        let mut lock = self.entries.lock().unwrap();
        lock.clear();
    }
}

/// Sanitizes passwords and credentials from log messages
pub fn sanitize_credentials(input: &str) -> String {
    // Replace rtsp://username:password@host with rtsp://username:***@host
    let rtsp_regex = regex_lite_sanitize(input);
    rtsp_regex
}

fn regex_lite_sanitize(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        if let Some(pos) = input[i..].find("rtsp://") {
            let start = i + pos + 7;
            result.push_str(&input[i..start]);
            
            // Check if there is a '@' before next '/' or whitespace
            let rest = &input[start..];
            if let Some(at_pos) = rest.find('@') {
                let user_pass = &rest[..at_pos];
                if let Some(colon_pos) = user_pass.find(':') {
                    let user = &user_pass[..colon_pos];
                    result.push_str(user);
                    result.push_str(":***@");
                    i = start + at_pos + 1;
                    continue;
                }
            }
            i = start;
        } else {
            result.push_str(&input[i..]);
            break;
        }
    }
    result
}
