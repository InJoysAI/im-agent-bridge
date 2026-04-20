use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Matterbridge `/api/messages` 轮询间隔。
///
/// 选择 1s 的理由：
/// - Matterbridge `Buffer=1000` 足以覆盖 1s 窗口内的消息堆积；
/// - IM 对话端到端延迟 +1s 对体验可接受；
/// - 轮询频率过高会浪费网络/CPU，且无益于实时性。
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// 连续轮询失败的最大等待退避（不让瞬时网络抖动无限刷错误日志）。
const POLL_ERROR_BACKOFF: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct MbMessage {
    #[serde(default)]
    text: String,
    #[serde(default)]
    channel: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    userid: String,
    #[serde(default)]
    account: String,
    #[serde(default)]
    protocol: String,
    #[serde(default)]
    gateway: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    event: String,
    #[serde(default)]
    parent_id: String,
    // Extra 如果需要可以加 #[serde(default)] extra: Option<Value>,
}

#[derive(Debug, Serialize)]
struct RawMessage {
    chat_id: String,
    chat_type: String,
    user_id: String,
    message_type: String,
    timestamp: String,
    message_id: String,
    sender_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Debug, Serialize)]
struct InboundRequest {
    platform: String,
    bridge_gateway_name: String,
    raw_message: RawMessage,
}

fn infer_telegram_chat_type(chat_id: &str) -> &'static str {
    // Telegram: private chat_id 通常为正整数，群组/超级群通常为负整数（例如 -100...）。
    // 若格式异常则回退为 group，保持与历史行为兼容并避免误放过群聊 mention 过滤。
    match chat_id.trim().parse::<i64>() {
        Ok(id) if id > 0 => "private",
        _ => "group",
    }
}

pub async fn run_poller(mb_url: String, gateway_self_url: String, gateway_bearer_token: String) {
    // `/api/messages` 是 Matterbridge 的 drain 端点：每次 GET 返回并清空队列，
    // **服务器端无订阅者状态**，因此 Gateway 任意次重启都不需要重启 Matterbridge。
    // 注意：不设置总体 `.timeout()`。
    // Poller 会 POST 到自身 /gateway/inbound，该端点内部会串行调用 NanoBot（可能 60s+）+ Bridge
    // 回写。若在此处设超时会把 forward 请求中断，inbound handler 虽仍继续，但 poller 看到假失败。
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .tcp_keepalive(Duration::from_secs(30))
        .http1_only()
        .no_proxy()
        .build()
        .expect("failed to build reqwest client");

    info!(
        "Matterbridge poller started (mode=poll /api/messages interval={:?})",
        POLL_INTERVAL
    );

    loop {
        match poll_once(&client, &mb_url, &gateway_self_url, &gateway_bearer_token).await {
            Ok(count) => {
                if count > 0 {
                    debug!("poll_once drained {} message(s)", count);
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Err(e) => {
                error!(
                    "Matterbridge poll error: {} — retrying in {:?}",
                    e, POLL_ERROR_BACKOFF
                );
                tokio::time::sleep(POLL_ERROR_BACKOFF).await;
            }
        }
    }
}

/// 执行一次 `GET /api/messages`，把队列中的消息全部转发给 Gateway inbound。
/// 返回本次处理的有效消息数量（不含被过滤的 api 回环消息）。
async fn poll_once(
    client: &Client,
    mb_url: &str,
    gateway_self_url: &str,
    gateway_bearer_token: &str,
) -> anyhow::Result<usize> {
    let url = format!("{}/api/messages", mb_url.trim_end_matches('/'));
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "GET /api/messages returned {}",
            resp.status()
        ));
    }

    let msgs: Vec<MbMessage> = resp.json().await?;
    let mut forwarded = 0usize;

    for msg in msgs {
        // 过滤 api 回环消息（Gateway 自己通过 POST /api/message 发出的内容理论上不回放回环；
        // 保留判定为双保险）。
        if msg.protocol == "api" || msg.account.starts_with("api.") {
            continue;
        }
        if msg.event == "api_connected" {
            continue;
        }
        if msg.gateway.trim().is_empty() {
            error!(
                "Matterbridge message missing required `gateway`; dropping message [channel={} account={} protocol={}]",
                msg.channel, msg.account, msg.protocol
            );
            continue;
        }

        info!(
            "Received message from {} (protocol: {})",
            msg.account, msg.protocol
        );

        // 关键：把 forward_inbound 放到独立 task 中执行。
        // inbound handler 内部会串行调用 NanoBot (最长 60s) + Bridge 回写，单消息处理耗时
        // 可达 60s+。若在 poll_once 内 await，会导致 poller 阻塞，后续消息堆积在 Matterbridge
        // `Buffer=1000` 队列中（仍不会丢，但端到端延迟被放大）。
        let client = client.clone();
        let self_url = gateway_self_url.to_string();
        let token = gateway_bearer_token.to_string();
        tokio::spawn(async move {
            forward_inbound(&client, &msg, &self_url, &token).await;
        });
        forwarded += 1;
    }

    Ok(forwarded)
}

async fn forward_inbound(
    client: &Client,
    msg: &MbMessage,
    gateway_self_url: &str,
    gateway_bearer_token: &str,
) {
    let message_type = if !msg.text.is_empty() {
        "text"
    } else {
        "other"
    };
    let user_id = if !msg.userid.is_empty() {
        msg.userid.clone()
    } else {
        msg.username.clone()
    };

    // Matterbridge 1.26.0 对部分协议（含 Telegram）不一定填充 `id`。
    // 为避免所有 `id=""` 的消息碰撞到同一幂等键导致 duplicate_ignored，
    // 当 id 为空时回退到 {channel}:{userid}:{timestamp} 的稳定组合键：
    // 同一条消息重投仍可去重；不同消息因 timestamp 差异不会误判。
    let effective_message_id = if !msg.id.is_empty() {
        msg.id.clone()
    } else {
        let fallback = format!(
            "fallback:{}:{}:{}",
            msg.channel,
            if !msg.userid.is_empty() {
                &msg.userid
            } else {
                &msg.username
            },
            msg.timestamp
        );
        warn!(
            "Matterbridge message missing `id` field; using fallback idempotency key: {}",
            fallback
        );
        fallback
    };

    let raw_message = RawMessage {
        chat_id: msg.channel.clone(),
        chat_type: infer_telegram_chat_type(&msg.channel).to_string(),
        user_id,
        message_type: message_type.to_string(),
        timestamp: msg.timestamp.clone(),
        message_id: effective_message_id,
        sender_name: msg.username.clone(),
        text: if message_type == "text" {
            Some(msg.text.clone())
        } else {
            None
        },
    };

    let gateway_name = msg.gateway.clone();

    let inbound = InboundRequest {
        platform: "telegram".to_string(),
        bridge_gateway_name: gateway_name.clone(),
        raw_message,
    };

    match client
        .post(format!("{}/gateway/inbound", gateway_self_url))
        .bearer_auth(gateway_bearer_token)
        .json(&inbound)
        .send()
        .await
    {
        Ok(resp) => {
            info!(
                "Forwarded [gateway={} channel={}] → self gateway/inbound → status: {}",
                gateway_name,
                msg.channel,
                resp.status()
            );
        }
        Err(e) => error!("Failed to forward inbound message: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::infer_telegram_chat_type;

    #[test]
    fn infer_private_chat_type_from_positive_chat_id() {
        assert_eq!(infer_telegram_chat_type("123456789"), "private");
    }

    #[test]
    fn infer_group_chat_type_from_negative_chat_id() {
        assert_eq!(infer_telegram_chat_type("-1001234567890"), "group");
    }

    #[test]
    fn fallback_to_group_when_chat_id_is_not_numeric() {
        assert_eq!(infer_telegram_chat_type("not-a-number"), "group");
    }
}
