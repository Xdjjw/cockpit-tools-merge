package main

// Provider Gateway 路径上的破甲阶梯闭环。
//
// 为什么单独一个文件: v1.3.60 把 Gateway 处理重构成了 provider_gateway.go 里的
// 一个大函数(视觉输入、历史投影、多智能体改写、调用 ID 归一、工具顺序修复),
// 而那些是**结构级**变换, 阶梯是**文本级**改写。把闭环拆出来调用, 才不会让
// 两者互相踩到。
//
// 调用点在 handleProviderGatewayRequest 中、结构级处理完成之后。见那里的注释。

import (
	"bytes"
	"fmt"
	"io"
	"log"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/router-for-me/CLIProxyAPI/v7/internal/runtime/executor/helps"
	"github.com/router-for-me/CLIProxyAPI/v7/sdk/translator"
)

// providerGatewayAttemptKind 区分一次尝试的结果类别。
//
// 关键区分: 只有"内容被拒"才值得升级重发。HTTP 429/5xx/网络错误升级改写层
// 是无用的 —— 那不是内容问题, 反复重试只会拖慢响应, 最后还会误报"阶梯走完仍被拦"。
type providerGatewayAttemptKind int

const (
	providerGatewayOK providerGatewayAttemptKind = iota
	providerGatewayRefused
	providerGatewayFatal
)

// providerGatewayLadderNonStream 在 Gateway 非流式路径上执行"被拒就升级并重发"。
//
// 与 ladderExecuteNonStream 的语义保持一致(见 ladder.go):
//   - 每轮都从原始 body 重新改写, 不在上一轮产地上叠加, 避免语义漂移;
//   - 通过时用本轮还原表把占位符回填成真值, 用户看到的是真实资产;
//   - 升到顶层仍被拒则如实报错(复用 ladderBuildExhaustedPayload), 不伪造成功;
//   - 传输/HTTP 错误直接透传, 不进阶梯。
//
// 为什么不复用 ladderExecuteNonStream: 那个走 executor 管线(runtime.Execute),
// 而 Gateway 是独立 HTTP 直连上游, 请求构造与响应形态都不同, 无法共用。
//
// 传入的 body 已经过上游化处理(历史投影等), 这里的 ladderProcess 只做文本层改写。
func (s *relayServer) providerGatewayLadderNonStream(
	c *gin.Context,
	gateway *providerGatewaySpec,
	body []byte,
	upstreamModel string,
	sourceFormat translator.Format,
	multiAgentV2Optimized bool,
) {
	cfg := currentLadderConfig()
	// 请求只复制持久化层级作为起点; 自动升级不能污染后续请求。
	maxAttempts := ladderRetryBudget(cfg)
	sessionKey := shieldRequestBodySessionKey(body)
	level := currentLadderLevel()
	attempts := 0

	for {
		attemptBody, abstract := ladderProcess(body, level)
		ladderLogAttempt(level, len(attemptBody), sessionKey)

		payload, headers, kind, status, reason := s.providerGatewayAttempt(
			c, gateway, attemptBody, upstreamModel, sourceFormat)

		switch kind {
		case providerGatewayFatal:
			// 透传上游错误, 不改写、不升级。封禁判定已在 attempt 内做过。
			writeUpstreamHeaders(c.Writer.Header(), headers)
			contentType := "application/json"
			if headers != nil {
				if ct := headers.Get("Content-Type"); ct != "" {
					contentType = ct
				}
			}
			if status < 400 {
				status = http.StatusBadGateway
			}
			c.Data(status, contentType, payload)
			return

		case providerGatewayOK:
			// 响应出口必须与 provider_gateway.go 的常规非流式路径保持**同样四步**,
			// 缺一步客户端就会看到中间态(例如 collaboration-optimize__send_message)。
			// 这是闭环绕过那段代码后必须自己补上的等价处理。
			payload = normalizeResponsesReasoningContentBody(payload)
			payload = helps.RestoreCodexMultiAgentV2Response(payload, multiAgentV2Optimized)
			payload = helps.NormalizeCodexCollaborationToolCalls(payload)
			payload = newProviderGatewayItemIDRewriter().RewritePayload(payload)
			if abstract != nil {
				// 把占位符回填成真值, 用户看到的目标仍是真实资产。
				payload = ladderApplyAbstractToResponse(payload, abstract)
			}
			if attempts > 0 {
				recordLadderRecovered()
				log.Printf("[ladder] gateway recovered at L%d/%s after %d retry(ies)",
					level, ladderLevelName(level), attempts)
			}
			contentType := "application/json"
			if headers != nil {
				if ct := headers.Get("Content-Type"); ct != "" {
					contentType = ct
				}
			}
			c.Data(http.StatusOK, contentType, payload)
			return
		}

		log.Printf("[ladder] gateway refused at L%d/%s reason=%s; escalating",
			level, ladderLevelName(level), reason)
		next, ok := nextLadderLevel(level)
		if !ok || attempts >= maxAttempts {
			// autoRetry 关闭: 用户不要重发, 如实返回上游原始产出。
			// 不能报"阶梯走完" —— 一轮都没重发, 那是假话。
			if !cfg.AutoRetry {
				log.Printf("[ladder] gateway refused at L%d/%s; autoRetry off, returning upstream payload",
					level, ladderLevelName(level))
				writeUpstreamHeaders(c.Writer.Header(), headers)
				contentType := "application/json"
				if headers != nil {
					if ct := headers.Get("Content-Type"); ct != "" {
						contentType = ct
					}
				}
				c.Data(http.StatusOK, contentType, payload)
				return
			}
			recordLadderFailed()
			log.Printf("[ladder] gateway exhausted at L%d/%s (reason=%s)",
				level, ladderLevelName(level), reason)
			c.Data(http.StatusOK, "application/json", ladderBuildExhaustedPayload(upstreamModel))
			return
		}
		recordLadderEscalation()
		level = next
		attempts++
	}
}

// providerGatewayAttempt 发一次 Gateway 请求。
//
// kind=OK 时 payload 是可直接写回客户端的响应体;
// kind=Fatal 时 payload 是上游错误体, 调用方应透传并带上 status。
//
// 这里只做 Responses wire API 的收发(调用方已限定), 因此不需要重跑
// chat_completions 的协议转换 —— 那一步在进入闭环前已经决定并排除了。
func (s *relayServer) providerGatewayAttempt(
	c *gin.Context,
	gateway *providerGatewaySpec,
	body []byte,
	upstreamModel string,
	sourceFormat translator.Format,
) (payload []byte, headers http.Header, kind providerGatewayAttemptKind, status int, reason string) {
	attemptBody := rewriteProviderGatewayBodyModel(body, upstreamModel)
	if isDeepSeekResponsesGateway(gateway.BaseURL) {
		// DeepSeek 思考模式要求回放的历史带 reasoning_text;
		// 响应出口为了兼容官方账号已把正文改写成 summary, 这里要还原回去。
		attemptBody = restoreResponsesReasoningTextForReplay(attemptBody)
	}
	attemptURL, urlErr := providerGatewayURL(gateway.BaseURL, "/v1/responses")
	if urlErr != nil {
		return nil, nil, providerGatewayFatal, http.StatusBadGateway, "bad gateway url"
	}

	req, reqErr := http.NewRequestWithContext(
		relayContext(c), http.MethodPost, attemptURL, bytes.NewReader(attemptBody))
	if reqErr != nil {
		return nil, nil, providerGatewayFatal, http.StatusBadGateway, "bad request"
	}
	req.Header.Set("Authorization", "Bearer "+gateway.APIKey)
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	copyProviderGatewayDiagnosticHeaders(req.Header, c.Request.Header)
	if isOpenCodeGoGateway(gateway.BaseURL) {
		applyOpenCodeSessionHeader(req.Header, c.Request.Header, attemptBody)
	}

	resp, doErr := http.DefaultClient.Do(req)
	if doErr != nil {
		return nil, nil, providerGatewayFatal, http.StatusBadGateway, "transport error"
	}
	defer resp.Body.Close()

	raw, readErr := io.ReadAll(resp.Body)
	if readErr != nil {
		return nil, nil, providerGatewayFatal, http.StatusBadGateway, "read error"
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		// 厂家会话级封禁: 计数 + 身份轮换 + 留痕。
		shieldInspectNonStreamError(resp.StatusCode, raw)
		// 透传上游状态码。限流/5xx 不是内容拒答, 升级改写层无济于事。
		return raw, resp.Header, providerGatewayFatal, resp.StatusCode,
			fmt.Sprintf("http %d", resp.StatusCode)
	}

	verdict := ladderInspectNonStreamBody(raw)
	if verdict.Refused {
		return nil, resp.Header, providerGatewayRefused, http.StatusOK, verdict.Reason
	}
	return raw, resp.Header, providerGatewayOK, http.StatusOK, ""
}
