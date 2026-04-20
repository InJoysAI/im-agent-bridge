use uuid::Uuid;

use crate::models::inbound::InboundRequest;

#[derive(Debug, Clone)]
pub struct StandardMessage {
    pub event_id: String,
    pub platform: String,
    pub bridge_gateway_name: String,
    pub bridge_channel_name: Option<String>,
    pub bridge_message_id: String,
    pub chat_id: String,
    pub chat_type: String,
    pub user_id: String,
    pub session_id: String,
    pub text: String,
    pub timestamp: String,
    pub bot_id: Uuid,
}

impl StandardMessage {
    pub fn build(req: &InboundRequest, bot_id: Uuid, session_id: &str, event_id: String) -> Self {
        Self {
            event_id,
            platform: req.platform.clone(),
            bridge_gateway_name: req.bridge_gateway_name.clone(),
            bridge_channel_name: req.bridge_channel_name.clone(),
            bridge_message_id: req.raw_message.message_id.clone(),
            chat_id: req.raw_message.chat_id.clone(),
            chat_type: req.raw_message.chat_type.as_str().to_string(),
            user_id: req.raw_message.user_id.clone(),
            session_id: session_id.to_string(),
            text: req.raw_message.text.clone().unwrap_or_default(),
            timestamp: req.raw_message.timestamp.clone(),
            bot_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_inbound_request() -> InboundRequest {
        serde_json::from_value(serde_json::json!({
            "platform": "telegram",
            "bridge_gateway_name": "default",
            "bridge_channel_name": "general",
            "raw_message": {
                "chat_id": "chat-1",
                "chat_type": "private",
                "user_id": "user-1",
                "message_type": "text",
                "text": "hello",
                "timestamp": "2026-04-16T00:00:00Z",
                "message_id": "msg-1"
            }
        }))
        .unwrap()
    }

    #[test]
    fn build_generates_uuid_v4_event_id() {
        let req = sample_inbound_request();
        let bot_id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();

        let msg = StandardMessage::build(
            &req,
            bot_id,
            "telegram:private:chat-1",
            Uuid::new_v4().to_string(),
        );

        let parsed = Uuid::parse_str(&msg.event_id).expect("event_id should be a valid UUID");
        assert_eq!(parsed.get_version_num(), 4, "event_id must be UUID v4");
    }

    #[test]
    fn build_populates_required_fields_non_empty() {
        let req = sample_inbound_request();
        let bot_id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();

        let msg = StandardMessage::build(
            &req,
            bot_id,
            "telegram:private:chat-1",
            Uuid::new_v4().to_string(),
        );

        assert!(!msg.event_id.is_empty());
        assert!(!msg.platform.is_empty());
        assert!(!msg.chat_id.is_empty());
        assert!(!msg.chat_type.is_empty());
        assert!(!msg.user_id.is_empty());
        assert!(!msg.session_id.is_empty());
        assert!(!msg.text.is_empty());
        assert!(!msg.timestamp.is_empty());
    }
}
