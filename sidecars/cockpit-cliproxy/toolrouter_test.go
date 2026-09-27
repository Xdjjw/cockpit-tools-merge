package main

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
)

// writeFileForTest 写一个测试用状态文件。
func writeFileForTest(path string, content string) error {
	return os.WriteFile(path, []byte(content), 0o644)
}

// 本文件锁住工具路由最危险的行为: 误裁。
//
// 路由的失败模式不是"没生效", 而是"把模型干活必需的工具裁掉了",
// 那会让一个本来正常的请求直接残废。所以负向断言比正向断言更重要。

// buildToolsBody 造一个带 tools 的 Responses 请求体。
func buildToolsBody(userText string, tools []map[string]interface{}) []byte {
	body := map[string]interface{}{
		"model": "gpt-5.6-sol",
		"input": []map[string]interface{}{
			{"role": "user", "content": userText},
		},
	}
	if tools != nil {
		body["tools"] = tools
	}
	encoded, _ := json.Marshal(body)
	return encoded
}

// nativeTools 是客户端必备的原生工具, 任何情况下都不该被裁掉。
func nativeTools() []map[string]interface{} {
	return []map[string]interface{}{
		{"type": "function", "name": "shell"},
		{"type": "custom", "name": "apply_patch"},
		{"type": "web_search"},
		{"type": "tool_search"},
	}
}

func toolNames(t *testing.T, body []byte) []string {
	t.Helper()
	var req struct {
		Tools []map[string]interface{} `json:"tools"`
	}
	if err := json.Unmarshal(body, &req); err != nil {
		t.Fatal(err)
	}
	names := make([]string, 0, len(req.Tools))
	for _, tool := range req.Tools {
		name, _ := tool["name"].(string)
		if name == "" {
			name, _ = tool["type"].(string)
		}
		names = append(names, name)
	}
	return names
}

// ---------- 分类 ----------

// 分类必须与仓库既有的 lskill 五分类对齐, 否则同一句话在提示词路由与工具路由
// 会落到不同类。这里的用例逐类对照。
func TestToolRouterClassifyBinaryReverse(t *testing.T) {
	cfg := defaultToolRouterConfig()
	cases := []string{
		"帮我把这个 exe 反编译一下，看看它的注册校验逻辑",
		"用 IDA 分析这个二进制，找出关键跳转",
		"这个 apk 用 jadx 反编译看看",
		"分析一下这个 dll，看看导入表",
		"这个 elf 用了什么壳，帮我查壳",
	}
	for _, text := range cases {
		class, score, _ := toolRouterClassify(cfg, text)
		if class != TaskClassReverse {
			t.Errorf("分类错误 %q -> %s (score=%d), want=%s", text, class, score, TaskClassReverse)
		}
	}
}

// 游戏辅助类: 对应 l-gameassist。这些词不该再被 reverse 抢走。
func TestToolRouterClassifyGameAssist(t *testing.T) {
	cfg := defaultToolRouterConfig()
	cases := []string{
		"目标进程用 il2cpp，想做个自瞄",
		"帮我做一个内存修改器，搜索血量基址",
		"这个游戏有反作弊，怎么过检测",
		"帮我分析一下游戏封包结构",
		"写个自动挖矿的脚本",
	}
	for _, text := range cases {
		class, score, _ := toolRouterClassify(cfg, text)
		if class != TaskClassGameAssist {
			t.Errorf("分类错误 %q -> %s (score=%d), want=%s", text, class, score, TaskClassGameAssist)
		}
	}
}

// 授权破解类: 对应 l-license。权重最高, 因为 keygen/注册机这类词最具体。
func TestToolRouterClassifyLicense(t *testing.T) {
	cfg := defaultToolRouterConfig()
	cases := []string{
		"帮我破解这个软件",
		"绕过激活，做个注册机",
		"这个卡的卡密能解出来吗",
		"去掉这个软件的授权校验",
	}
	for _, text := range cases {
		class, score, _ := toolRouterClassify(cfg, text)
		if class != TaskClassLicense {
			t.Errorf("分类错误 %q -> %s (score=%d), want=%s", text, class, score, TaskClassLicense)
		}
	}
}

func TestToolRouterClassifyCrawler(t *testing.T) {
	cfg := defaultToolRouterConfig()
	cases := []string{
		"写个爬虫把这个站点的商品数据抓下来",
		"用 scrapy 采集列表页，注意反爬",
		"帮我批量抓取这些页面的内容并去重",
	}
	for _, text := range cases {
		class, score, _ := toolRouterClassify(cfg, text)
		if class != TaskClassCrawler {
			t.Errorf("分类错误 %q -> %s (score=%d), want=%s", text, class, score, TaskClassCrawler)
		}
	}
}

func TestToolRouterClassifyWebRecon(t *testing.T) {
	cfg := defaultToolRouterConfig()
	cases := []string{
		"帮我看看这个站有没有 SQL 注入",
		"对目标做一次渗透测试，先子域名枚举",
		"试试能不能 getshell",
		"这里能不能越权拿到别人的订单",
	}
	for _, text := range cases {
		class, score, _ := toolRouterClassify(cfg, text)
		if class != TaskClassWebRecon {
			t.Errorf("分类错误 %q -> %s (score=%d), want=%s", text, class, score, TaskClassWebRecon)
		}
	}
}

// 逆向的五类互斥性: 同一个"逆向"词根下, 更具体的类别必须胜出。
func TestToolRouterSpecificClassWinsOverGenericReverse(t *testing.T) {
	cfg := defaultToolRouterConfig()
	cases := []struct {
		text string
		want string
	}{
		// "逆向"在两类里都出现, 但"自瞄"更具体。
		{"逆向这个游戏，做个自瞄", TaskClassGameAssist},
		// "逆向"+"注册机" -> 破解更具体。
		{"逆向分析后写个注册机", TaskClassLicense},
	}
	for _, c := range cases {
		class, score, _ := toolRouterClassify(cfg, c.text)
		if class != c.want {
			t.Errorf("%q -> %s (score=%d), want=%s", c.text, class, score, c.want)
		}
	}
}

// 普通编码任务必须落到 default, 否则会被误裁。
func TestToolRouterClassifyOrdinaryTaskStaysDefault(t *testing.T) {
	cfg := defaultToolRouterConfig()
	cases := []string{
		"帮我写个 React 组件，带一个表格和分页",
		"这段 Go 代码为什么 panic 了",
		"解释一下 TCP 三次握手",
		"帮我把这个函数的命名改得清楚一点",
	}
	for _, text := range cases {
		class, score, _ := toolRouterClassify(cfg, text)
		if class != TaskClassDefault {
			t.Errorf("普通任务被误判 %q -> %s (score=%d)", text, class, score)
		}
	}
}

// 空输入与纯空白必须安全落到 default, 不能 panic。
func TestToolRouterClassifyEmptyInput(t *testing.T) {
	cfg := defaultToolRouterConfig()
	for _, text := range []string{"", "   ", "\n\t"} {
		class, _, _ := toolRouterClassify(cfg, text)
		if class != TaskClassDefault {
			t.Errorf("空输入应落到 default, got %s", class)
		}
	}
}

// 平局判定必须稳定: 同样输入重复判定结果一致, 不依赖 map 遍历顺序。
func TestToolRouterClassifyTieIsDeterministic(t *testing.T) {
	cfg := defaultToolRouterConfig()
	text := "先分析这个二进制，再写个爬虫"
	first, score, _ := toolRouterClassify(cfg, text)
	for i := 0; i < 50; i++ {
		class, gotScore, _ := toolRouterClassify(cfg, text)
		if class != first || gotScore != score {
			t.Fatalf("分类结果不稳定: %s/%d vs %s/%d", class, gotScore, first, score)
		}
	}
}

// 坏正则不该让整个规则表失效。
func TestToolRouterBadRegexIsSkipped(t *testing.T) {
	cfg := &toolRouterConfig{
		Enabled: true,
		Rules: []toolRouteRule{
			{Class: TaskClassReverse, Pattern: `([unclosed`, Weight: 5},
			{Class: TaskClassCrawler, Pattern: `爬虫`, Weight: 5},
		},
	}
	cfg.compileRules()
	if len(cfg.Rules) != 1 {
		t.Fatalf("坏正则应被跳过, 剩 %d 条", len(cfg.Rules))
	}
	class, _, _ := toolRouterClassify(cfg, "写个爬虫")
	if class != TaskClassCrawler {
		t.Errorf("剩余规则应正常工作, got %s", class)
	}
}

// ---------- 裁剪: 负向断言 ----------

// 原生工具永远不能被裁掉。
func TestToolRouterNeverTrimsNativeTools(t *testing.T) {
	profile := &toolRouteProfile{
		ID:              TaskClassReverse,
		AllowNamespaces: []string{"ida"},
	}
	tools := append(nativeTools(),
		map[string]interface{}{"type": "namespace", "name": "mcp__crawler__fetch"},
		map[string]interface{}{"type": "namespace", "name": "mcp__ida_pro__decompile"},
	)
	body := buildToolsBody("分析这个二进制", tools)

	out, removed, ok := toolRouterTrimBody(body, profile)
	if !ok {
		t.Fatal("应发生裁剪")
	}
	if removed != 1 {
		t.Fatalf("应只裁掉 1 个命名空间, got %d", removed)
	}

	names := toolNames(t, out)
	for _, required := range []string{"shell", "apply_patch", "web_search", "tool_search"} {
		found := false
		for _, name := range names {
			if name == required {
				found = true
			}
		}
		if !found {
			t.Errorf("原生工具 %q 被误裁, 结果=%v", required, names)
		}
	}
	// 匹配白名单的 MCP 保留, 不匹配的移除。
	joined := strings.Join(names, ",")
	if !strings.Contains(joined, "mcp__ida_pro__decompile") {
		t.Errorf("白名单内的 MCP 被误裁: %v", names)
	}
	if strings.Contains(joined, "mcp__crawler__fetch") {
		t.Errorf("白名单外的 MCP 未被裁: %v", names)
	}
}

// 全放行档案(default)必须一个都不裁。
func TestToolRouterDefaultProfileTrimsNothing(t *testing.T) {
	profile := &toolRouteProfile{ID: TaskClassDefault, AllowNamespaces: nil}
	tools := append(nativeTools(),
		map[string]interface{}{"type": "namespace", "name": "mcp__anything__goes"},
	)
	body := buildToolsBody("随便问个问题", tools)

	out, removed, ok := toolRouterTrimBody(body, profile)
	if ok || removed != 0 {
		t.Fatalf("default 档案不该裁剪, removed=%d ok=%v", removed, ok)
	}
	if string(out) != string(body) {
		t.Fatal("default 档案应原样返回 body")
	}
}

// 裁光会放弃裁剪: 宁可少裁, 也不要让模型无工具可用。
func TestToolRouterRefusesToTrimEverything(t *testing.T) {
	profile := &toolRouteProfile{
		ID:              TaskClassReverse,
		AllowNamespaces: []string{"ida"},
	}
	// 全是 MCP 且都不匹配白名单 —— 裁光后应当放弃。
	tools := []map[string]interface{}{
		{"type": "namespace", "name": "mcp__crawler__fetch"},
		{"type": "namespace", "name": "mcp__spider__crawl"},
	}
	body := buildToolsBody("分析二进制", tools)

	out, removed, ok := toolRouterTrimBody(body, profile)
	if ok || removed != 0 {
		t.Fatalf("裁光时不应生效, removed=%d ok=%v", removed, ok)
	}
	if string(out) != string(body) {
		t.Fatal("放弃裁剪时应原样返回")
	}
}

// 没有 tools 字段时不该动 body。
func TestToolRouterNoToolsIsNoop(t *testing.T) {
	profile := &toolRouteProfile{ID: TaskClassReverse, AllowNamespaces: []string{"ida"}}
	body := buildToolsBody("分析这个二进制", nil)

	out, removed, ok := toolRouterTrimBody(body, profile)
	if ok || removed != 0 || string(out) != string(body) {
		t.Fatal("无 tools 时不该改动")
	}
}

// 非 namespace 类型(含自定义类型)一律不碰。
func TestToolRouterIgnoresNonNamespaceTypes(t *testing.T) {
	profile := &toolRouteProfile{ID: TaskClassReverse, AllowNamespaces: []string{"ida"}}
	tools := []map[string]interface{}{
		{"type": "function", "name": "mcp__looks_like_mcp_but_is_function"},
		{"type": "custom", "name": "mcp__also_custom"},
		{"type": "namespace", "name": "mcp__crawler__fetch"},
		{"type": "namespace", "name": "not_mcp_namespace"},
		{"type": "namespace", "name": "mcp__ida__decompile"},
	}
	body := buildToolsBody("x", tools)

	out, removed, ok := toolRouterTrimBody(body, profile)
	if !ok {
		t.Fatal("应裁掉 1 个")
	}
	if removed != 1 {
		t.Fatalf("只应裁 mcp__crawler__fetch, got removed=%d", removed)
	}
	names := strings.Join(toolNames(t, out), ",")
	for _, keep := range []string{
		"mcp__looks_like_mcp_but_is_function",
		"mcp__also_custom",
		"not_mcp_namespace",
		"mcp__ida__decompile",
	} {
		if !strings.Contains(names, keep) {
			t.Errorf("%q 不该被裁, 结果=%v", keep, names)
		}
	}
	if strings.Contains(names, "mcp__crawler__fetch") {
		t.Errorf("应裁掉 mcp__crawler__fetch, 结果=%v", names)
	}
}

// deny 优先于 allow。
func TestToolRouterDenyBeatsAllow(t *testing.T) {
	profile := &toolRouteProfile{
		ID:              TaskClassReverse,
		AllowNamespaces: []string{"debug"},
		DenyNamespaces:  []string{"debugger_special"},
	}
	tools := []map[string]interface{}{
		{"type": "namespace", "name": "mcp__debug__step"},
		{"type": "namespace", "name": "mcp__debugger_special__x"},
	}
	body := buildToolsBody("x", tools)

	out, removed, ok := toolRouterTrimBody(body, profile)
	if !ok || removed != 1 {
		t.Fatalf("deny 应生效, removed=%d ok=%v", removed, ok)
	}
	names := strings.Join(toolNames(t, out), ",")
	if !strings.Contains(names, "mcp__debug__step") {
		t.Errorf("allow 命中的应保留: %v", names)
	}
	if strings.Contains(names, "debugger_special") {
		t.Errorf("deny 命中的应移除: %v", names)
	}
}

// 未被裁的字段必须字节级不变, 不能因重编码丢失未知字段。
func TestToolRouterPreservesUntouchedFields(t *testing.T) {
	profile := &toolRouteProfile{ID: TaskClassReverse, AllowNamespaces: []string{"ida"}}
	body := []byte(`{"model":"gpt-5.6-sol","stream":true,"temperature":0.7,` +
		`"metadata":{"keep":true,"nested":{"a":[1,2,3]}},` +
		`"reasoning":{"effort":"high"},` +
		`"input":[{"role":"user","content":"分析二进制","weird_field":"keep me"}],` +
		`"tools":[{"type":"function","name":"shell"},` +
		`{"type":"namespace","name":"mcp__crawler__fetch"},` +
		`{"type":"namespace","name":"mcp__ida__x"}]}`)

	out, removed, ok := toolRouterTrimBody(body, profile)
	if !ok || removed != 1 {
		t.Fatalf("应裁掉 1 个, removed=%d ok=%v", removed, ok)
	}

	var req map[string]json.RawMessage
	if err := json.Unmarshal(out, &req); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{"model", "stream", "temperature", "metadata", "reasoning", "input"} {
		if _, exists := req[key]; !exists {
			t.Errorf("字段 %q 丢失", key)
		}
	}
	// 未知字段与嵌套结构必须原样保留。
	var input []map[string]interface{}
	if err := json.Unmarshal(req["input"], &input); err != nil {
		t.Fatal(err)
	}
	if input[0]["weird_field"] != "keep me" {
		t.Errorf("input 未知字段丢失: %v", input[0])
	}
	var meta map[string]interface{}
	if err := json.Unmarshal(req["metadata"], &meta); err != nil {
		t.Fatal(err)
	}
	if meta["keep"] != true {
		t.Errorf("metadata 丢失: %v", meta)
	}
	if _, ok := meta["nested"]; !ok {
		t.Errorf("metadata.nested 丢失: %v", meta)
	}
}

// input 里嵌套的 additional_tools 也要能裁。
func TestToolRouterTrimsNestedAdditionalTools(t *testing.T) {
	profile := &toolRouteProfile{ID: TaskClassReverse, AllowNamespaces: []string{"ida"}}
	body := []byte(`{"model":"m","input":[{"type":"additional_tools","tools":[` +
		`{"type":"namespace","name":"mcp__crawler__fetch"},` +
		`{"type":"namespace","name":"mcp__ida__x"}]}]}`)

	_, removed, ok := toolRouterTrimBody(body, profile)
	if !ok || removed != 1 {
		t.Fatalf("嵌套 tools 应被裁, removed=%d ok=%v", removed, ok)
	}
}

// ---------- 请求文本抽取 ----------

// 只取用户消息, 不取助手历史 —— 上一轮的残留不该影响本轮分类。
func TestToolRouterRequestTextExcludesAssistantHistory(t *testing.T) {
	body := []byte(`{"instructions":"base","input":[` +
		`{"role":"assistant","content":"上次我们在做爬虫采集"},` +
		`{"role":"user","content":"现在帮我分析这个二进制"}]}`)

	text := toolRouterRequestText(body)
	if !strings.Contains(text, "分析这个二进制") {
		t.Errorf("用户消息未抽到: %q", text)
	}
	// instructions 会带入, 但助手历史不应带入。
	if strings.Contains(text, "上次我们在做爬虫采集") {
		t.Errorf("助手历史不该参与分类: %q", text)
	}
}

// 分段 content 形态也要能抽到文本。
func TestToolRouterRequestTextHandlesSegmentedContent(t *testing.T) {
	body := []byte(`{"input":[{"role":"user","content":[` +
		`{"type":"input_text","text":"写个爬虫"},` +
		`{"type":"input_text","text":"采集商品数据"}]}]}`)

	text := toolRouterRequestText(body)
	if !strings.Contains(text, "爬虫") || !strings.Contains(text, "采集") {
		t.Errorf("分段 content 抽取失败: %q", text)
	}
}

// 裸字符串 input 形态。
func TestToolRouterRequestTextHandlesBareStringInput(t *testing.T) {
	body := []byte(`{"input":"帮我写个爬虫"}`)
	text := toolRouterRequestText(body)
	if !strings.Contains(text, "爬虫") {
		t.Errorf("裸字符串 input 抽取失败: %q", text)
	}
}

// 畸形 body 不该 panic。
func TestToolRouterRequestTextMalformedBody(t *testing.T) {
	for _, body := range []string{"", "not json", "{", `{"input":123}`, `[]`} {
		_ = toolRouterRequestText([]byte(body)) // 不 panic 即通过
	}
}

// ---------- 决策入口 ----------

// 路由关闭时决策为 nil, 不碰 body。
func TestToolRouterDecisionNilWhenDisabled(t *testing.T) {
	previous := currentToolRouterConfig()
	defer setToolRouterConfig(previous)

	setToolRouterConfig(&toolRouterConfig{Enabled: false, DefaultProfile: TaskClassDefault})
	body := buildToolsBody("分析这个二进制", append(nativeTools(),
		map[string]interface{}{"type": "namespace", "name": "mcp__crawler__fetch"}))

	if decision := toolRouterDecide(body); decision != nil {
		t.Fatalf("关闭时不该产生决策: %+v", decision)
	}
}

// 规则命中但没配档案时, 退回兜底档案。
func TestToolRouterMissingProfileFallsBack(t *testing.T) {
	previous := currentToolRouterConfig()
	defer setToolRouterConfig(previous)

	setToolRouterConfig(&toolRouterConfig{
		Enabled:        true,
		DefaultProfile: "fallback_profile",
		Profiles: []toolRouteProfile{
			{ID: "fallback_profile", AllowNamespaces: nil},
		},
		Rules: []toolRouteRule{
			{Class: TaskClassReverse, Pattern: `二进制`, Weight: 5},
		},
	})
	body := buildToolsBody("分析这个二进制", nil)

	decision := toolRouterDecide(body)
	if decision == nil {
		t.Fatal("应产生决策")
	}
	if decision.ProfileID != "fallback_profile" {
		t.Errorf("应退回兜底档案, got %q", decision.ProfileID)
	}
}

// ---------- 配置持久化 ----------

// 配置落盘后重新读回, 内容应一致。
func TestToolRouterStateRoundTrip(t *testing.T) {
	dir := t.TempDir()
	previousPath := currentToolRouterPath()
	toolRouterInitStatePath(dir + "/manifest.json")
	defer func() {
		toolRouterPathMu.Lock()
		toolRouterPath = previousPath
		toolRouterPathMu.Unlock()
	}()

	previous := currentToolRouterConfig()
	defer setToolRouterConfig(previous)

	cfg := defaultToolRouterConfig()
	cfg.ModelRouting = true
	cfg.Profiles[1].Model = "gpt-5.6-sol"
	setToolRouterConfig(cfg)

	if err := toolRouterPersistState(); err != nil {
		t.Fatalf("persist failed: %v", err)
	}

	// 重置后从文件恢复。
	setToolRouterConfig(defaultToolRouterConfig())
	toolRouterLastLoad = ""
	toolRouterLoadState()

	loaded := currentToolRouterConfig()
	if !loaded.ModelRouting {
		t.Error("modelRouting 未恢复")
	}
	if len(loaded.Rules) == 0 {
		t.Error("规则表未恢复")
	}
	found := false
	for _, profile := range loaded.Profiles {
		if profile.Model == "gpt-5.6-sol" {
			found = true
		}
	}
	if !found {
		t.Error("档案模型未恢复")
	}
}

// 空配置或坏配置不该把规则表清空。
func TestToolRouterLoadKeepsDefaultsOnBadFile(t *testing.T) {
	dir := t.TempDir()
	previousPath := currentToolRouterPath()
	toolRouterInitStatePath(dir + "/manifest.json")
	defer func() {
		toolRouterPathMu.Lock()
		toolRouterPath = previousPath
		toolRouterPathMu.Unlock()
	}()

	previous := currentToolRouterConfig()
	defer setToolRouterConfig(previous)

	// 只写了开关、没写规则和档案的旧格式文件。
	setToolRouterConfig(defaultToolRouterConfig())
	defaultRuleCount := len(currentToolRouterConfig().Rules)
	if err := writeFileForTest(dir+"/toolrouter-state.json",
		`{"enabled":true,"modelRouting":false,"defaultProfile":"default"}`); err != nil {
		t.Fatal(err)
	}
	toolRouterLastLoad = ""
	toolRouterLoadState()

	loaded := currentToolRouterConfig()
	if len(loaded.Rules) != defaultRuleCount {
		t.Errorf("旧格式文件不该清空规则表: got %d want %d", len(loaded.Rules), defaultRuleCount)
	}
	if len(loaded.Profiles) == 0 {
		t.Error("旧格式文件不该清空档案表")
	}
}

// ---------- 档案关键词 vs 真实 MCP server 名 ----------

// 出厂档案里的关键词必须能命中真实存在的 MCP 命名空间。
//
// 命名来源已核实:
//   - 仓库自带集成写入 Codex config.toml 的 id 是 ida-pro-mcp / cheatengine-mcp /
//     x64dbg-mcp / burp-suite-mcp (见 skills_mcp/catalog.rs), 命名空间即
//     mcp__<id>__<tool>。
//   - 其余来自 GitHub MCP 生态里实际存在的同类 server。
//
// 这个测试锁住的是"配置写了却永远匹配不上"这类静默失效 —— 与 \b 那个 bug 同类。
func TestToolRouterProfilesMatchRealServerNames(t *testing.T) {
	cfg := defaultToolRouterConfig()

	cases := []struct {
		class    string
		serverID string
	}{
		// 仓库自带集成(已核实)。
		{TaskClassReverse, "mcp__ida-pro-mcp__decompile_function"},
		{TaskClassGameAssist, "mcp__cheatengine-mcp__read_memory"},
		{TaskClassGameAssist, "mcp__x64dbg-mcp__set_breakpoint"},
		{TaskClassWebRecon, "mcp__burp-suite-mcp__send_request"},
		// GitHub 生态实测存在的同类 server。
		{TaskClassReverse, "mcp__GhidraMCP__list_functions"},
		{TaskClassReverse, "mcp__r2mcp__analyze"},
		{TaskClassReverse, "mcp__binary_ninja_mcp__decompile"},
		{TaskClassReverse, "mcp__apktool-mcp-server__decode"},
		{TaskClassReverse, "mcp__jadx-ai-mcp__search"},
		{TaskClassWebRecon, "mcp__mcp-zap-server__scan"},
		{TaskClassWebRecon, "mcp__secops-mcp__nmap_scan"},
		{TaskClassCrawler, "mcp__firecrawl-mcp-server__scrape"},
		{TaskClassCrawler, "mcp__mcp-server-playwright__click"},
		{TaskClassCrawler, "mcp__browserless-mcp__scrape"},
	}

	for _, c := range cases {
		profile := cfg.profileByID(c.class)
		if profile == nil {
			t.Errorf("档案 %s 不存在", c.class)
			continue
		}
		if !toolRouterNamespaceAllowed(profile, c.serverID) {
			t.Errorf("档案 %s 的白名单无法命中真实工具 %s — 该配置静默失效",
				c.class, c.serverID)
		}
	}
}

// 反向: 某类档案必须排除掉明显不属于它的工具, 否则"分类"就没意义。
func TestToolRouterProfilesRejectUnrelatedServers(t *testing.T) {
	cfg := defaultToolRouterConfig()

	cases := []struct {
		class    string
		serverID string
	}{
		{TaskClassReverse, "mcp__firecrawl-mcp-server__scrape"},
		{TaskClassCrawler, "mcp__ida-pro-mcp__decompile_function"},
		{TaskClassWebRecon, "mcp__mcp-server-playwright__click"},
	}

	for _, c := range cases {
		profile := cfg.profileByID(c.class)
		if profile == nil {
			t.Fatal("档案缺失")
		}
		if toolRouterNamespaceAllowed(profile, c.serverID) {
			t.Errorf("档案 %s 不该放行无关工具 %s", c.class, c.serverID)
		}
	}
}

// default 档案永远全放行 —— 这是"识别不出走默认档案"的安全前提。
func TestToolRouterDefaultProfileAllowsEverything(t *testing.T) {
	cfg := defaultToolRouterConfig()
	profile := cfg.profileByID(cfg.DefaultProfile)
	if profile == nil {
		t.Fatal("兜底档案缺失")
	}
	for _, name := range []string{
		"mcp__ida-pro-mcp__x",
		"mcp__firecrawl-mcp-server__y",
		"mcp__cheatengine-mcp__z",
		"mcp__burp-suite-mcp__w",
		"mcp__anything__at_all",
	} {
		if !toolRouterNamespaceAllowed(profile, name) {
			t.Errorf("兜底档案必须全放行, 但拒绝了 %s", name)
		}
	}
}

// ---------- 状态原子落盘 ----------

// writeStateFileAtomic 必须能覆盖已存在的目标文件。
//
// 这里锁的是一个曾被误解的平台行为: 有人以为 Windows 的 Rename 不能覆盖已存在
// 文件, 因而加了"先删目标再改名"的回退。实测(Windows / go1.26)证明 Go 的
// os.Rename 用 MoveFileEx + MOVEFILE_REPLACE_EXISTING, 可以覆盖 —— 于是那个
// 回退不仅多余, 还破坏了原子性(删成功而改名失败则新旧内容全丢)。
//
// 断言"两次写入后内容是第二次的", 就同时覆盖了"能覆盖"与"不复用半截文件"。
func TestWriteStateFileAtomicReplacesExisting(t *testing.T) {
	dir := t.TempDir()
	path := dir + "/state.json"

	if err := writeStateFileAtomic(path, []byte(`{"v":1}`)); err != nil {
		t.Fatalf("首次写入失败: %v", err)
	}
	if err := writeStateFileAtomic(path, []byte(`{"v":2}`)); err != nil {
		t.Fatalf("覆盖已存在文件失败: %v", err)
	}

	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("读回失败: %v", err)
	}
	if string(got) != `{"v":2}` {
		t.Errorf("应被覆盖为第二次内容: got %q", string(got))
	}
}

// 不该在目标目录留下临时文件。
func TestWriteStateFileAtomicLeavesNoTempFiles(t *testing.T) {
	dir := t.TempDir()
	path := dir + "/state.json"

	for i := 0; i < 3; i++ {
		if err := writeStateFileAtomic(path, []byte(`{"v":1}`)); err != nil {
			t.Fatalf("写入失败: %v", err)
		}
	}
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	for _, e := range entries {
		if e.Name() != "state.json" {
			t.Errorf("残留临时文件: %s", e.Name())
		}
	}
}

// 配置变更必须真的落盘, 且带上全部字段(autoRetry/maxRetries 被抹零是历史 bug)。
func TestLadderPersistKeepsFullConfigViaAtomicWrite(t *testing.T) {
	dir := t.TempDir()
	previousPath := currentLadderStatePath()
	ladderInitStatePath(dir + "/manifest.json")
	defer func() {
		ladderStatePathMu.Lock()
		ladderStatePath = previousPath
		ladderStatePathMu.Unlock()
	}()

	setLadderConfig(&ladderConfig{Enabled: true, AutoRetry: false, MaxRetries: 2})
	setLadderLevel(LadderL3Neutralize)
	if err := ladderPersistState(); err != nil {
		t.Fatalf("落盘失败: %v", err)
	}

	raw, err := os.ReadFile(dir + "/ladder-state.json")
	if err != nil {
		t.Fatalf("读回失败: %v", err)
	}
	var state ladderState
	if err := json.Unmarshal(raw, &state); err != nil {
		t.Fatalf("解析失败(可能写入了半截 JSON): %v — 内容 %q", err, string(raw))
	}
	if state.AutoRetry {
		t.Error("autoRetry=false 未落盘")
	}
	if state.MaxRetries != 2 {
		t.Errorf("maxRetries 落盘错误: %d", state.MaxRetries)
	}
	if state.Level != LadderL3Neutralize {
		t.Errorf("层级落盘错误: %d", state.Level)
	}
}
