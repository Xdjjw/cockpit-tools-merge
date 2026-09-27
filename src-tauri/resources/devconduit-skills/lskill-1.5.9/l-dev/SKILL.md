---
name: l-dev
description: "Internal service maintenance route; disabled for ordinary client delivery."
---

# l-dev · internal

Reserved for maintainers of the local license and skills services. Review
schema migrations, seed manifests, namespace isolation, session binding, and
loopback startup only. Never use this route to connect to production, move
production credentials, or change a production database.

The service keeps public and internal resources physically separate. Changes
must include focused tests and a redacted handoff note without credentials.

## 深度模块（本地离线，直接 Read）

完成上面的 baseline 后，如果任务命中下列某个子领域，Read 对应本地文件，并按其中 Scope / Safety Rules 执行。每次只读真正命中的那一个。

本地路径：
`C:/Users/jjwde/.codex/skills/_offline/modules/<MODULE_ID>.md`

不要调本机桥、云兑换接口或远程技能服务，也不要依赖破甲助手。模块正文已经全部落在上述目录。

