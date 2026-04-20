import "@typespec/http";
import "@typespec/openapi";
import "@typespec/openapi3";

using TypeSpec.Http;
using OpenAPI;

@service(#{ title: "IM Agent Bridge Gateway API" })
@info(#{ version: "1.0.0" })
namespace ImAgentBridge;

// ============================================================
// Shared Models
// ============================================================

/** Bridge 入站消息中的原始消息体 */
model RawMessage {
  chat_id: string;
  chat_type: "private" | "group";
  user_id: string;
  message_type: "text" | "image" | "audio" | "video" | "file" | "other";
  text?: string;  // 仅 message_type=text 时携带；非文本类型 Gateway 返回 400 并忽略（BR-001）
  timestamp: string;
  message_id: string;
  sender_name?: string;
}

/** Bridge → Gateway 入站请求 */
model InboundRequest {
  platform: string;
  bridge_gateway_name: string;
  bridge_channel_name?: string;
  raw_message: RawMessage;
}

/** 入站响应 */
model InboundResponse {
  status: "accepted" | "ignored_duplicate" | "ignored_no_mention";
}

/** Gateway → Bridge 回写请求 */
model ReplyRequest {
  reply_id: string;
  chat_id: string;
  platform: string;
  text: string;
  bridge_gateway_name: string;
  bridge_channel_name?: string;
}

/** 标准消息请求对象 (Gateway → Runtime) */
model RuntimeProcessRequest {
  event_id: string;
  platform: string;
  chat_id: string;
  chat_type: "private" | "group";
  user_id: string;
  session_id: string;
  text: string;
  timestamp: string;
  bot_id: string;
  metadata?: Record<string>;
}

/** 标准回复对象 (Runtime → Gateway) */
model RuntimeProcessResponse {
  reply_id: string;
  bot_id: string;
  platform: string;
  chat_id: string;
  reply_type: "text";
  text: string;
  session_id: string;
  status: "success" | "error";
  metadata?: Record<string>;
}

// ============================================================
// Bridge ↔ Gateway Endpoints
// ============================================================

@route("/gateway")
namespace Gateway {
  /** 入站：Matterbridge → Gateway */
  @route("/inbound")
  @post
  op inbound(@header authorization: string, @body body: InboundRequest): {
    @statusCode statusCode: 200;
    @body body: InboundResponse;
  } | {
    @statusCode statusCode: 400;
    @body body: { error: string };
  } | {
    @statusCode statusCode: 401;
    @body body: { error: string };
  } | {
    @statusCode statusCode: 404;
    @body body: { error: string };
  } | {
    @statusCode statusCode: 429;
    @body body: { error: string };
  } | {
    @statusCode statusCode: 503;
    @body body: { error: string };
  };
}

@route("/bridge")
namespace Bridge {
  /** 回写：Gateway → Matterbridge */
  @route("/reply")
  @post
  op reply(@header authorization: string, @body body: ReplyRequest): {
    @statusCode statusCode: 200;
    @body body: { status: "ok" };
  } | {
    @statusCode statusCode: 400;
    @body body: { error: string };
  } | {
    @statusCode statusCode: 401;
    @body body: { error: string };
  } | {
    @statusCode statusCode: 409;
    @body body: { error: string };
  };
}

// ============================================================
// Gateway ↔ Runtime Endpoint
// ============================================================

@route("/runtime")
namespace Runtime {
  /** 处理：Gateway → Runtime */
  @route("/process")
  @post
  op process(@body body: RuntimeProcessRequest): {
    @statusCode statusCode: 200;
    @body body: RuntimeProcessResponse;
  };
}

// ============================================================
// Health Check
// ============================================================

@route("/health")
op health(): { status: "ok" };
