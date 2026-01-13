//! Internationalization (i18n) module
//!
//! Provides multi-language support for the application.
//! Currently supports English and Chinese.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::sync::RwLock;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    /// English (default)
    #[default]
    English,
    /// Chinese (Simplified)
    Chinese,
}

impl Language {
    /// Get language display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }

    /// Get language code
    #[allow(dead_code)]
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }

    /// Get all available languages
    pub fn all() -> &'static [Language] {
        &[Language::English, Language::Chinese]
    }
}

/// Global language instance
static CURRENT_LANGUAGE: OnceLock<RwLock<Language>> = OnceLock::new();

/// Get the current language
pub fn current_language() -> Language {
    *CURRENT_LANGUAGE
        .get_or_init(|| RwLock::new(Language::default()))
        .read()
        .unwrap()
}

/// Set the current language
pub fn set_language(lang: Language) {
    let lock = CURRENT_LANGUAGE.get_or_init(|| RwLock::new(Language::default()));
    *lock.write().unwrap() = lang;
}

/// Translation strings
pub struct Translations;

impl Translations {
    // ============ Toolbar ============
    pub fn open() -> &'static str {
        match current_language() {
            Language::English => "Open",
            Language::Chinese => "打开",
        }
    }

    pub fn open_file_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Open file (Cmd+O)",
            Language::Chinese => "打开文件 (Cmd+O)",
        }
    }

    pub fn pause() -> &'static str {
        match current_language() {
            Language::English => "Pause",
            Language::Chinese => "暂停",
        }
    }

    pub fn follow() -> &'static str {
        match current_language() {
            Language::English => "Follow",
            Language::Chinese => "跟随",
        }
    }

    pub fn toggle_auto_scroll_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Toggle auto-scroll (Space)",
            Language::Chinese => "切换自动滚动 (Space)",
        }
    }

    pub fn clear() -> &'static str {
        match current_language() {
            Language::English => "Clear",
            Language::Chinese => "清空",
        }
    }

    pub fn clear_display_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Clear display (Cmd+L)",
            Language::Chinese => "清空显示 (Cmd+L)",
        }
    }

    pub fn reload() -> &'static str {
        match current_language() {
            Language::English => "Reload",
            Language::Chinese => "重新加载",
        }
    }

    pub fn reload_file_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Reload file (Cmd+Shift+R)",
            Language::Chinese => "重新加载文件 (Cmd+Shift+R)",
        }
    }

    pub fn newest_first() -> &'static str {
        match current_language() {
            Language::English => "Newest First",
            Language::Chinese => "最新优先",
        }
    }

    pub fn oldest_first() -> &'static str {
        match current_language() {
            Language::English => "Oldest First",
            Language::Chinese => "最旧优先",
        }
    }

    pub fn toggle_order_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Toggle display order (Cmd+R)",
            Language::Chinese => "切换显示顺序 (Cmd+R)",
        }
    }

    pub fn search() -> &'static str {
        match current_language() {
            Language::English => "Search",
            Language::Chinese => "搜索",
        }
    }

    pub fn toggle_search_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Toggle search (Cmd+F)",
            Language::Chinese => "切换搜索 (Cmd+F)",
        }
    }

    pub fn go_to() -> &'static str {
        match current_language() {
            Language::English => "Go to",
            Language::Chinese => "跳转",
        }
    }

    pub fn go_to_line_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Go to line (Cmd+G)",
            Language::Chinese => "跳转到行 (Cmd+G)",
        }
    }

    pub fn go_to_top_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Go to top (Home)",
            Language::Chinese => "跳转到顶部 (Home)",
        }
    }

    pub fn go_to_bottom_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Go to bottom (End)",
            Language::Chinese => "跳转到底部 (End)",
        }
    }

    pub fn toggle_theme() -> &'static str {
        match current_language() {
            Language::English => "Toggle theme",
            Language::Chinese => "切换主题",
        }
    }

    pub fn settings() -> &'static str {
        match current_language() {
            Language::English => "Settings",
            Language::Chinese => "设置",
        }
    }

    // ============ Search Bar ============
    pub fn search_placeholder() -> &'static str {
        match current_language() {
            Language::English => "Search...",
            Language::Chinese => "搜索...",
        }
    }

    pub fn case_sensitive() -> &'static str {
        match current_language() {
            Language::English => "Case sensitive",
            Language::Chinese => "区分大小写",
        }
    }

    pub fn use_regex() -> &'static str {
        match current_language() {
            Language::English => "Use regular expression",
            Language::Chinese => "使用正则表达式",
        }
    }

    pub fn match_whole_word() -> &'static str {
        match current_language() {
            Language::English => "Match whole word",
            Language::Chinese => "全字匹配",
        }
    }

    pub fn previous_match_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Previous match (Shift+F3)",
            Language::Chinese => "上一个匹配 (Shift+F3)",
        }
    }

    pub fn next_match_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Next match (F3)",
            Language::Chinese => "下一个匹配 (F3)",
        }
    }

    pub fn no_results() -> &'static str {
        match current_language() {
            Language::English => "No results",
            Language::Chinese => "无结果",
        }
    }

    pub fn close_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Close (Esc)",
            Language::Chinese => "关闭 (Esc)",
        }
    }

    // ============ Filter Panel ============
    pub fn levels() -> &'static str {
        match current_language() {
            Language::English => "Levels:",
            Language::Chinese => "级别:",
        }
    }

    pub fn all() -> &'static str {
        match current_language() {
            Language::English => "All",
            Language::Chinese => "全部",
        }
    }

    pub fn show_all_levels() -> &'static str {
        match current_language() {
            Language::English => "Show all levels",
            Language::Chinese => "显示所有级别",
        }
    }

    pub fn errors() -> &'static str {
        match current_language() {
            Language::English => "Errors",
            Language::Chinese => "错误",
        }
    }

    pub fn errors_and_warnings_only() -> &'static str {
        match current_language() {
            Language::English => "Show only errors and warnings",
            Language::Chinese => "仅显示错误和警告",
        }
    }

    pub fn less() -> &'static str {
        match current_language() {
            Language::English => "Less",
            Language::Chinese => "收起",
        }
    }

    pub fn more() -> &'static str {
        match current_language() {
            Language::English => "More",
            Language::Chinese => "展开",
        }
    }

    pub fn advanced_filters() -> &'static str {
        match current_language() {
            Language::English => "Advanced Filters",
            Language::Chinese => "高级过滤",
        }
    }

    pub fn bookmarks_only() -> &'static str {
        match current_language() {
            Language::English => "Bookmarks only",
            Language::Chinese => "仅显示书签",
        }
    }

    pub fn exclude_patterns() -> &'static str {
        match current_language() {
            Language::English => "Exclude patterns:",
            Language::Chinese => "排除模式:",
        }
    }

    pub fn exclude_pattern_hint() -> &'static str {
        match current_language() {
            Language::English => "Enter pattern to exclude...",
            Language::Chinese => "输入要排除的模式...",
        }
    }

    pub fn add() -> &'static str {
        match current_language() {
            Language::English => "Add",
            Language::Chinese => "添加",
        }
    }

    pub fn clear_all_filters() -> &'static str {
        match current_language() {
            Language::English => "Clear All Filters",
            Language::Chinese => "清除所有过滤",
        }
    }

    // ============ Status Bar ============
    pub fn copy_path() -> &'static str {
        match current_language() {
            Language::English => "Copy path",
            Language::Chinese => "复制路径",
        }
    }

    pub fn lines() -> &'static str {
        match current_language() {
            Language::English => "lines",
            Language::Chinese => "行",
        }
    }

    pub fn selected() -> &'static str {
        match current_language() {
            Language::English => "selected",
            Language::Chinese => "已选",
        }
    }

    pub fn auto() -> &'static str {
        match current_language() {
            Language::English => "Auto",
            Language::Chinese => "自动",
        }
    }

    pub fn manual() -> &'static str {
        match current_language() {
            Language::English => "Manual",
            Language::Chinese => "手动",
        }
    }

    pub fn memory() -> &'static str {
        match current_language() {
            Language::English => "Mem",
            Language::Chinese => "内存",
        }
    }

    // ============ Activity Bar ============
    pub fn explorer() -> &'static str {
        match current_language() {
            Language::English => "Explorer",
            Language::Chinese => "资源管理器",
        }
    }

    pub fn server_running() -> &'static str {
        match current_language() {
            Language::English => "Server running (port {})\n{} connections",
            Language::Chinese => "服务运行中 (端口 {})\n{} 个连接",
        }
    }

    pub fn server_waiting() -> &'static str {
        match current_language() {
            Language::English => "Server running (port {})\nWaiting for connections...",
            Language::Chinese => "服务运行中 (端口 {})\n等待连接...",
        }
    }

    pub fn server_stopped() -> &'static str {
        match current_language() {
            Language::English => "Server stopped",
            Language::Chinese => "服务未启动",
        }
    }

    // ============ Explorer Panel ============
    pub fn open_editors() -> &'static str {
        match current_language() {
            Language::English => "OPEN EDITORS",
            Language::Chinese => "已打开",
        }
    }

    pub fn no_open_files() -> &'static str {
        match current_language() {
            Language::English => "No open files",
            Language::Chinese => "无打开的文件",
        }
    }

    pub fn remote_streams() -> &'static str {
        match current_language() {
            Language::English => "REMOTE STREAMS",
            Language::Chinese => "远程流",
        }
    }

    pub fn waiting_for_connections() -> &'static str {
        match current_language() {
            Language::English => "Waiting for connections...",
            Language::Chinese => "等待连接...",
        }
    }

    pub fn agents_will_appear() -> &'static str {
        match current_language() {
            Language::English => "Agents will appear after connecting",
            Language::Chinese => "Agent 将在连接后显示",
        }
    }

    pub fn offline() -> &'static str {
        match current_language() {
            Language::English => "offline",
            Language::Chinese => "离线",
        }
    }

    pub fn project() -> &'static str {
        match current_language() {
            Language::English => "Project",
            Language::Chinese => "项目",
        }
    }

    pub fn address() -> &'static str {
        match current_language() {
            Language::English => "Address",
            Language::Chinese => "地址",
        }
    }

    pub fn status() -> &'static str {
        match current_language() {
            Language::English => "Status",
            Language::Chinese => "状态",
        }
    }

    pub fn received() -> &'static str {
        match current_language() {
            Language::English => "Received",
            Language::Chinese => "接收",
        }
    }

    pub fn local_files() -> &'static str {
        match current_language() {
            Language::English => "LOCAL FILES",
            Language::Chinese => "本地文件",
        }
    }

    pub fn no_recent_files() -> &'static str {
        match current_language() {
            Language::English => "No recent files",
            Language::Chinese => "无最近文件",
        }
    }

    pub fn open_file() -> &'static str {
        match current_language() {
            Language::English => "Open file...",
            Language::Chinese => "打开文件...",
        }
    }

    // ============ Settings Panel ============
    pub fn settings_title() -> &'static str {
        match current_language() {
            Language::English => "Settings",
            Language::Chinese => "设置",
        }
    }

    pub fn remote_service() -> &'static str {
        match current_language() {
            Language::English => "Remote Service",
            Language::Chinese => "远程服务",
        }
    }

    pub fn listen_port() -> &'static str {
        match current_language() {
            Language::English => "Listen port:",
            Language::Chinese => "监听端口:",
        }
    }

    pub fn auto_start_server() -> &'static str {
        match current_language() {
            Language::English => "Auto-start server on launch",
            Language::Chinese => "启动时自动开启服务",
        }
    }

    pub fn mcp_service() -> &'static str {
        match current_language() {
            Language::English => "MCP Service (AI Integration)",
            Language::Chinese => "MCP服务 (AI集成)",
        }
    }

    pub fn enable_mcp() -> &'static str {
        match current_language() {
            Language::English => "Enable MCP server",
            Language::Chinese => "启用MCP服务",
        }
    }

    pub fn mcp_port() -> &'static str {
        match current_language() {
            Language::English => "MCP port:",
            Language::Chinese => "MCP端口:",
        }
    }

    pub fn mcp_endpoint() -> &'static str {
        match current_language() {
            Language::English => "Endpoint:",
            Language::Chinese => "端点:",
        }
    }

    pub fn cache_directory() -> &'static str {
        match current_language() {
            Language::English => "Cache directory:",
            Language::Chinese => "缓存目录:",
        }
    }

    pub fn appearance() -> &'static str {
        match current_language() {
            Language::English => "Appearance",
            Language::Chinese => "外观",
        }
    }

    pub fn dark_theme() -> &'static str {
        match current_language() {
            Language::English => "Dark theme",
            Language::Chinese => "深色主题",
        }
    }

    pub fn language() -> &'static str {
        match current_language() {
            Language::English => "Language",
            Language::Chinese => "语言",
        }
    }

    pub fn about() -> &'static str {
        match current_language() {
            Language::English => "About",
            Language::Chinese => "关于",
        }
    }

    pub fn app_description() -> &'static str {
        match current_language() {
            Language::English => "High-performance real-time log viewer",
            Language::Chinese => "高性能实时日志查看器",
        }
    }

    pub fn documentation() -> &'static str {
        match current_language() {
            Language::English => "Documentation",
            Language::Chinese => "文档",
        }
    }

    // ============ Go to Line Dialog ============
    #[allow(dead_code)]
    pub fn go_to_line() -> &'static str {
        match current_language() {
            Language::English => "Go to Line",
            Language::Chinese => "跳转到行",
        }
    }

    #[allow(dead_code)]
    pub fn line_number() -> &'static str {
        match current_language() {
            Language::English => "Line number:",
            Language::Chinese => "行号:",
        }
    }

    #[allow(dead_code)]
    pub fn go() -> &'static str {
        match current_language() {
            Language::English => "Go",
            Language::Chinese => "跳转",
        }
    }

    #[allow(dead_code)]
    pub fn cancel() -> &'static str {
        match current_language() {
            Language::English => "Cancel",
            Language::Chinese => "取消",
        }
    }

    // ============ Messages ============
    #[allow(dead_code)]
    pub fn file_opened() -> &'static str {
        match current_language() {
            Language::English => "File opened",
            Language::Chinese => "文件已打开",
        }
    }

    #[allow(dead_code)]
    pub fn file_reloaded() -> &'static str {
        match current_language() {
            Language::English => "File reloaded",
            Language::Chinese => "文件已重新加载",
        }
    }

    #[allow(dead_code)]
    pub fn reload_failed() -> &'static str {
        match current_language() {
            Language::English => "Reload failed",
            Language::Chinese => "重新加载失败",
        }
    }

    #[allow(dead_code)]
    pub fn no_file_to_reload() -> &'static str {
        match current_language() {
            Language::English => "No file to reload",
            Language::Chinese => "无文件可重新加载",
        }
    }

    #[allow(dead_code)]
    pub fn file_rotated() -> &'static str {
        match current_language() {
            Language::English => "File rotated, reloading...",
            Language::Chinese => "文件已轮转，正在重新加载...",
        }
    }

    #[allow(dead_code)]
    pub fn error() -> &'static str {
        match current_language() {
            Language::English => "Error",
            Language::Chinese => "错误",
        }
    }

    #[allow(dead_code)]
    pub fn server_started() -> &'static str {
        match current_language() {
            Language::English => "Server started on port {}",
            Language::Chinese => "服务已在端口 {} 启动",
        }
    }

    #[allow(dead_code)]
    pub fn server_start_failed() -> &'static str {
        match current_language() {
            Language::English => "Failed to start server",
            Language::Chinese => "启动服务失败",
        }
    }

    #[allow(dead_code)]
    pub fn agent_connected() -> &'static str {
        match current_language() {
            Language::English => "Agent connected: {}",
            Language::Chinese => "Agent 已连接: {}",
        }
    }

    #[allow(dead_code)]
    pub fn agent_disconnected() -> &'static str {
        match current_language() {
            Language::English => "Agent disconnected: {}",
            Language::Chinese => "Agent 已断开: {}",
        }
    }

    #[allow(dead_code)]
    pub fn lines_copied() -> &'static str {
        match current_language() {
            Language::English => "Copied {} lines",
            Language::Chinese => "已复制 {} 行",
        }
    }

    #[allow(dead_code)]
    pub fn display_cleared() -> &'static str {
        match current_language() {
            Language::English => "Display cleared",
            Language::Chinese => "显示已清空",
        }
    }

    #[allow(dead_code)]
    pub fn bookmarks_cleared() -> &'static str {
        match current_language() {
            Language::English => "Bookmarks cleared",
            Language::Chinese => "书签已清除",
        }
    }

    // ============ Display Settings ============
    pub fn display() -> &'static str {
        match current_language() {
            Language::English => "Display",
            Language::Chinese => "显示",
        }
    }

    pub fn font_size() -> &'static str {
        match current_language() {
            Language::English => "Font size:",
            Language::Chinese => "字体大小:",
        }
    }

    pub fn line_height() -> &'static str {
        match current_language() {
            Language::English => "Line height:",
            Language::Chinese => "行高:",
        }
    }

    pub fn show_line_numbers() -> &'static str {
        match current_language() {
            Language::English => "Show line numbers",
            Language::Chinese => "显示行号",
        }
    }

    pub fn word_wrap() -> &'static str {
        match current_language() {
            Language::English => "Word wrap",
            Language::Chinese => "自动换行",
        }
    }

    pub fn show_row_separator() -> &'static str {
        match current_language() {
            Language::English => "Show row separator",
            Language::Chinese => "显示行分隔线",
        }
    }

    // ============ Global Search ============
    pub fn global_search_placeholder() -> &'static str {
        match current_language() {
            Language::English => "Search in logs...",
            Language::Chinese => "搜索日志内容...",
        }
    }

    pub fn results() -> &'static str {
        match current_language() {
            Language::English => "results",
            Language::Chinese => "个结果",
        }
    }

    pub fn global_no_results() -> &'static str {
        match current_language() {
            Language::English => "No results found",
            Language::Chinese => "未找到结果",
        }
    }

    pub fn enter_search_query() -> &'static str {
        match current_language() {
            Language::English => "Enter a search query to find logs",
            Language::Chinese => "输入关键词搜索日志",
        }
    }

    pub fn level_filter() -> &'static str {
        match current_language() {
            Language::English => "Level:",
            Language::Chinese => "级别:",
        }
    }
}

/// Convenient macro for translations
#[macro_export]
macro_rules! t {
    ($key:ident) => {
        $crate::i18n::Translations::$key()
    };
}
