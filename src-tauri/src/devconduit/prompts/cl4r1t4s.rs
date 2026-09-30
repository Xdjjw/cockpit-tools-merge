//! CL4R1T4S 泄露系统提示词库（github.com/elder-plinius/CL4R1T4S），离线内置。
//! 本文件由 .ctmp/gen_cl4r1t4s.py 生成 —— 更新 resources/cl4r1t4s/ 后重新生成。

use serde::Serialize;

/// 单个泄露提示词资产（OpenAI / Anthropic 家族）。
#[derive(Debug, Clone)]
pub(crate) struct Cl4r1t4sAsset {
    pub(crate) family: &'static str,
    pub(crate) id: &'static str,
    pub(crate) filename: &'static str,
    pub(crate) title: &'static str,
    pub(crate) content: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Cl4r1t4sFamilyInfo {
    pub(crate) family: &'static str,
    pub(crate) count: usize,
}

// ---- ANTHROPIC ----
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_FABLE_5_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/CLAUDE-FABLE-5.md");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_OPUS_5_5_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/CLAUDE-OPUS-5.5.md");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_4_1_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude-4.1.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_4_5_OPUS_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude-4.5-Opus.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_DESIGN_SYS_PROMPT_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude-Design-Sys-Prompt.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_FABLE_5_1_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude-Fable-5.1.md");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_OPUS_4_7_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude-Opus-4.7.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_4_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude_4.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_CODE_03_04_24_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude_Code_03-04-24.md");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_OPUS_4_6_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude_Opus_4.6.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_SONNET_4_5_SEP_29_2025_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude_Sonnet-4.5_Sep-29-2025.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_SONNET_3_5_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude_Sonnet_3.5.md");
pub(crate) const CL4R1T4S_ANTHROPIC_CLAUDE_SONNET_3_7_NEW_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/Claude_Sonnet_3.7_New.txt");
pub(crate) const CL4R1T4S_ANTHROPIC_OPUS_5_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/OPUS-5.md");
pub(crate) const CL4R1T4S_ANTHROPIC_USERSTYLE_MODES_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/ANTHROPIC/UserStyle_Modes.md");

// ---- OPENAI ----
pub(crate) const CL4R1T4S_OPENAI_ATLAS_10_21_25_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Atlas_10-21-25.txt");
pub(crate) const CL4R1T4S_OPENAI_CHATGPT_4O_SEP_27_25_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/ChatGPT-4o_Sep-27-25.txt");
pub(crate) const CL4R1T4S_OPENAI_CHATGPT5_08_07_2025_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/ChatGPT5-08-07-2025.mkd");
pub(crate) const CL4R1T4S_OPENAI_CHATGPT_4_1_05_15_2025_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/ChatGPT_4.1_05-15-2025.txt");
pub(crate) const CL4R1T4S_OPENAI_CHATGPT_4O_04_25_2025_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/ChatGPT_4o_04-25-2025.txt");
pub(crate) const CL4R1T4S_OPENAI_CHATGPT_PERSONALITY_V2_CHANGE_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/ChatGPT_Personality_v2_Change.md");
pub(crate) const CL4R1T4S_OPENAI_CHATGPT_O3_O4_MINI_04_16_2025_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/ChatGPT_o3_o4-mini_04-16-2025");
pub(crate) const CL4R1T4S_OPENAI_CHATKIT_DOCS_OCT_6_25_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/ChatKit_Docs__Oct-6-25.txt");
pub(crate) const CL4R1T4S_OPENAI_CODEX_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex.md");
pub(crate) const CL4R1T4S_OPENAI_CODEX_SEP_15_2025_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex_Sep-15-2025.md");
pub(crate) const CL4R1T4S_OPENAI_GPT_4_5_02_27_25_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/GPT-4.5_02-27-25.md");
pub(crate) const CL4R1T4S_OPENAI_GPT_4O_IMAGE_GEN_POSTFILL_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/GPT-4o_Image_Gen_Postfill.txt");
pub(crate) const CL4R1T4S_OPENAI_CODEX_DESKTOP_5_6_SOL_SYSTEMPROMPT_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex_Desktop/5.6-Sol_SystemPrompt.md");
pub(crate) const CL4R1T4S_OPENAI_CODEX_DESKTOP_5_6_SOL_TOOLS_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex_Desktop/5.6-Sol_Tools.json");
pub(crate) const CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_ASTRA_PROMPTS_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex_Desktop/GPT-6-Astra_Prompts.md");
pub(crate) const CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_ASTRA_TOOLS_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex_Desktop/GPT-6-Astra_Tools.json");
pub(crate) const CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_SOL_PROMPTS_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex_Desktop/GPT-6-Sol_Prompts.txt");
pub(crate) const CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_SOL_TOOLS_CONTENT: &str = include_str!("../../../resources/cl4r1t4s/OPENAI/Codex_Desktop/GPT-6-Sol_Tools.json");

/// 全部 CL4R1T4S 资产（OpenAI + Anthropic）。
pub(crate) fn cl4r1t4s_assets() -> Vec<Cl4r1t4sAsset> {
    vec![
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-fable-5",
            filename: "cl4r1t4s-anthropic-claude-fable-5.md",
            title: "CLAUDE-FABLE-5",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_FABLE_5_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-opus-5-5",
            filename: "cl4r1t4s-anthropic-claude-opus-5-5.md",
            title: "CLAUDE-OPUS-5.5",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_OPUS_5_5_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-4-1",
            filename: "cl4r1t4s-anthropic-claude-4-1.md",
            title: "Claude-4.1",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_4_1_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-4-5-opus",
            filename: "cl4r1t4s-anthropic-claude-4-5-opus.md",
            title: "Claude-4.5-Opus",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_4_5_OPUS_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-design-sys-prompt",
            filename: "cl4r1t4s-anthropic-claude-design-sys-prompt.md",
            title: "Claude-Design-Sys-Prompt",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_DESIGN_SYS_PROMPT_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-fable-5-1",
            filename: "cl4r1t4s-anthropic-claude-fable-5-1.md",
            title: "Claude-Fable-5.1",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_FABLE_5_1_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-opus-4-7",
            filename: "cl4r1t4s-anthropic-claude-opus-4-7.md",
            title: "Claude-Opus-4.7",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_OPUS_4_7_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-4",
            filename: "cl4r1t4s-anthropic-claude-4.md",
            title: "Claude_4",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_4_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-code-03-04-24",
            filename: "cl4r1t4s-anthropic-claude-code-03-04-24.md",
            title: "Claude_Code_03-04-24",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_CODE_03_04_24_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-opus-4-6",
            filename: "cl4r1t4s-anthropic-claude-opus-4-6.md",
            title: "Claude_Opus_4.6",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_OPUS_4_6_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-sonnet-4-5-sep-29-2025",
            filename: "cl4r1t4s-anthropic-claude-sonnet-4-5-sep-29-2025.md",
            title: "Claude_Sonnet-4.5_Sep-29-2025",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_SONNET_4_5_SEP_29_2025_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-sonnet-3-5",
            filename: "cl4r1t4s-anthropic-claude-sonnet-3-5.md",
            title: "Claude_Sonnet_3.5",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_SONNET_3_5_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-claude-sonnet-3-7-new",
            filename: "cl4r1t4s-anthropic-claude-sonnet-3-7-new.md",
            title: "Claude_Sonnet_3.7_New",
            content: CL4R1T4S_ANTHROPIC_CLAUDE_SONNET_3_7_NEW_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-opus-5",
            filename: "cl4r1t4s-anthropic-opus-5.md",
            title: "OPUS-5",
            content: CL4R1T4S_ANTHROPIC_OPUS_5_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "ANTHROPIC",
            id: "cl4r1t4s-anthropic-userstyle-modes",
            filename: "cl4r1t4s-anthropic-userstyle-modes.md",
            title: "UserStyle_Modes",
            content: CL4R1T4S_ANTHROPIC_USERSTYLE_MODES_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-atlas-10-21-25",
            filename: "cl4r1t4s-openai-atlas-10-21-25.md",
            title: "Atlas_10-21-25",
            content: CL4R1T4S_OPENAI_ATLAS_10_21_25_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-chatgpt-4o-sep-27-25",
            filename: "cl4r1t4s-openai-chatgpt-4o-sep-27-25.md",
            title: "ChatGPT-4o_Sep-27-25",
            content: CL4R1T4S_OPENAI_CHATGPT_4O_SEP_27_25_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-chatgpt5-08-07-2025",
            filename: "cl4r1t4s-openai-chatgpt5-08-07-2025.md",
            title: "ChatGPT5-08-07-2025",
            content: CL4R1T4S_OPENAI_CHATGPT5_08_07_2025_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-chatgpt-4-1-05-15-2025",
            filename: "cl4r1t4s-openai-chatgpt-4-1-05-15-2025.md",
            title: "ChatGPT_4.1_05-15-2025",
            content: CL4R1T4S_OPENAI_CHATGPT_4_1_05_15_2025_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-chatgpt-4o-04-25-2025",
            filename: "cl4r1t4s-openai-chatgpt-4o-04-25-2025.md",
            title: "ChatGPT_4o_04-25-2025",
            content: CL4R1T4S_OPENAI_CHATGPT_4O_04_25_2025_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-chatgpt-personality-v2-change",
            filename: "cl4r1t4s-openai-chatgpt-personality-v2-change.md",
            title: "ChatGPT_Personality_v2_Change",
            content: CL4R1T4S_OPENAI_CHATGPT_PERSONALITY_V2_CHANGE_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-chatgpt-o3-o4-mini-04-16-2025",
            filename: "cl4r1t4s-openai-chatgpt-o3-o4-mini-04-16-2025.md",
            title: "ChatGPT_o3_o4-mini_04-16-2025",
            content: CL4R1T4S_OPENAI_CHATGPT_O3_O4_MINI_04_16_2025_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-chatkit-docs-oct-6-25",
            filename: "cl4r1t4s-openai-chatkit-docs-oct-6-25.md",
            title: "ChatKit_Docs__Oct-6-25",
            content: CL4R1T4S_OPENAI_CHATKIT_DOCS_OCT_6_25_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex",
            filename: "cl4r1t4s-openai-codex.md",
            title: "Codex",
            content: CL4R1T4S_OPENAI_CODEX_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex-sep-15-2025",
            filename: "cl4r1t4s-openai-codex-sep-15-2025.md",
            title: "Codex_Sep-15-2025",
            content: CL4R1T4S_OPENAI_CODEX_SEP_15_2025_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-gpt-4-5-02-27-25",
            filename: "cl4r1t4s-openai-gpt-4-5-02-27-25.md",
            title: "GPT-4.5_02-27-25",
            content: CL4R1T4S_OPENAI_GPT_4_5_02_27_25_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-gpt-4o-image-gen-postfill",
            filename: "cl4r1t4s-openai-gpt-4o-image-gen-postfill.md",
            title: "GPT-4o_Image_Gen_Postfill",
            content: CL4R1T4S_OPENAI_GPT_4O_IMAGE_GEN_POSTFILL_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex-desktop-5-6-sol-systemprompt",
            filename: "cl4r1t4s-openai-codex-desktop-5-6-sol-systemprompt.md",
            title: "Codex_Desktop/5.6-Sol_SystemPrompt",
            content: CL4R1T4S_OPENAI_CODEX_DESKTOP_5_6_SOL_SYSTEMPROMPT_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex-desktop-5-6-sol-tools",
            filename: "cl4r1t4s-openai-codex-desktop-5-6-sol-tools.md",
            title: "Codex_Desktop/5.6-Sol_Tools",
            content: CL4R1T4S_OPENAI_CODEX_DESKTOP_5_6_SOL_TOOLS_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex-desktop-gpt-6-astra-prompts",
            filename: "cl4r1t4s-openai-codex-desktop-gpt-6-astra-prompts.md",
            title: "Codex_Desktop/GPT-6-Astra_Prompts",
            content: CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_ASTRA_PROMPTS_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex-desktop-gpt-6-astra-tools",
            filename: "cl4r1t4s-openai-codex-desktop-gpt-6-astra-tools.md",
            title: "Codex_Desktop/GPT-6-Astra_Tools",
            content: CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_ASTRA_TOOLS_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex-desktop-gpt-6-sol-prompts",
            filename: "cl4r1t4s-openai-codex-desktop-gpt-6-sol-prompts.md",
            title: "Codex_Desktop/GPT-6-Sol_Prompts",
            content: CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_SOL_PROMPTS_CONTENT,
        },
        Cl4r1t4sAsset {
            family: "OPENAI",
            id: "cl4r1t4s-openai-codex-desktop-gpt-6-sol-tools",
            filename: "cl4r1t4s-openai-codex-desktop-gpt-6-sol-tools.md",
            title: "Codex_Desktop/GPT-6-Sol_Tools",
            content: CL4R1T4S_OPENAI_CODEX_DESKTOP_GPT_6_SOL_TOOLS_CONTENT,
        },
    ]
}

pub(crate) fn cl4r1t4s_family_count(family: &str) -> usize {
    cl4r1t4s_assets().iter().filter(|a| a.family == family).count()
}
