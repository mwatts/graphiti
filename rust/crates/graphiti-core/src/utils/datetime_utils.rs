/*
Copyright 2024, Zep Software, Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

//! Date and time utilities

use chrono::{DateTime, Utc};

/// Get current UTC time
pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

/// Format datetime for database storage
pub fn format_for_db(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Parse datetime from database string
pub fn parse_from_db(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}

/// Convert timestamp to DateTime
pub fn from_timestamp(timestamp: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(timestamp, 0)
}

/// Convert DateTime to timestamp
pub fn to_timestamp(dt: DateTime<Utc>) -> i64 {
    dt.timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utc_now() {
        let now = utc_now();
        assert!(now.timestamp() > 0);
    }

    #[test]
    fn test_format_and_parse() {
        let now = utc_now();
        let formatted = format_for_db(now);
        let parsed = parse_from_db(&formatted).unwrap();

        // Allow for small differences due to rounding
        assert!((now.timestamp() - parsed.timestamp()).abs() <= 1);
    }
}
