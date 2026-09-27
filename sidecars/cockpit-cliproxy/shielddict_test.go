package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestDynamicDictHotReload 验证外置词表能加载并被 shieldObfuscate 使用。
// 封闭测试：写入临时词表并覆盖路径，不依赖真实 ~/.cockpit/shield-dict.json。
func TestDynamicDictHotReload(t *testing.T) {
	dir := t.TempDir()
	dictFile := filepath.Join(dir, "shield-dict.json")
	payload := map[string]map[string]string{
		"zh": {"新测试词": "DYNAMIC-TEST-OK"},
	}
	data, err := json.Marshal(payload)
	if err != nil {
		t.Fatalf("marshal dict: %v", err)
	}
	if err := os.WriteFile(dictFile, data, 0o644); err != nil {
		t.Fatalf("write temp dict: %v", err)
	}

	oldPath := shieldDictPath
	shieldDictPath = func() string { return dictFile }
	t.Cleanup(func() {
		shieldDictPath = oldPath
		dictMu.Lock()
		dynamicEnMap = cloneMap(shieldBucketMap)
		dynamicZhMap = cloneMap(shieldBucketMapZH)
		dictLoaded = false
		dictMu.Unlock()
	})

	loadShieldDict()
	zh := currentZhMap()
	if v, ok := zh["新测试词"]; !ok || v != "DYNAMIC-TEST-OK" {
		t.Fatalf("dict not loaded: 新测试词 => %q (ok=%v)", v, ok)
	}
	out := shieldObfuscate("这里是新测试词语句", 1)
	if !strings.Contains(out, "DYNAMIC-TEST-OK") {
		t.Fatalf("obfuscate did not apply dynamic zh word: %q", out)
	}
}

// TestDynamicDictFallbackKeepsBuiltinSeed 验证文件缺失时回退内置表。
func TestDynamicDictFallbackKeepsBuiltinSeed(t *testing.T) {
	dir := t.TempDir()
	oldPath := shieldDictPath
	shieldDictPath = func() string { return filepath.Join(dir, "missing-shield-dict.json") }
	t.Cleanup(func() {
		shieldDictPath = oldPath
		dictMu.Lock()
		dynamicEnMap = cloneMap(shieldBucketMap)
		dynamicZhMap = cloneMap(shieldBucketMapZH)
		dictLoaded = false
		dictMu.Unlock()
	})

	loadShieldDict()
	if _, ok := currentZhMap()["外挂"]; !ok {
		t.Fatal("builtin zh seed missing after fallback load")
	}
}

// TestDynamicDictSeed 验证内置种子仍可用（文件缺失时回退内置表）
func TestDynamicDictSeed(t *testing.T) {
	// 保留原文件内容用于恢复
	// 仅验证内置种子非空
	if len(shieldBucketMap) == 0 || len(shieldBucketMapZH) == 0 {
		t.Fatal("builtin seed dicts are empty")
	}
}
