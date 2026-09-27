package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"unicode/utf8"

	"github.com/gin-gonic/gin"
)

func testLocalCompactRequest(t *testing.T, body []byte) *httptest.ResponseRecorder {
	t.Helper()
	gin.SetMode(gin.TestMode)
	router := gin.New()
	router.POST("/v1/responses/compact", handleLocalCompact)
	req := httptest.NewRequest(http.MethodPost, "/v1/responses/compact", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)
	return w
}

func TestHandleLocalCompactRejectsMalformedInput(t *testing.T) {
	tests := []struct {
		name string
		body string
	}{
		{name: "invalid JSON", body: `{"input":`},
		{name: "missing input", body: `{"model":"gpt-5.5"}`},
		{name: "null input", body: `{"input":null}`},
		{name: "string input", body: `{"input":"hello"}`},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			w := testLocalCompactRequest(t, []byte(tc.body))
			if w.Code != http.StatusBadRequest {
				t.Fatalf("status=%d body=%s", w.Code, w.Body.String())
			}
		})
	}
}

func TestHandleLocalCompactSelectsRecentTextPastToolItems(t *testing.T) {
	input := []map[string]interface{}{{"role": "user", "content": "keep this message"}}
	for i := 0; i < 10; i++ {
		input = append(input, map[string]interface{}{"type": "function_call", "call_id": "call"})
	}
	payload, err := json.Marshal(map[string]interface{}{"model": "gpt-5.5", "input": input})
	if err != nil {
		t.Fatal(err)
	}
	w := testLocalCompactRequest(t, payload)
	if w.Code != http.StatusOK || !strings.Contains(w.Body.String(), "keep this message") {
		t.Fatalf("recent text was hidden by tool items: status=%d body=%s", w.Code, w.Body.String())
	}
}

func TestHandleLocalCompactTruncatesUnicodeSafely(t *testing.T) {
	payload, err := json.Marshal(map[string]interface{}{
		"model": "gpt-5.5",
		"input": []map[string]interface{}{
			{"role": "user", "content": strings.Repeat("界", 450)},
		},
	})
	if err != nil {
		t.Fatal(err)
	}
	w := testLocalCompactRequest(t, payload)
	if w.Code != http.StatusOK {
		t.Fatalf("status=%d body=%s", w.Code, w.Body.String())
	}
	var response struct {
		Status string `json:"status"`
		Output []struct {
			Content []struct {
				Text string `json:"text"`
			} `json:"content"`
		} `json:"output"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &response); err != nil {
		t.Fatal(err)
	}
	if response.Status != "completed" || len(response.Output) != 1 || len(response.Output[0].Content) != 1 {
		t.Fatalf("unexpected compact response: %#v", response)
	}
	text := response.Output[0].Content[0].Text
	if !utf8.ValidString(text) {
		t.Fatal("compact summary contains invalid UTF-8")
	}
	if !strings.Contains(text, strings.Repeat("界", 400)+"...") {
		t.Fatalf("unicode text was not truncated at a rune boundary: %q", text)
	}
}
