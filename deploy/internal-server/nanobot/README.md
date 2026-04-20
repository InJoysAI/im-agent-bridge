# NanoBot Runtime 部署说明

NanoBot 作为 AI Agent Runtime，运行于 Internal Server，通过 Gateway Runtime Adapter 接收请求，并调用 Shopify MCP 子进程完成多店铺数据查询。

## 目录结构

```
nanobot/
├── Dockerfile               # NanoBot 镜像（Python + Node.js）
├── docker-compose.yml       # 容器编排
├── config.json.example      # NanoBot 配置模板（含 LLM + mcpServers）
├── .env.example             # 凭证变量模板（LLM + Shopify 多店铺）
├── memory/
│   ├── MEMORY.md.example    # MEMORY.md 格式样板（TAD §9.4.1）
│   └── MEMORY.md            # 实际 MEMORY.md（不入库，volume 挂载至容器）
└── nanobot-data/            # 运行时数据目录（volume 挂载，不入库）
```

## 初次部署

```bash
# 1. 复制并填写 NanoBot 配置（含真实 LLM API Key 和 Shopify 凭证）
cp config.json.example nanobot-data/config.json
# 编辑 nanobot-data/config.json，将 ${VAR} 占位符替换为实际值

# 2. 复制并填写凭证环境变量
cp .env.example .env
# 编辑 .env，填入真实 LLM API Key 和 Shopify 凭证

# 3. 复制并填写 MEMORY.md（AI Agent 上下文声明）
cp memory/MEMORY.md.example memory/MEMORY.md
# 编辑 memory/MEMORY.md，按实际店铺信息填写 MCP Server 列表，删除注释

# 4. 启动容器
docker compose up -d
```

## 如何增加新店铺

1. **`.env`**：追加三行凭证变量（将 `STOREN` 替换为实际 slug 大写形式）：
   ```
   SHOPIFY_STOREN_CLIENT_ID=your-client-id
   SHOPIFY_STOREN_CLIENT_SECRET=your-client-secret
   SHOPIFY_STOREN_DOMAIN=storen.myshopify.com
   ```

2. **`nanobot-data/config.json`** → `tools.mcpServers`：新增条目（参考 `config.json.example`）：
   ```json
   "shopify-storen": {
     "command": "npx",
     "args": [
       "shopify-mcp",
       "--clientId", "${SHOPIFY_STOREN_CLIENT_ID}",
       "--clientSecret", "${SHOPIFY_STOREN_CLIENT_SECRET}",
       "--domain", "${SHOPIFY_STOREN_DOMAIN}"
     ],
     "toolTimeout": 10
   }
   ```

3. **`memory/MEMORY.md`** → `## MCP Servers` 表格：追加新店铺一行。

4. 重启 NanoBot 容器使配置生效：
   ```bash
   docker compose restart nanobot
   docker compose logs nanobot --tail=50
   ```

## 安全注意事项

- `config.json`（实际文件）和 `.env` 已在根目录 `.gitignore` 中，**严禁提交至代码仓库**
- Shopify 凭证仅通过 `${VAR}` 语法引用，禁止硬编码（criterion.md §4）
- PostgreSQL 中不存储任何 MCP 配置或凭证（criterion.md §3.7 MUST NOT）
- MCP 工具调用审计日志输出至 NanoBot stdout/文件，不入库（BR-034）

## 超时与降级

- MCP 工具调用超时上限：**10s**（`toolTimeout: 10`，BR-052）
- 超时后 NanoBot 自动返回降级提示，不挂起 Gateway（Gateway hard timeout 15s）
- 错误码 `MCP_TIMEOUT` / `MCP_UNAVAILABLE` 由 Gateway Runtime Adapter 处理（criterion.md §5.4）
