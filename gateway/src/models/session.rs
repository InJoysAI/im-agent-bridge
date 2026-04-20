/// Generate a standard `session_id` string from inbound message attributes.
///
/// Rules (BR-010, BR-011, criterion.md §5.3):
/// - `"private"` → `"{platform}:private:{chat_id}"`
/// - `"group"`   → `"{platform}:group:{chat_id}"`
/// - other       → `"{platform}:unknown:{chat_id}"` + WARN log (defensive, MVP guard)
pub fn generate_session_id(platform: &str, chat_type: &str, chat_id: &str) -> String {
    match chat_type {
        "private" => format!("{}:private:{}", platform, chat_id),
        "group" => format!("{}:group:{}", platform, chat_id),
        other => {
            tracing::warn!(
                platform,
                chat_type = other,
                chat_id,
                "unknown chat_type in generate_session_id, defaulting to 'unknown' segment"
            );
            format!("{}:unknown:{}", platform, chat_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 6.2.1
    #[test]
    fn private_generates_private_session_id() {
        let id = generate_session_id("telegram", "private", "111000001");
        assert_eq!(id, "telegram:private:111000001");
    }

    // 6.2.2
    #[test]
    fn group_generates_group_session_id() {
        let id = generate_session_id("telegram", "group", "444000004");
        assert_eq!(id, "telegram:group:444000004");
    }

    // 6.2.3 — same chat_id for private vs group → two distinct session_ids
    #[test]
    fn private_and_group_same_chat_id_are_distinct() {
        let private_id = generate_session_id("telegram", "private", "111000001");
        let group_id = generate_session_id("telegram", "group", "111000001");
        assert_ne!(private_id, group_id);
        assert_eq!(private_id, "telegram:private:111000001");
        assert_eq!(group_id, "telegram:group:111000001");
    }

    #[test]
    fn unknown_chat_type_produces_unknown_segment() {
        let id = generate_session_id("telegram", "supergroup", "999");
        assert_eq!(id, "telegram:unknown:999");
    }
}
