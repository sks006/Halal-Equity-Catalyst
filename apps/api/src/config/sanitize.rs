/// Masks credentials inside a connection URL before logging.
pub fn sanitize_connection_url(raw: &str) -> String {
    if let Some((proto, rest)) = raw.split_once("://") {
        if let Some((creds, host_part)) = rest.split_once('@') {
            if let Some((user, _pass)) = creds.split_once(':') {
                return format!("{}://{}:***@{}", proto, user, host_part);
            }
            return format!("{}://***@{}", proto, host_part);
        }
    }
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_password() {
        assert_eq!(
            sanitize_connection_url("postgres://alice:secret@db.example/x"),
            "postgres://alice:***@db.example/x"
        );
    }

    #[test]
    fn passes_through_when_no_creds() {
        assert_eq!(
            sanitize_connection_url("redis://localhost:6379"),
            "redis://localhost:6379"
        );
    }
}
