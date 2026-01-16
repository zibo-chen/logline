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

    pub fn stop() -> &'static str {
        match current_language() {
            Language::English => "Stop",
            Language::Chinese => "停止",
        }
    }

    pub fn start() -> &'static str {
        match current_language() {
            Language::English => "Start",
            Language::Chinese => "开始",
        }
    }

    pub fn toggle_monitoring_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Start/Stop monitoring new logs (Space)",
            Language::Chinese => "开始/停止监听新日志 (Space)",
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
            Language::English => "Reverse",
            Language::Chinese => "倒序",
        }
    }

    pub fn oldest_first() -> &'static str {
        match current_language() {
            Language::English => "Normal",
            Language::Chinese => "正序",
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

    pub fn filters() -> &'static str {
        match current_language() {
            Language::English => "Filters",
            Language::Chinese => "过滤器",
        }
    }

    pub fn bookmarks() -> &'static str {
        match current_language() {
            Language::English => "Bookmarks",
            Language::Chinese => "书签",
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

    pub fn open_file_dialog_title() -> &'static str {
        match current_language() {
            Language::English => "Open File",
            Language::Chinese => "打开文件",
        }
    }

    pub fn file_path_input_hint() -> &'static str {
        match current_language() {
            Language::English => "Enter file path or start typing to search...",
            Language::Chinese => "输入文件路径或开始输入搜索文件...",
        }
    }

    pub fn browse_button() -> &'static str {
        match current_language() {
            Language::English => "Browse...",
            Language::Chinese => "浏览...",
        }
    }

    pub fn recent_files_label() -> &'static str {
        match current_language() {
            Language::English => "Recent files:",
            Language::Chinese => "最近打开:",
        }
    }

    pub fn start_typing_hint() -> &'static str {
        match current_language() {
            Language::English => "Start typing to search files...",
            Language::Chinese => "开始输入以搜索文件...",
        }
    }

    pub fn file_encoding() -> &'static str {
        match current_language() {
            Language::English => "Encoding:",
            Language::Chinese => "文件编码:",
        }
    }

    pub fn file_encoding_hint() -> &'static str {
        match current_language() {
            Language::English => "(Auto-detect if not specified)",
            Language::Chinese => "(未指定时自动检测)",
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

    pub fn letter_spacing() -> &'static str {
        match current_language() {
            Language::English => "Letter spacing:",
            Language::Chinese => "字符间距:",
        }
    }

    pub fn show_line_numbers() -> &'static str {
        match current_language() {
            Language::English => "Show line numbers",
            Language::Chinese => "显示行号",
        }
    }

    pub fn show_row_separator() -> &'static str {
        match current_language() {
            Language::English => "Show row separator",
            Language::Chinese => "显示行分隔线",
        }
    }

    pub fn show_grok_fields() -> &'static str {
        match current_language() {
            Language::English => "Show Grok formatted output",
            Language::Chinese => "显示 Grok 格式化输出",
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

    // ============ Advanced Filters Panel ============
    pub fn log_levels() -> &'static str {
        match current_language() {
            Language::English => "Log Levels",
            Language::Chinese => "日志级别",
        }
    }

    pub fn no_exclude_patterns() -> &'static str {
        match current_language() {
            Language::English => "No exclude patterns",
            Language::Chinese => "无排除模式",
        }
    }

    pub fn text_pattern() -> &'static str {
        match current_language() {
            Language::English => "Text pattern",
            Language::Chinese => "文本模式",
        }
    }

    pub fn regex_pattern() -> &'static str {
        match current_language() {
            Language::English => "Regex pattern",
            Language::Chinese => "正则表达式模式",
        }
    }

    pub fn add_pattern() -> &'static str {
        match current_language() {
            Language::English => "Add Pattern",
            Language::Chinese => "添加模式",
        }
    }

    pub fn pattern_type() -> &'static str {
        match current_language() {
            Language::English => "Type:",
            Language::Chinese => "类型:",
        }
    }

    pub fn text() -> &'static str {
        match current_language() {
            Language::English => "Text",
            Language::Chinese => "文本",
        }
    }

    pub fn regex() -> &'static str {
        match current_language() {
            Language::English => "Regex",
            Language::Chinese => "正则",
        }
    }

    pub fn exclude_regex_hint() -> &'static str {
        match current_language() {
            Language::English => "Enter regex pattern to exclude...",
            Language::Chinese => "输入要排除的正则表达式...",
        }
    }

    pub fn regex_help() -> &'static str {
        match current_language() {
            Language::English => "Regex syntax: . * + ? [] {} () | ^ $ \\",
            Language::Chinese => "正则语法: . * + ? [] {} () | ^ $ \\",
        }
    }

    // ============ Bookmarks Panel ============
    pub fn no_bookmarks() -> &'static str {
        match current_language() {
            Language::English => "No bookmarks",
            Language::Chinese => "无书签",
        }
    }

    pub fn bookmark_hint() -> &'static str {
        match current_language() {
            Language::English => "Press Cmd+B or right-click to add bookmarks",
            Language::Chinese => "按 Cmd+B 或右键点击添加书签",
        }
    }

    pub fn total_segments() -> &'static str {
        match current_language() {
            Language::English => "Segments",
            Language::Chinese => "分段",
        }
    }

    pub fn total_bookmarks() -> &'static str {
        match current_language() {
            Language::English => "Total",
            Language::Chinese => "总计",
        }
    }

    pub fn line() -> &'static str {
        match current_language() {
            Language::English => "Line",
            Language::Chinese => "行",
        }
    }

    pub fn remove_segment() -> &'static str {
        match current_language() {
            Language::English => "Remove segment",
            Language::Chinese => "移除分段",
        }
    }

    #[allow(dead_code)]
    pub fn and() -> &'static str {
        match current_language() {
            Language::English => "and",
            Language::Chinese => "及",
        }
    }

    #[allow(dead_code)]
    pub fn more_lines() -> &'static str {
        match current_language() {
            Language::English => "more lines",
            Language::Chinese => "行",
        }
    }

    pub fn clear_all_bookmarks() -> &'static str {
        match current_language() {
            Language::English => "Clear All Bookmarks",
            Language::Chinese => "清除所有书签",
        }
    }

    // ============ Tab Bar ============
    pub fn close() -> &'static str {
        match current_language() {
            Language::English => "Close",
            Language::Chinese => "关闭",
        }
    }

    pub fn close_others() -> &'static str {
        match current_language() {
            Language::English => "Close Others",
            Language::Chinese => "关闭其他",
        }
    }

    pub fn close_tabs_to_right() -> &'static str {
        match current_language() {
            Language::English => "Close Tabs to the Right",
            Language::Chinese => "关闭右侧标签",
        }
    }

    pub fn close_all() -> &'static str {
        match current_language() {
            Language::English => "Close All",
            Language::Chinese => "关闭全部",
        }
    }

    pub fn remote_stream() -> &'static str {
        match current_language() {
            Language::English => "Remote Stream",
            Language::Chinese => "远程流",
        }
    }

    pub fn no_open_tabs() -> &'static str {
        match current_language() {
            Language::English => "No open files. Open a file or connect a remote stream.",
            Language::Chinese => "没有打开的文件。请打开文件或连接远程流。",
        }
    }

    // Split view related translations
    pub fn split_view() -> &'static str {
        match current_language() {
            Language::English => "Split",
            Language::Chinese => "分屏",
        }
    }

    pub fn close_split() -> &'static str {
        match current_language() {
            Language::English => "Close Split",
            Language::Chinese => "关闭分屏",
        }
    }

    pub fn toggle_split_tooltip() -> &'static str {
        match current_language() {
            Language::English => "Toggle split view to show two files side by side",
            Language::Chinese => "切换分屏视图，并排显示两个文件",
        }
    }

    pub fn open_in_split() -> &'static str {
        match current_language() {
            Language::English => "Open in Split View",
            Language::Chinese => "在分屏中打开",
        }
    }

    // ============ Explorer Context Menu ============
    pub fn copy_absolute_path() -> &'static str {
        match current_language() {
            Language::English => "Copy Absolute Path",
            Language::Chinese => "复制绝对路径",
        }
    }

    pub fn copy_relative_path() -> &'static str {
        match current_language() {
            Language::English => "Copy Relative Path",
            Language::Chinese => "复制相对路径",
        }
    }

    pub fn copy_filename() -> &'static str {
        match current_language() {
            Language::English => "Copy Filename",
            Language::Chinese => "复制文件名",
        }
    }

    pub fn reveal_in_finder() -> &'static str {
        match current_language() {
            Language::English => "Reveal in Finder",
            Language::Chinese => "在访达中显示",
        }
    }

    pub fn open_file_context() -> &'static str {
        match current_language() {
            Language::English => "Open File",
            Language::Chinese => "打开文件",
        }
    }

    pub fn remove_from_recent() -> &'static str {
        match current_language() {
            Language::English => "Remove from Recent Files",
            Language::Chinese => "从最近文件中移除",
        }
    }

    pub fn clear_recent_files() -> &'static str {
        match current_language() {
            Language::English => "Clear Recent Files",
            Language::Chinese => "清空最近文件",
        }
    }

    // ============ Welcome/Empty State ============
    pub fn welcome_title() -> &'static str {
        match current_language() {
            Language::English => "Welcome to Logline",
            Language::Chinese => "欢迎使用 Logline",
        }
    }

    pub fn keyboard_shortcuts_title() -> &'static str {
        match current_language() {
            Language::English => "⌨ Keyboard Shortcuts",
            Language::Chinese => "⌨ 快捷键",
        }
    }

    pub fn shortcut_open_file() -> &'static str {
        match current_language() {
            Language::English => "Cmd+O - Open file",
            Language::Chinese => "Cmd+O - 打开文件",
        }
    }

    pub fn shortcut_find() -> &'static str {
        match current_language() {
            Language::English => "Cmd+F - Search in file",
            Language::Chinese => "Cmd+F - 文件内搜索",
        }
    }

    pub fn shortcut_goto_line() -> &'static str {
        match current_language() {
            Language::English => "Cmd+G - Go to line",
            Language::Chinese => "Cmd+G - 跳转到行",
        }
    }

    pub fn shortcut_reload() -> &'static str {
        match current_language() {
            Language::English => "Cmd+Shift+R - Reload file",
            Language::Chinese => "Cmd+Shift+R - 重新加载文件",
        }
    }

    pub fn shortcut_clear() -> &'static str {
        match current_language() {
            Language::English => "Cmd+L - Clear buffer",
            Language::Chinese => "Cmd+L - 清空缓冲区",
        }
    }

    pub fn shortcut_bookmark() -> &'static str {
        match current_language() {
            Language::English => "Cmd+B - Toggle bookmark",
            Language::Chinese => "Cmd+B - 切换书签",
        }
    }

    pub fn shortcut_auto_scroll() -> &'static str {
        match current_language() {
            Language::English => "Space - Toggle auto-scroll",
            Language::Chinese => "Space - 切换自动滚动",
        }
    }

    // ============ Grok Parser ============
    pub fn grok_parser() -> &'static str {
        match current_language() {
            Language::English => "Grok Parser",
            Language::Chinese => "Grok 解析器",
        }
    }

    pub fn grok_builtin_patterns() -> &'static str {
        match current_language() {
            Language::English => "Built-in Patterns",
            Language::Chinese => "内置模板",
        }
    }

    pub fn grok_custom_patterns() -> &'static str {
        match current_language() {
            Language::English => "Custom Patterns",
            Language::Chinese => "自定义模板",
        }
    }

    pub fn grok_pattern_name() -> &'static str {
        match current_language() {
            Language::English => "Pattern Name",
            Language::Chinese => "模板名称",
        }
    }

    pub fn grok_pattern_string() -> &'static str {
        match current_language() {
            Language::English => "Pattern",
            Language::Chinese => "模板表达式",
        }
    }

    pub fn grok_pattern_description() -> &'static str {
        match current_language() {
            Language::English => "Description",
            Language::Chinese => "描述",
        }
    }

    pub fn grok_pattern_example() -> &'static str {
        match current_language() {
            Language::English => "Example",
            Language::Chinese => "示例",
        }
    }

    pub fn grok_test_pattern() -> &'static str {
        match current_language() {
            Language::English => "Test Pattern",
            Language::Chinese => "测试模板",
        }
    }

    pub fn grok_test_input() -> &'static str {
        match current_language() {
            Language::English => "Test Input",
            Language::Chinese => "测试输入",
        }
    }

    pub fn grok_test_result() -> &'static str {
        match current_language() {
            Language::English => "Test Result",
            Language::Chinese => "测试结果",
        }
    }

    pub fn grok_add_pattern() -> &'static str {
        match current_language() {
            Language::English => "Add Pattern",
            Language::Chinese => "添加模板",
        }
    }

    pub fn grok_edit_pattern() -> &'static str {
        match current_language() {
            Language::English => "Edit Pattern",
            Language::Chinese => "编辑模板",
        }
    }

    pub fn grok_delete_pattern() -> &'static str {
        match current_language() {
            Language::English => "Delete Pattern",
            Language::Chinese => "删除模板",
        }
    }

    pub fn grok_save_pattern() -> &'static str {
        match current_language() {
            Language::English => "Save Pattern",
            Language::Chinese => "保存模板",
        }
    }

    pub fn grok_cancel() -> &'static str {
        match current_language() {
            Language::English => "Cancel",
            Language::Chinese => "取消",
        }
    }

    pub fn grok_no_pattern_selected() -> &'static str {
        match current_language() {
            Language::English => "No pattern selected",
            Language::Chinese => "未选择模板",
        }
    }

    pub fn grok_parsed_fields() -> &'static str {
        match current_language() {
            Language::English => "Parsed Fields",
            Language::Chinese => "解析字段",
        }
    }

    pub fn grok_no_match() -> &'static str {
        match current_language() {
            Language::English => "No match",
            Language::Chinese => "无匹配",
        }
    }

    pub fn grok_pattern_error() -> &'static str {
        match current_language() {
            Language::English => "Pattern Error",
            Language::Chinese => "模板错误",
        }
    }

    pub fn grok_display_template() -> &'static str {
        match current_language() {
            Language::English => "Display Template",
            Language::Chinese => "展示模板",
        }
    }

    pub fn grok_display_template_hint() -> &'static str {
        match current_language() {
            Language::English => {
                "Use %{field} to reference parsed fields. Supports colors: %{field:color=red} or %{field:color=#FF0000}. Date formatting: %{timestamp:format=%Y-%m-%d}. Leave empty to show original."
            }
            Language::Chinese => "使用 %{字段名} 引用解析的字段。支持颜色：%{字段名:color=red} 或 %{字段名:color=#FF0000}。日期格式化：%{timestamp:format=%Y-%m-%d}。留空则显示原始内容。",
        }
    }

    pub fn grok_formatted_preview() -> &'static str {
        match current_language() {
            Language::English => "Formatted Preview",
            Language::Chinese => "格式化预览",
        }
    }

    pub fn grok_active_pattern() -> &'static str {
        match current_language() {
            Language::English => "Active Pattern",
            Language::Chinese => "当前模板",
        }
    }

    pub fn grok_no_custom_patterns() -> &'static str {
        match current_language() {
            Language::English => "No custom patterns defined",
            Language::Chinese => "暂无自定义模板",
        }
    }

    pub fn grok_pattern_definitions() -> &'static str {
        match current_language() {
            Language::English => "Pattern Definitions",
            Language::Chinese => "模板定义",
        }
    }

    pub fn grok_none() -> &'static str {
        match current_language() {
            Language::English => "None",
            Language::Chinese => "无",
        }
    }

    pub fn grok_pattern_cleared() -> &'static str {
        match current_language() {
            Language::English => "Grok pattern cleared",
            Language::Chinese => "Grok 模板已清除",
        }
    }

    pub fn grok_panel_hint() -> &'static str {
        match current_language() {
            Language::English => "Configure patterns here. Select pattern in status bar.",
            Language::Chinese => "在此配置模板，通过状态栏选择使用的模板。",
        }
    }

    pub fn grok_add_definition() -> &'static str {
        match current_language() {
            Language::English => "Add Definition",
            Language::Chinese => "添加定义",
        }
    }

    pub fn grok_definition_name() -> &'static str {
        match current_language() {
            Language::English => "Definition Name",
            Language::Chinese => "定义名称",
        }
    }

    pub fn grok_definition_pattern() -> &'static str {
        match current_language() {
            Language::English => "Definition Pattern",
            Language::Chinese => "定义模板",
        }
    }

    // ============ AI Assist ============
    pub fn grok_ai_assist() -> &'static str {
        match current_language() {
            Language::English => "AI Assist",
            Language::Chinese => "AI辅助",
        }
    }

    pub fn grok_ai_generate_prompt() -> &'static str {
        match current_language() {
            Language::English => "Generate Prompt",
            Language::Chinese => "生成提示词",
        }
    }

    pub fn grok_ai_prompt_hint() -> &'static str {
        match current_language() {
            Language::English => "Click to generate a prompt for LLM. Copy the prompt, send it to your favorite LLM (ChatGPT, Claude, etc.), then paste the JSON response below.",
            Language::Chinese => "点击生成提示词。将提示词复制给您喜欢的LLM（如ChatGPT、Claude等），然后将返回的JSON粘贴到下方。",
        }
    }

    pub fn grok_ai_copy_prompt() -> &'static str {
        match current_language() {
            Language::English => "Copy Prompt",
            Language::Chinese => "复制提示词",
        }
    }

    pub fn grok_ai_prompt_copied() -> &'static str {
        match current_language() {
            Language::English => "Prompt copied to clipboard!",
            Language::Chinese => "提示词已复制到剪贴板！",
        }
    }

    pub fn grok_ai_paste_json() -> &'static str {
        match current_language() {
            Language::English => "Paste JSON Response",
            Language::Chinese => "粘贴JSON响应",
        }
    }

    pub fn grok_ai_json_placeholder() -> &'static str {
        match current_language() {
            Language::English => "Paste the JSON response from LLM here...",
            Language::Chinese => "在此粘贴LLM返回的JSON响应...",
        }
    }

    pub fn grok_ai_apply_pattern() -> &'static str {
        match current_language() {
            Language::English => "Apply Pattern",
            Language::Chinese => "应用模板",
        }
    }

    pub fn grok_ai_pattern_applied() -> &'static str {
        match current_language() {
            Language::English => "Pattern applied successfully!",
            Language::Chinese => "模板应用成功！",
        }
    }

    pub fn grok_ai_invalid_json() -> &'static str {
        match current_language() {
            Language::English => "Invalid JSON format. Please check the response from LLM.",
            Language::Chinese => "无效的JSON格式。请检查LLM的响应。",
        }
    }

    pub fn grok_ai_no_file_open() -> &'static str {
        match current_language() {
            Language::English => "Please open a log file first",
            Language::Chinese => "请先打开一个日志文件",
        }
    }

    pub fn grok_ai_save_as_custom() -> &'static str {
        match current_language() {
            Language::English => "Save as Custom Pattern",
            Language::Chinese => "保存为自定义模板",
        }
    }

    pub fn agent_usage_title() -> &'static str {
        match current_language() {
            Language::English => "🔧 Remote Agent Usage",
            Language::Chinese => "🔧 远程 Agent 使用方式",
        }
    }

    pub fn agent_install_command() -> &'static str {
        match current_language() {
            Language::English => "Install:",
            Language::Chinese => "安装：",
        }
    }

    pub fn agent_basic_usage() -> &'static str {
        match current_language() {
            Language::English => "Basic usage:",
            Language::Chinese => "基本用法：",
        }
    }

    pub fn agent_server_address() -> &'static str {
        match current_language() {
            Language::English => "Default server port: 12500",
            Language::Chinese => "默认服务器端口：12500",
        }
    }

    pub fn local_network_addresses() -> &'static str {
        match current_language() {
            Language::English => "Local network addresses (for agent connection):",
            Language::Chinese => "本地网络地址（用于 Agent 连接）：",
        }
    }

    pub fn agent_more_info() -> &'static str {
        match current_language() {
            Language::English => "More info: github.com/zibo-chen/logline-agent",
            Language::Chinese => "更多信息：github.com/zibo-chen/logline-agent",
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
