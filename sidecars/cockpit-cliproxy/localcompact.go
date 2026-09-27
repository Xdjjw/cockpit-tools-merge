package main

// v21: 本地 compact 兜底。
// 厂家 /responses/compact 实测 503（纯净请求亦然），codex 归档/自动压缩通道
// 因此报废、会话只能滚雪球。此兜底直接在 43958 合成合法的 completed 响应，
// codex 视为压缩成功、会话随之瘦身；摘要为机械拼接（保留最近对话要点），
// 权威进度由 PROGRESS-NOTES.md 兜底。

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
)

func handleLocalCompact(c *gin.Context) {
	body, err := readAndRestoreBody(c.Request)
	if err != nil {
		writeAPIError(c, http.StatusBadRequest, "failed to read request body", "invalid_request")
		return
	}
	var probe struct {
		Input json.RawMessage `json:"input"`
	}
	if err := json.Unmarshal(body, &probe); err != nil {
		writeAPIError(c, http.StatusBadRequest, "invalid compact request JSON", "invalid_request")
		return
	}
	if len(probe.Input) == 0 {
		writeAPIError(c, http.StatusBadRequest, "compact input is required", "invalid_request")
		return
	}
	var arr []map[string]interface{}
	if err := json.Unmarshal(probe.Input, &arr); err != nil || arr == nil {
		writeAPIError(c, http.StatusBadRequest, "compact input must be an array", "invalid_request")
		return
	}

	pickText := func(item map[string]interface{}) (string, string) {
		role, _ := item["role"].(string)
		var sb strings.Builder
		switch content := item["content"].(type) {
		case string:
			sb.WriteString(content)
		case []interface{}:
			for _, seg := range content {
				if m, okSeg := seg.(map[string]interface{}); okSeg {
					if t, _ := m["type"].(string); t == "input_text" || t == "output_text" || t == "text" {
						if txt, okTxt := m["text"].(string); okTxt {
							sb.WriteString(txt)
						}
					}
				}
			}
		}
		return role, sb.String()
	}

	type highlight struct {
		role string
		text string
	}
	const keep = 8
	highlights := make([]highlight, 0, keep)
	for i := len(arr) - 1; i >= 0 && len(highlights) < keep; i-- {
		role, text := pickText(arr[i])
		text = strings.TrimSpace(text)
		if text == "" || (role != "user" && role != "assistant") {
			continue
		}
		highlights = append(highlights, highlight{role: role, text: compactTextLimit(text, 400)})
	}
	var sb strings.Builder
	sb.WriteString("[上下文本地压缩] 保留最近对话要点:")
	for i := len(highlights) - 1; i >= 0; i-- {
		sb.WriteString("\n[" + highlights[i].role + "] " + highlights[i].text)
	}
	if len(highlights) == 0 {
		sb.WriteString("\n(无近期文本消息)")
	}
	sb.WriteString("\n更早历史已归档; 权威进度以 PROGRESS-NOTES.md 为准。")

	now := time.Now().Unix()
	resp := map[string]interface{}{
		"id":           fmt.Sprintf("resp_shield_compact_%d", time.Now().UnixNano()),
		"object":       "response",
		"created_at":   now,
		"status":       "completed",
		"completed_at": now,
		"error":        nil,
		"output": []interface{}{
			map[string]interface{}{
				"type": "message",
				"role": "assistant",
				"id":   fmt.Sprintf("msg_shield_compact_%d", time.Now().UnixNano()),
				"content": []interface{}{
					map[string]interface{}{"type": "output_text", "text": sb.String()},
				},
			},
		},
		"usage": map[string]interface{}{
			"input_tokens": 0, "output_tokens": 0, "total_tokens": 0,
		},
		"model": "gpt-5.4-local-compact",
	}
	log.Printf("[shield] v21 local-compact served (kept=%d msgs)", len(highlights))
	c.JSON(http.StatusOK, resp)
}

func compactTextLimit(text string, maxRunes int) string {
	if maxRunes <= 0 {
		return ""
	}
	runes := []rune(text)
	if len(runes) <= maxRunes {
		return text
	}
	return string(runes[:maxRunes]) + "..."
}
