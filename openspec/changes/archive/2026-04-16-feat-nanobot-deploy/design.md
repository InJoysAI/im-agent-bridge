## Context

`feat-nanobot-deploy` 需要构建 NanoBot 容器镜像，运行时同时需要 Python 3.10+（NanoBot Runtime）和 Node.js LTS（后续 `npx shopify-mcp` 子进程，预置于本提案 Dockerfile 中）。标准 Python Docker 镜像不含 Node.js，需要决策安装方案。同时需明确 volume 挂载路径与 NanoBot 配置目录的对应关系，以及 secret 注入机制。

## Goals / Non-Goals
- Goals:
  - 确定 Dockerfile 基础镜像及 Node.js 安装方式
  - 明确 volume 挂载路径与 NanoBot `~/.nanobot/` 配置目录的映射
  - 明确 `config.json` secret 引用机制（`${VAR}` + Docker Compose `env_file`）
- Non-Goals:
  - Shopify MCP 配置（`tools.mcpServers`）——由 `feat-nanobot-shopify-mcp` 处理
  - 生产环境资源规格优化（CPU/Memory limits）

## Decisions

- **Decision 1：Dockerfile 基础镜像选型**
  - 选择：`python:3.10-slim`（Debian Bullseye/Bookworm），通过官方 nodesource 脚本安装 Node.js LTS
  - 备选方案：
    - `ubuntu:22.04` + pip + node：镜像体积大（~600MB vs ~250MB），不选
    - 多阶段构建（独立 node + python 阶段 copy）：复杂度高于收益（MVP 阶段无需极致瘦身），不选
  - 原因：`python:3.10-slim` + nodesource 是业界成熟方案，体积小、维护成本低，满足 Python 3.10+ + Node.js LTS 双运行时需求

- **Decision 2：Volume 挂载路径**
  - 选择：`./nanobot-data:/home/nanobot/.nanobot`
  - 依据：`deployment_view.md` 明确规定此挂载路径；NanoBot 默认配置目录为 `~/.nanobot/`（容器内 home 目录）；`config.json` 与对话记忆文件均落于此目录
  - RISK-002 缓解：宿主机 `./nanobot-data/` 持久化，容器重启不丢失上下文

- **Decision 3：Secret 注入机制**
  - 选择：`config.json.example` 使用 `${VAR_NAME}` 语法，配合 Docker Compose `env_file: .env` 在运行时注入实际值
  - 依据：NanoBot 原生支持 `${VAR_NAME}` 语法从环境变量读取 secret（`deployment_view.md`）；符合 `criterion.md §4` 凭证管理要求（禁止硬编码）
  - 约束：实际 `config.json`（含真实 API Key）须加入 `.gitignore`；`config.json.example` 仅含占位符，可安全提交

## Risks / Trade-offs

- `python:3.10-slim` + nodesource 方案在网络受限/离线环境需配置 apt mirror 或 Docker registry mirror
- `nanobot-ai[api]` 具体版本和 `nanobot serve` 命令行参数需以 [HKUDS/nanobot](https://github.com/HKUDS/nanobot) 当前最新官方文档为准，可能随版本演进（tech_stack.md 注意项）

## Open Questions

- NanoBot 容器内实际 home 目录是否为 `/home/nanobot`？需以实际镜像运行验证（若 CMD user 不同，挂载路径须调整）
