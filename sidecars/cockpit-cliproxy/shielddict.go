package main

// v26: 外置动态词表（热加载）。
// 词表从编译期常量改为 ~/.cockpit/shield-dict.json, 修改保存即生效
// (复用 shieldHotReloadLoop 的轮询周期), 新词不再需要改代码-编译-部署。
// 文件缺失/损坏时回退到内置种子词表。
//
// 文件格式:
// {
//   "en": {"createremotethread": "CTX-G1", ...},
//   "zh": {"外挂": "game enhancement client", ...}
// }
// en 走全词匹配(不破坏标识符), zh 走子串替换。

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"sync"
)

var (
	dictMu       sync.RWMutex
	dynamicEnMap = map[string]string{}
	dynamicZhMap = map[string]string{}
	dictLoaded   bool
)

// shieldDictPath is a func-var so tests can point the loader at a temp file
// instead of depending on the developer's real ~/.cockpit/shield-dict.json.
var shieldDictPath = func() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return "shield-dict.json"
	}
	return filepath.Join(home, ".cockpit", "shield-dict.json")
}

// loadShieldDict 读取外置词表; 出错时保留现有内存表(优雅降级)。
func loadShieldDict() {
	data, err := os.ReadFile(shieldDictPath())
	if err != nil {
		if !dictLoaded {
			// 首次: 用内置种子初始化内存表
			dictMu.Lock()
			dynamicEnMap = cloneMap(shieldBucketMap)
			dynamicZhMap = cloneMap(shieldBucketMapZH)
			dictMu.Unlock()
		}
		return
	}
	var parsed struct {
		En map[string]string `json:"en"`
		Zh map[string]string `json:"zh"`
	}
	if err := json.Unmarshal(data, &parsed); err != nil {
		log.Printf("[shield] dict parse failed (%v), keeping current table", err)
		return
	}
	en := cloneMap(shieldBucketMap)
	zh := cloneMap(shieldBucketMapZH)
	for k, v := range parsed.En {
		if k != "" && v != "" {
			en[k] = v // 外置覆盖内置
		}
	}
	for k, v := range parsed.Zh {
		if k != "" && v != "" {
			zh[k] = v
		}
	}
	dictMu.Lock()
	dynamicEnMap = en
	dynamicZhMap = zh
	dictLoaded = true
	dictMu.Unlock()
	log.Printf("[shield] dict loaded: en=%d zh=%d", len(en), len(zh))
}

func cloneMap(src map[string]string) map[string]string {
	out := make(map[string]string, len(src))
	for k, v := range src {
		out[k] = v
	}
	return out
}

func currentEnMap() map[string]string {
	dictMu.RLock()
	defer dictMu.RUnlock()
	if !dictLoaded {
		return shieldBucketMap
	}
	return dynamicEnMap
}

func currentZhMap() map[string]string {
	dictMu.RLock()
	defer dictMu.RUnlock()
	if !dictLoaded {
		return shieldBucketMapZH
	}
	return dynamicZhMap
}

// WriteSeedDict 把当前生效词表导出为外置文件, 便于用户在此基础上增删。
func WriteSeedDict() error {
	p := shieldDictPath()
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return err
	}
	seed := struct {
		En map[string]string `json:"en"`
		Zh map[string]string `json:"zh"`
	}{En: currentEnMap(), Zh: currentZhMap()}
	data, err := json.MarshalIndent(seed, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(p, data, 0o644)
}
