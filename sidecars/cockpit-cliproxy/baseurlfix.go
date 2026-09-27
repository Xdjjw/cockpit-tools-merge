package main

// v22: 渠道 base-url 规范化（内存层）。
// Cockpit 账号库里 3 个渠道的 base-url 存的是 https://ai.input.im（无 /v1），
// executor 拼 url = baseURL + "/responses" 会打到 https://ai.input.im/responses
// -> 厂家 INVALID_API_KEY（间歇性 401，只有带 /v1 的渠道 1 能通）。
// 本补丁在 config 加载后的 shieldPatchConfig 里统一修正，Cockpit 重写配置也不受影响。

import (
	"log"
	"net/url"
	"strings"

	"github.com/router-for-me/CLIProxyAPI/v7/internal/config"
)

func shieldNormalizeCodexBaseURLs(cfg *config.Config) {
	if cfg == nil {
		return
	}
	fixed := 0
	for i := range cfg.CodexKey {
		u := strings.TrimSpace(cfg.CodexKey[i].BaseURL)
		if u == "" {
			continue
		}
		u = strings.TrimSuffix(u, "/")
		parsed, err := url.Parse(u)
		if err != nil || parsed.Host == "" {
			continue
		}
		if parsed.Path == "" || parsed.Path == "/" {
			cfg.CodexKey[i].BaseURL = u + "/v1"
			fixed++
		}
	}
	if fixed > 0 {
		log.Printf("[shield] v22 base-url normalize: appended /v1 to %d channel(s)", fixed)
	}
}
