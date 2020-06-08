//! Various helper methods for common tasks.

/// Sanitizes unwanted or potentially dangerous characters and formatting from user input.
pub fn sanitize_user_input(i: &str) -> String {
    i.replace('@', "@\u{200B}")
}