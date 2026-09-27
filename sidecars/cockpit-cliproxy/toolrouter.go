package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/gin-gonic/gin"
)

// writeStateFileAtomic 原子落盘: 先写同目录临时文件, 再 Rename 覆盖目标。
//
// 为什么必须原子: Go 侧每 3 秒轮询这两个状态文件, 直接写会留下被读到半截 JSON
// 的窗口 —— 加载器解析失败就保留旧值, 表现为"改了配置却没生效"。ladder 与
// toolrouter 的状态落盘都走这里, 所以名字是中性描述, 不是 toolrouter 专用。
//
// 关于 Windows 的 Rename(已在 Windows / go1.26 实测): os.Rename **能**覆盖已存在
// 的目标文件, 因为 Go 内部用 MoveFileEx + MOVEFILE_REPLACE_EXISTING。
//
// 这里曾经有个"Rename 失败就先删目标再改名"的回退, 依据是"Windows 不能覆盖"——
// 该前提是错的(那是 Python os.rename 的语义, Go 不是)。回退反而破坏原子性:
// 删成功而改名再失败时, 新旧内容一起没了。Rename 失败时保住旧文件完整才是对的。
//
// 旁证: shieldPersistState 一直用裸 os.Rename 覆盖落盘, 从未因此出问题。
func writeStateFileAtomic(path string, data []byte) error {
	dir := filepath.Dir(path)
	tmp, err := os.CreateTemp(dir, ".toolrouter-state-*.tmp")
	if err != nil {
		return err
	}
	tmpName := tmp.Name()
	defer os.Remove(tmpName)
	if err := tmp.Chmod(0o644); err != nil {
		_ = tmp.Close()
		return err
	}
	if _, err := tmp.Write(data); err != nil {
		_ = tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	return os.Rename(tmpName, path)
}

// 工具路由: 按任务类型决定这次请求把哪些 MCP 工具摆到模型面前, 以及用哪个模型。
//
// 边界(必读):
// proxy 是 HTTP 中继, 它 **不执行** 工具。真正的工具在客户端本地跑, proxy 眼里的
// tools 只是一段 JSON 声明。所以这里能做的是"少摆出来", 不能"凭空变出来"。
// 给请求塞进客户端没注册的工具会让模型回调一个不存在的工具名, 客户端直接失败。
//
// 因此本文件只有两个动作:
//  1. 从客户端已声明的工具里 **收窄**(移除部分 MCP 命名空间);
//  2. 按任务类型覆盖 model。
// 绝不新增工具。

// ---------- 任务类型 ----------

// 任务类型与仓库既有的 lskill 五分类对齐(见 src-tauri/resources/devconduit-skills/
// lskill-1.5.9/)。同一句话在两处必须落到同一类, 否则提示词路由与工具路由会互相矛盾。
//
// 对齐关系:
//
//	toolrouter      lskill          说明
//	reverse         l-reverse       通用逆向 / 未点名工具时的逆向
//	gameassist      l-gameassist    游戏内存 / 外挂 / 反作弊
//	webrecon        l-webrecon      Web/API 渗透
//	license         l-license       破解 / 激活 / keygen
//	(default)       l-dev           内部服务维护, 不参与工具路由(见下)
//
// l-dev 刻意不设为任务类型: 它是破甲助手自身的维护路线("disabled for ordinary
// client delivery"), 不该由用户消息触发, 也就不该影响工具裁剪。
const (
	TaskClassDefault    = "default"
	TaskClassReverse    = "reverse"
	TaskClassGameAssist = "gameassist"
	TaskClassWebRecon   = "webrecon"
	TaskClassLicense    = "license"
	TaskClassCrawler    = "crawler" // 本地无对应 lskill, 属通用能力
)

// 原生工具类型。这些一律不碰:
// 它们是客户端必备的执行能力(shell / apply_patch / 联网 / 生图),
// 裁掉会让模型连基本命令都跑不了。
var toolRouterNativeTypes = map[string]bool{
	"function":         true,
	"custom":           true,
	"web_search":       true,
	"image_generation": true,
	"tool_search":      true,
	"computer_use":     true,
	"code_interpreter": true,
}

// toolRouterNamespacePrefix 是 MCP 工具在 Responses 里的命名前缀。
const toolRouterNamespacePrefix = "mcp__"

// ---------- 配置 ----------

// 工具档案: 某类任务允许摆出哪些 MCP 命名空间, 以及要不要换模型。
type toolRouteProfile struct {
	ID    string `json:"id"`
	Label string `json:"label"`
	// AllowNamespaces 为空表示 **全部放行**(保守默认)。
	// 非空表示白名单: 命名空间名里含任一 token 才保留。
	AllowNamespaces []string `json:"allowNamespaces"`
	// DenyNamespaces 优先于白名单, 命中即移除。
	DenyNamespaces []string `json:"denyNamespaces"`
	// Model 为空表示不改模型。
	Model string `json:"model"`
}

type toolRouteRule struct {
	Class   string `json:"class"`
	Pattern string `json:"pattern"`
	Weight  int    `json:"weight"`

	re *regexp.Regexp
}

type toolRouterConfig struct {
	Enabled bool `json:"enabled"`
	// ModelRouting 独立开关: 裁剪和换模型可以分开用。
	ModelRouting bool `json:"modelRouting"`
	// DefaultProfile 是分类不出结果时用的档案 ID。
	// 默认档案必须保守(不裁剪), 否则分类器一失效就误裁。
	DefaultProfile string             `json:"defaultProfile"`
	Profiles       []toolRouteProfile `json:"profiles"`
	Rules          []toolRouteRule    `json:"rules"`
}

// ---------- 运行时状态 ----------

var (
	toolRouterCfg      atomic.Pointer[toolRouterConfig]
	toolRouterCfgMu    sync.Mutex
	toolRouterPath     string
	toolRouterPathMu   sync.Mutex
	toolRouterLastLoad string
)

type toolRouterStats struct {
	Classified map[string]int64 `json:"classified"`
	Trimmed    int64            `json:"trimmed"`
	Skipped    int64            `json:"skipped"` // 无需裁剪(档案全放行)
	Routed     int64            `json:"routed"`  // 换过模型的次数
	Failed     int64            `json:"failed"`
}

var (
	toolRouterStatsMu sync.Mutex
	toolRouterClass   = map[string]int64{}
	toolRouterTrimmed int64
	toolRouterSkipped int64
	toolRouterRouted  int64
	toolRouterFailed  int64
)

func recordToolRouterClass(class string) {
	toolRouterStatsMu.Lock()
	toolRouterClass[class]++
	toolRouterStatsMu.Unlock()
}

func recordToolRouterTrimmed() {
	toolRouterStatsMu.Lock()
	toolRouterTrimmed++
	toolRouterStatsMu.Unlock()
}

func recordToolRouterSkipped() {
	toolRouterStatsMu.Lock()
	toolRouterSkipped++
	toolRouterStatsMu.Unlock()
}

func recordToolRouterRouted() {
	toolRouterStatsMu.Lock()
	toolRouterRouted++
	toolRouterStatsMu.Unlock()
}

func recordToolRouterFailed() {
	toolRouterStatsMu.Lock()
	toolRouterFailed++
	toolRouterStatsMu.Unlock()
}

func toolRouterSnapshot() toolRouterStats {
	toolRouterStatsMu.Lock()
	defer toolRouterStatsMu.Unlock()
	attempts := make(map[string]int64, len(toolRouterClass))
	for k, v := range toolRouterClass {
		attempts[k] = v
	}
	return toolRouterStats{
		Classified: attempts,
		Trimmed:    toolRouterTrimmed,
		Skipped:    toolRouterSkipped,
		Routed:     toolRouterRouted,
		Failed:     toolRouterFailed,
	}
}

// ---------- 默认配置 ----------

// defaultToolRouterConfig 内置一组种子规则与档案。
//
// 关键取舍: default 档案 **不做任何裁剪**。你选择"识别不出走默认档案",
// 那这个档案就必须是最保守的形态, 否则分类器一旦判错就会误裁正常请求。
// 它是可编辑的槽位, 但出厂值是 no-op。
func defaultToolRouterConfig() *toolRouterConfig {
	cfg := &toolRouterConfig{
		Enabled:        true,
		ModelRouting:   false, // 换模型影响面大, 默认关, 由你在面板上开
		DefaultProfile: TaskClassDefault,
		Profiles: []toolRouteProfile{
			{
				ID:    TaskClassDefault,
				Label: "默认(不裁剪)",
				// 空 = 全部放行。这是保守兜底, 不要改成白名单。
				AllowNamespaces: nil,
			},
			{
				// 通用逆向: 静态分析 / 反编译 / 反汇编 / 调试器。与 l-reverse 对齐。
				ID:    TaskClassReverse,
				Label: "二进制逆向",
				AllowNamespaces: []string{
					"ida-pro", "ida-headless", "ida", "ghidra", "radare", "r2mcp",
					"binary_ninja", "binaryninja", "binja", "lldb", "solvitor",
					"apktool", "jadx", "objdump", "decompile",
				},
				DenyNamespaces: []string{"crawl", "scrap", "playwright"},
			},
			{
				// 游戏辅助: 内存读写 / 调试器 / 模拟器 / 引擎侧。与 l-gameassist 对齐。
				ID:    TaskClassGameAssist,
				Label: "游戏辅助",
				AllowNamespaces: []string{
					"cheatengine", "cheat-engine", "cheat_engine", "x64dbg", "x32dbg",
					"frida", "il2cpp", "ue5", "unreal", "unity",
					"pine", "bizhawk", "dolphin", "mgba", "pcsx2", "emulator",
					"ida", "ghidra",
				},
				DenyNamespaces: []string{"crawl", "scrap"},
			},
			{
				// Web 渗透: 代理拦截 / 扫描器 / 抓包 / 代码审计。与 l-webrecon 对齐。
				ID:    TaskClassWebRecon,
				Label: "Web 渗透",
				AllowNamespaces: []string{
					"burp", "zap", "nuclei", "sqlmap", "nmap", "masscan",
					"secops", "sast", "dast", "wireshark", "cybersec",
					"httpx", "ffuf",
				},
				DenyNamespaces: []string{"crawl", "scrap"},
			},
			{
				// 授权/激活分析: 工具集与逆向高度重合(静态分析 + 调试器),
				// 差异在提示词侧(skill 不同), 不在工具侧。如实照此配置。
				ID:    TaskClassLicense,
				Label: "授权破解",
				AllowNamespaces: []string{
					"ida", "ghidra", "radare", "r2mcp", "binary_ninja", "binja",
					"x64dbg", "x32dbg", "cheatengine", "lldb", "frida",
					"apktool", "jadx", "decompile",
				},
				DenyNamespaces: []string{"crawl", "scrap"},
			},
			{
				// 爬虫采集。本地 lskill 五分类里无对应项, 属通用能力;
				// 保留是因为你的原始需求明确提到"爬虫就走爬虫"。
				ID:    TaskClassCrawler,
				Label: "爬虫采集",
				AllowNamespaces: []string{
					"crawl", "scrap", "spider", "fetch", "webreaper", "pyrecrawl",
					"browser", "playwright", "puppeteer", "selenium", "browserless",
					"browser-use", "firecrawl", "headless", "nodriver",
				},
				DenyNamespaces: []string{"ida", "ghidra", "x64dbg", "cheatengine", "burp"},
			},
		},
		Rules: []toolRouteRule{
			// ── 二进制逆向 (l-reverse) ──
			{Class: TaskClassReverse, Weight: 10, Pattern: `逆向|反编译|脱壳|反汇编|逆向工程|动态调试|静态分析`},
			{Class: TaskClassReverse, Weight: 9, Pattern: `(?i)\bIDA\b|\bGhidra\b|\bradare2?\b|\brizin\b|\bBinary ?Ninja\b|\blldb\b|\bobjdump\b`},
			{Class: TaskClassReverse, Weight: 9, Pattern: `(?i)\bFrida\b|微信小程序|小程序逆向`},
			{Class: TaskClassReverse, Weight: 8, Pattern: `(?i)\bapk\b|\bjadx\b|apktool|smali|\.dex\b`},
			{Class: TaskClassReverse, Weight: 8, Pattern: `(?i)\bpe\b|\belf\b|\bdll\b|\.so\b|加壳|查壳|节表|导入表`},
			{Class: TaskClassReverse, Weight: 7, Pattern: `协议逆向|抓包分析|封包结构|通信协议分析`},

			// ── 游戏辅助 (l-gameassist) ──
			{Class: TaskClassGameAssist, Weight: 10, Pattern: `外挂|自瞄|透视|ESP|游戏辅助|游戏脚本|游戏内存|修改器`},
			{Class: TaskClassGameAssist, Weight: 10, Pattern: `内存修改|内存读写|内存搜索|读写内存|指针扫描|特征码|基址|偏移`},
			{Class: TaskClassGameAssist, Weight: 9, Pattern: `反作弊|反外挂|隐藏进程|驱动隐藏|过检测`},
			{Class: TaskClassGameAssist, Weight: 9, Pattern: `封包|改包|发包|服务器绕过|GM指令|GM面板|游戏协议`},
			{Class: TaskClassGameAssist, Weight: 8, Pattern: `(?i)\bil2cpp\b|\bue5\b|\bUE4\b|\bunreal\b`},
			{Class: TaskClassGameAssist, Weight: 8, Pattern: `(?i)\bCheat ?Engine\b|\bx64dbg\b|\bx32dbg\b`},
			{Class: TaskClassGameAssist, Weight: 7, Pattern: `自动挖矿|连招|打怪|自动战斗|世界坐标|游戏状态读取`},

			// ── 授权破解 (l-license) ──
			{Class: TaskClassLicense, Weight: 11, Pattern: `破解|绕过激活|去激活|去验证|移除授权|授权校验`},
			{Class: TaskClassLicense, Weight: 10, Pattern: `(?i)keygen|注册机|卡密|序列号|激活码|授权码`},
			{Class: TaskClassLicense, Weight: 8, Pattern: `(?i)\bpatch\b|验证点|校验点|试用期|\btrial\b`},

			// ── Web 渗透 (l-webrecon) ──
			{Class: TaskClassWebRecon, Weight: 10, Pattern: `渗透|打点|漏洞扫描|漏扫|渗透测试|测安全`},
			{Class: TaskClassWebRecon, Weight: 9, Pattern: `(?i)\bsql ?注入|\bxss\b|\bssrf\b|\bcsrf\b|命令注入|反序列化`},
			{Class: TaskClassWebRecon, Weight: 9, Pattern: `越权|\bIDOR\b|权限提升|提权|认证绕过|加管理员|拿管理员`},
			{Class: TaskClassWebRecon, Weight: 8, Pattern: `(?i)\bwaf\b|验证码绕过|\bjwt\b|凭证提取|撞库|未授权访问`},
			{Class: TaskClassWebRecon, Weight: 8, Pattern: `子域名|目录爆破|指纹识别|接口探测|端口扫描`},
			{Class: TaskClassWebRecon, Weight: 8, Pattern: `(?i)getshell|webshell|拿shell|反弹shell`},

			// ── 爬虫采集 ──
			{Class: TaskClassCrawler, Weight: 10, Pattern: `爬虫|爬取|采集|抓取|批量下载|数据抓取`},
			{Class: TaskClassCrawler, Weight: 9, Pattern: `(?i)\bscrapy\b|\bselenium\b|\bplaywright\b|\bpuppeteer\b|无头浏览器`},
			{Class: TaskClassCrawler, Weight: 8, Pattern: `反爬|验证码识别|代理池|限速抓取|增量抓取`},
		},
	}
	cfg.compileRules()
	return cfg
}

// compileRules 预编译正则。编译失败的单条规则会被跳过并告警, 而不是让整个配置失效。
func (c *toolRouterConfig) compileRules() {
	compiled := make([]toolRouteRule, 0, len(c.Rules))
	for _, rule := range c.Rules {
		pattern := strings.TrimSpace(rule.Pattern)
		if pattern == "" {
			continue
		}
		re, err := regexp.Compile(pattern)
		if err != nil {
			log.Printf("[toolrouter] rule pattern skipped (bad regex): %q: %v", pattern, err)
			continue
		}
		rule.re = re
		if rule.Weight <= 0 {
			rule.Weight = 1
		}
		compiled = append(compiled, rule)
	}
	c.Rules = compiled
}

func setToolRouterConfig(cfg *toolRouterConfig) {
	if cfg == nil {
		cfg = defaultToolRouterConfig()
	}
	cfg.compileRules()
	toolRouterCfgMu.Lock()
	toolRouterCfg.Store(cfg)
	toolRouterCfgMu.Unlock()
}

func currentToolRouterConfig() *toolRouterConfig {
	cfg := toolRouterCfg.Load()
	if cfg == nil {
		return defaultToolRouterConfig()
	}
	return cfg
}

func toolRouterEnabled() bool {
	return currentToolRouterConfig().Enabled
}

func init() {
	toolRouterCfg.Store(defaultToolRouterConfig())
}

// profileByID 按 ID 找档案。
func (c *toolRouterConfig) profileByID(id string) *toolRouteProfile {
	for i := range c.Profiles {
		if c.Profiles[i].ID == id {
			return &c.Profiles[i]
		}
	}
	return nil
}

// defaultProfile 取兜底档案。找不到就返回 nil(调用方据此跳过处理)。
func (c *toolRouterConfig) defaultProfile() *toolRouteProfile {
	if profile := c.profileByID(c.DefaultProfile); profile != nil {
		return profile
	}
	return c.profileByID(TaskClassDefault)
}

// ---------- 分类 ----------

// toolRouterClassify 用规则表打分, 返回得分最高的任务类型。
//
// 平局时按 reverse > crawler > web 的固定顺序取, 保证同样输入永远同样结果。
// 无任何命中时返回 default, 由调用方换上兜底档案。
func toolRouterClassify(cfg *toolRouterConfig, text string) (string, int, string) {
	if strings.TrimSpace(text) == "" {
		return TaskClassDefault, 0, "empty_input"
	}
	scores := map[string]int{}
	reasons := map[string]string{}
	for _, rule := range cfg.Rules {
		if rule.re == nil {
			continue
		}
		if match := rule.re.FindString(text); match != "" {
			scores[rule.Class] += rule.Weight
			if _, ok := reasons[rule.Class]; !ok {
				reasons[rule.Class] = match
			}
		}
	}
	if len(scores) == 0 {
		return TaskClassDefault, 0, "no_rule_matched"
	}

	best := TaskClassDefault
	bestScore := 0
	for class, score := range scores {
		if score > bestScore {
			best, bestScore = class, score
			continue
		}
		// 平局: 按固定优先级, 不依赖 map 遍历顺序。
		if score == bestScore && toolRouterClassRank(class) < toolRouterClassRank(best) {
			best = class
		}
	}
	return best, bestScore, reasons[best]
}

// toolRouterClassRank 数值越小优先级越高。仅在分数打平时用于决定归属。
//
// 排列依据: 关键词越具体, 越不可能误判, 平局时优先采信。
// license(keygen/注册机) 与 gameassist(外挂/自瞄) 的词几乎不会在普通对话里出现,
// 而 reverse 的"逆向"、webrecon 的"渗透"、crawler 的"采集"都更宽泛。
// 固定顺序也保证了同样输入永远得到同样结果, 不依赖 map 遍历顺序。
func toolRouterClassRank(class string) int {
	switch class {
	case TaskClassLicense:
		return 0
	case TaskClassGameAssist:
		return 1
	case TaskClassReverse:
		return 2
	case TaskClassWebRecon:
		return 3
	case TaskClassCrawler:
		return 4
	default:
		return 99
	}
}

// ---------- 请求正文取文本 ----------

// toolRouterRequestText 抽出用于分类的文本: 用户消息 + instructions。
// 只取用户侧内容, 不取助手历史, 避免上一轮的残留把本轮分类带偏。
func toolRouterRequestText(body []byte) string {
	var req map[string]json.RawMessage
	if err := json.Unmarshal(body, &req); err != nil {
		return ""
	}
	var b strings.Builder

	if raw, ok := req["instructions"]; ok {
		var s string
		if json.Unmarshal(raw, &s) == nil {
			b.WriteString(s)
			b.WriteString("\n")
		}
	}

	raw, ok := req["input"]
	if !ok {
		return b.String()
	}
	var s string
	if json.Unmarshal(raw, &s) == nil {
		b.WriteString(s)
		return b.String()
	}

	var items []map[string]json.RawMessage
	if json.Unmarshal(raw, &items) != nil {
		return b.String()
	}
	// 只保留最后一条 user 消息附近的上下文: 长会话里早期内容不代表当前意图。
	start := 0
	if len(items) > 6 {
		start = len(items) - 6
	}
	for _, item := range items[start:] {
		var role string
		if json.Unmarshal(item["role"], &role) != nil || role != "user" {
			continue
		}
		b.WriteString(toolRouterItemText(item["content"]))
		b.WriteString("\n")
	}
	return b.String()
}

// toolRouterItemText 取一条 content 的文本, 兼容字符串与分段两种形态。
func toolRouterItemText(raw json.RawMessage) string {
	if len(raw) == 0 {
		return ""
	}
	var s string
	if json.Unmarshal(raw, &s) == nil {
		return s
	}
	var segs []map[string]json.RawMessage
	if json.Unmarshal(raw, &segs) != nil {
		return ""
	}
	var b strings.Builder
	for _, seg := range segs {
		var text string
		if json.Unmarshal(seg["text"], &text) == nil {
			b.WriteString(text)
			b.WriteString(" ")
		}
	}
	return b.String()
}

// ---------- 工具裁剪 ----------

// toolRouterNamespaceAllowed 判定某个 MCP 命名空间是否保留。
//
// token 匹配用大小写不敏感子串: 命名空间名形如 mcp__codex_apps__gmail,
// token 写 "gmail" 即可命中。空白名单 = 全放行。
func toolRouterNamespaceAllowed(profile *toolRouteProfile, name string) bool {
	if profile == nil {
		return true
	}
	lower := strings.ToLower(name)
	for _, token := range profile.DenyNamespaces {
		token = strings.ToLower(strings.TrimSpace(token))
		if token != "" && strings.Contains(lower, token) {
			return false
		}
	}
	if len(profile.AllowNamespaces) == 0 {
		return true
	}
	for _, token := range profile.AllowNamespaces {
		token = strings.ToLower(strings.TrimSpace(token))
		if token != "" && strings.Contains(lower, token) {
			return true
		}
	}
	return false
}

// toolRouterTrimToolsArray 裁一个 tools 数组。
//
// 只移除 type=namespace 且 name 带 mcp__ 前缀的条目; 其余原样保留。
// 若裁剪后数组为空但原本非空, 则放弃裁剪 —— 宁可少裁也不要裁出
// "模型没有任何工具可用"的状态。
func toolRouterTrimToolsArray(raw json.RawMessage, profile *toolRouteProfile) (json.RawMessage, int, bool) {
	var entries []json.RawMessage
	if json.Unmarshal(raw, &entries) != nil {
		return raw, 0, false
	}
	if len(entries) == 0 {
		return raw, 0, false
	}

	kept := make([]json.RawMessage, 0, len(entries))
	removed := 0
	for _, entry := range entries {
		var meta struct {
			Type string `json:"type"`
			Name string `json:"name"`
		}
		if json.Unmarshal(entry, &meta) != nil {
			kept = append(kept, entry)
			continue
		}
		// 原生工具永不裁剪。
		if !strings.EqualFold(meta.Type, "namespace") ||
			!strings.HasPrefix(meta.Name, toolRouterNamespacePrefix) {
			kept = append(kept, entry)
			continue
		}
		if toolRouterNamespaceAllowed(profile, meta.Name) {
			kept = append(kept, entry)
			continue
		}
		removed++
	}

	if removed == 0 {
		return raw, 0, false
	}
	if len(kept) == 0 {
		// 全被裁光: 放弃, 保住请求可用性。
		return raw, 0, false
	}
	encoded, err := json.Marshal(kept)
	if err != nil {
		return raw, 0, false
	}
	return encoded, removed, true
}

// toolRouterTrimBody 裁剪请求体里的所有 tools 数组(顶层与 input[].tools)。
// 用 RawMessage 逐字段处理, 保证未触及的字段字节不变。
func toolRouterTrimBody(body []byte, profile *toolRouteProfile) ([]byte, int, bool) {
	var req map[string]json.RawMessage
	if err := json.Unmarshal(body, &req); err != nil {
		return body, 0, false
	}

	totalRemoved := 0
	changed := false

	if raw, ok := req["tools"]; ok {
		if next, removed, ok2 := toolRouterTrimToolsArray(raw, profile); ok2 {
			req["tools"] = next
			totalRemoved += removed
			changed = true
		}
	}

	// input 里可能嵌 additional_tools。
	if raw, ok := req["input"]; ok {
		var items []map[string]json.RawMessage
		if json.Unmarshal(raw, &items) == nil {
			itemChanged := false
			for _, item := range items {
				nested, ok := item["tools"]
				if !ok {
					continue
				}
				next, removed, ok2 := toolRouterTrimToolsArray(nested, profile)
				if !ok2 {
					continue
				}
				item["tools"] = next
				totalRemoved += removed
				itemChanged = true
			}
			if itemChanged {
				if encoded, err := json.Marshal(items); err == nil {
					req["input"] = encoded
					changed = true
				}
			}
		}
	}

	if !changed {
		return body, 0, false
	}
	out, err := json.Marshal(req)
	if err != nil {
		return body, 0, false
	}
	return out, totalRemoved, true
}

// ---------- 决策入口 ----------

// toolRouteDecision 描述本次请求的路由结果, 用于日志与面板展示。
type toolRouteDecision struct {
	Class         string `json:"class"`
	ProfileID     string `json:"profileId"`
	Score         int    `json:"score"`
	Reason        string `json:"reason"`
	RemovedTools  int    `json:"removedTools"`
	ModelOverride string `json:"modelOverride"`
}

// toolRouterDecide 只做分类决策, 不碰 body。
//
// 必须在请求早期(shield/ladder 改写之前)调用: ladder 的 L2 会替换用户文本里的
// 术语, 事后分类的准确度会下降。而裁剪本身要等到最后做, 因为历史清理会重建
// input 树, 提前裁进去的 tools 字段有被重编码的风险。
func toolRouterDecide(body []byte) *toolRouteDecision {
	cfg := currentToolRouterConfig()
	if !cfg.Enabled {
		return nil
	}

	class, score, reason := toolRouterClassify(cfg, toolRouterRequestText(body))
	profile := cfg.profileByID(class)
	resolved := class
	if profile == nil {
		profile = cfg.defaultProfile()
		if profile == nil {
			return nil
		}
		resolved = profile.ID
		reason = "profile_fallback:" + reason
	}
	return &toolRouteDecision{
		Class:     class,
		ProfileID: resolved,
		Score:     score,
		Reason:    reason,
	}
}

// toolRouterApplyDecision 按已有决策裁剪工具并(可选)改模型。
// decision 为 nil 或路由关闭时原样返回。
func (s *relayServer) toolRouterApplyDecision(
	spec *apiKeySpec,
	body []byte,
	model string,
	decision *toolRouteDecision,
) ([]byte, string) {
	cfg := currentToolRouterConfig()
	if decision == nil || !cfg.Enabled {
		return body, model
	}
	profile := cfg.profileByID(decision.ProfileID)
	if profile == nil {
		return body, model
	}

	recordToolRouterClass(decision.Class)

	out := body
	if next, removed, ok := toolRouterTrimBody(body, profile); ok {
		out = next
		decision.RemovedTools = removed
		recordToolRouterTrimmed()
		log.Printf(
			"[toolrouter] class=%s profile=%s removed=%d namespaces (reason=%s)",
			decision.Class, decision.ProfileID, removed, decision.Reason,
		)
	} else {
		recordToolRouterSkipped()
	}

	// 模型路由: 独立开关, 且必须验证目标模型对当前 Key 可见。
	//
	// model 变量的去向: 执行器用 req.Model 覆盖 payload 里的 model
	// (codex_executor.go 里 sjson.SetBytes(body, "model", baseModel)),
	// 所以真正决定上游模型的是这里返回的 model, 由调用方传给 buildExecutorRequest。
	if cfg.ModelRouting && strings.TrimSpace(profile.Model) != "" &&
		!strings.EqualFold(strings.TrimSpace(profile.Model), model) {
		if target, ok := s.resolveRoutedModel(spec, profile.Model); ok {
			decision.ModelOverride = target
			model = target
			// payload 里的 model 一并同步: 执行器会覆盖它, 但两处不一致时
			// 中间环节(日志/计费/翻译)读到的模型会互相矛盾。
			if next, ok2 := toolRouterSetBodyModel(out, target); ok2 {
				out = next
			}
			recordToolRouterRouted()
			log.Printf("[toolrouter] model routed to %s for class=%s", target, decision.Class)
		} else {
			recordToolRouterFailed()
			log.Printf("[toolrouter] model %q not visible for this key, skipped", profile.Model)
		}
	}

	return out, model
}

// toolRouterApply 是 decide+apply 的组合入口, 供测试与单段调用使用。
func (s *relayServer) toolRouterApply(
	spec *apiKeySpec,
	body []byte,
	model string,
) ([]byte, string, *toolRouteDecision) {
	decision := toolRouterDecide(body)
	if decision == nil {
		return body, model, nil
	}
	out, nextModel := s.toolRouterApplyDecision(spec, body, model, decision)
	return out, nextModel, decision
}

// toolRouterSetBodyModel 覆盖请求体里的 model 字段。字段不存在则不动。
func toolRouterSetBodyModel(body []byte, model string) ([]byte, bool) {
	var req map[string]json.RawMessage
	if json.Unmarshal(body, &req) != nil {
		return body, false
	}
	if _, ok := req["model"]; !ok {
		return body, false
	}
	encoded, err := json.Marshal(model)
	if err != nil {
		return body, false
	}
	req["model"] = encoded
	out, err := json.Marshal(req)
	if err != nil {
		return body, false
	}
	return out, true
}

// resolveRoutedModel 把档案里的模型名解析成上游可用名, 并确认对当前 Key 可见。
// 解析不出来就返回 false —— 宁可不动, 也不要换成一个上游不认的模型。
func (s *relayServer) resolveRoutedModel(spec *apiKeySpec, target string) (string, bool) {
	target = strings.TrimSpace(target)
	if target == "" || s == nil || s.manifest == nil {
		return "", false
	}
	canonical := canonicalModelForClientModel(s.manifest, spec, target)
	if strings.TrimSpace(canonical) == "" {
		return "", false
	}
	if !validateClientModelVisible(s.manifest, spec, target, canonical) {
		return "", false
	}
	return canonical, true
}

// ---------- 状态持久化 ----------

type toolRouterState struct {
	Enabled        bool               `json:"enabled"`
	ModelRouting   bool               `json:"modelRouting"`
	DefaultProfile string             `json:"defaultProfile"`
	Profiles       []toolRouteProfile `json:"profiles"`
	Rules          []toolRouteRule    `json:"rules"`
}

func toolRouterInitStatePath(manifestPath string) {
	toolRouterPathMu.Lock()
	defer toolRouterPathMu.Unlock()
	toolRouterPath = filepath.Join(filepath.Dir(manifestPath), "toolrouter-state.json")
}

func currentToolRouterPath() string {
	toolRouterPathMu.Lock()
	defer toolRouterPathMu.Unlock()
	return toolRouterPath
}

// toolRouterLoadState 从文件读配置。文件不存在或解析失败都保持当前值 ——
// 配置读坏不该让路由停摆。
func toolRouterLoadState() {
	path := currentToolRouterPath()
	if path == "" {
		return
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return
	}
	content := strings.TrimSpace(string(data))
	if content == "" {
		return
	}

	toolRouterCfgMu.Lock()
	if content == toolRouterLastLoad {
		toolRouterCfgMu.Unlock()
		return
	}
	var rawState map[string]json.RawMessage
	if err := json.Unmarshal([]byte(content), &rawState); err != nil {
		toolRouterCfgMu.Unlock()
		log.Printf("[toolrouter] state presence parse failed, keeping current: %v", err)
		return
	}
	var state toolRouterState
	if err := json.Unmarshal([]byte(content), &state); err != nil {
		toolRouterCfgMu.Unlock()
		log.Printf("[toolrouter] state parse failed, keeping current: %v", err)
		return
	}
	toolRouterLastLoad = content

	// 缺字段时继承当前值，避免旧格式文件因 bool 零值意外关闭路由。
	defaults := defaultToolRouterConfig()
	current := currentToolRouterConfig()
	enabled := state.Enabled
	if _, ok := rawState["enabled"]; !ok {
		enabled = current.Enabled
	}
	modelRouting := state.ModelRouting
	if _, ok := rawState["modelRouting"]; !ok {
		modelRouting = current.ModelRouting
	}
	cfg := &toolRouterConfig{
		Enabled:        enabled,
		ModelRouting:   modelRouting,
		DefaultProfile: state.DefaultProfile,
		Profiles:       state.Profiles,
		Rules:          state.Rules,
	}
	if len(cfg.Profiles) == 0 {
		cfg.Profiles = defaults.Profiles
	}
	if len(cfg.Rules) == 0 {
		cfg.Rules = defaults.Rules
	}
	if strings.TrimSpace(cfg.DefaultProfile) == "" {
		cfg.DefaultProfile = defaults.DefaultProfile
	}
	cfg.compileRules()
	toolRouterCfg.Store(cfg)
	toolRouterCfgMu.Unlock()

	log.Printf(
		"[toolrouter] state loaded: enabled=%v modelRouting=%v profiles=%d rules=%d",
		cfg.Enabled, cfg.ModelRouting, len(cfg.Profiles), len(cfg.Rules),
	)
}

// toolRouterPersistState 把当前配置写回文件; 未初始化时静默跳过。
func toolRouterPersistState() error {
	path := currentToolRouterPath()
	if path == "" {
		return fmt.Errorf("tool router state path not initialized")
	}
	cfg := currentToolRouterConfig()
	state := toolRouterState{
		Enabled:        cfg.Enabled,
		ModelRouting:   cfg.ModelRouting,
		DefaultProfile: cfg.DefaultProfile,
		Profiles:       cfg.Profiles,
		Rules:          cfg.Rules,
	}
	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	return writeStateFileAtomic(path, data)
}

func toolRouterHotReloadLoop() {
	go func() {
		ticker := time.NewTicker(3 * time.Second)
		defer ticker.Stop()
		for range ticker.C {
			toolRouterLoadState()
		}
	}()
}

// ---------- 管理端点 ----------

// handleToolRouterAdmin 是 /v1/toolrouter 系列端点, 仅本机可访问。
func handleToolRouterAdmin(c *gin.Context) {
	if !isLoopbackRequest(c.Request) {
		writeAPIError(c, http.StatusForbidden, "tool router admin is loopback-only", "forbidden")
		return
	}

	updated := false
	if c.Request.Method == http.MethodPost {
		var incoming toolRouterState
		if err := c.ShouldBindJSON(&incoming); err == nil && len(incoming.Profiles) > 0 {
			defaults := defaultToolRouterConfig()
			cfg := &toolRouterConfig{
				Enabled:        incoming.Enabled,
				ModelRouting:   incoming.ModelRouting,
				DefaultProfile: incoming.DefaultProfile,
				Profiles:       incoming.Profiles,
				Rules:          incoming.Rules,
			}
			if len(cfg.Rules) == 0 {
				cfg.Rules = defaults.Rules
			}
			if strings.TrimSpace(cfg.DefaultProfile) == "" {
				cfg.DefaultProfile = defaults.DefaultProfile
			}
			setToolRouterConfig(cfg)
			updated = true
		} else if strings.HasSuffix(c.Request.URL.Path, "/on") {
			cfg := *currentToolRouterConfig()
			cfg.Enabled = true
			setToolRouterConfig(&cfg)
			updated = true
		} else if strings.HasSuffix(c.Request.URL.Path, "/off") {
			cfg := *currentToolRouterConfig()
			cfg.Enabled = false
			setToolRouterConfig(&cfg)
			updated = true
		}
	}

	if updated {
		if err := toolRouterPersistState(); err != nil &&
			!strings.Contains(err.Error(), "not initialized") {
			log.Printf("[toolrouter] persist failed: %v", err)
		}
	}

	cfg := currentToolRouterConfig()
	profiles := make([]gin.H, 0, len(cfg.Profiles))
	for _, profile := range cfg.Profiles {
		profiles = append(profiles, gin.H{
			"id":              profile.ID,
			"label":           profile.Label,
			"allowNamespaces": profile.AllowNamespaces,
			"denyNamespaces":  profile.DenyNamespaces,
			"model":           profile.Model,
		})
	}
	rules := make([]gin.H, 0, len(cfg.Rules))
	for _, rule := range cfg.Rules {
		rules = append(rules, gin.H{
			"class":   rule.Class,
			"pattern": rule.Pattern,
			"weight":  rule.Weight,
		})
	}

	c.JSON(http.StatusOK, gin.H{
		"enabled":        cfg.Enabled,
		"modelRouting":   cfg.ModelRouting,
		"defaultProfile": cfg.DefaultProfile,
		"profiles":       profiles,
		"rules":          rules,
		"stats":          toolRouterSnapshot(),
	})
}

// toolRouterDescribeClasses 供面板展示: 有哪些任务类型及其标签。
func toolRouterDescribeClasses() []gin.H {
	cfg := currentToolRouterConfig()
	out := make([]gin.H, 0, len(cfg.Profiles))
	for _, profile := range cfg.Profiles {
		out = append(out, gin.H{
			"id":    profile.ID,
			"label": profile.Label,
			"model": profile.Model,
		})
	}
	sort.SliceStable(out, func(i, j int) bool {
		left, _ := out[i]["id"].(string)
		right, _ := out[j]["id"].(string)
		return toolRouterClassRank(left) < toolRouterClassRank(right)
	})
	return out
}
