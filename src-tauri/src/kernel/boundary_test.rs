//! Kernel 边界守护测试
//!
//! 验证 kernel/ 下的 Rust 源码不会反向依赖业务层模块，
//! 也不会包含业务域词汇（除注释和字符串字面量外）。
//!
//! 运行方式：
//!   cargo test kernel::boundary -- --nocapture
//!
//! 本文件是自动 CI 门禁的一部分，任何违反边界约定的代码变更都会
//! 在 PR 阶段被阻止合并。

use std::fs;

/// 递归收集 kernel/ 目录下所有 .rs 文件路径
fn collect_kernel_rs_files() -> Vec<String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let kernel_dir = format!("{}/src/kernel", manifest_dir);

    let mut files = Vec::new();
    collect_rs_files_recursive(&kernel_dir, &mut files);
    files
}

fn collect_rs_files_recursive(dir: &str, files: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files_recursive(path.to_str().unwrap_or(""), files);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            if let Some(path_str) = path.to_str() {
                files.push(path_str.to_string());
            }
        }
    }
}

/// 检查一行是否为注释（支持行内注释，会截取注释前的部分）
/// 返回 (是否为纯注释行, 去除注释后的代码部分)
fn strip_comment(line: &str) -> (bool, String) {
    let trimmed = line.trim();

    // 纯注释行
    if trimmed.starts_with("//") {
        return (true, String::new());
    }

    // 纯属性行（#[...]）
    if trimmed.starts_with('#') && !trimmed.contains("use ") {
        return (false, line.to_string());
    }

    // 行内注释：截取 // 前的部分（注意排除字符串内的 //）
    let mut in_string = false;
    let mut chars = line.chars().enumerate();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '"' if !in_string => {
                in_string = true;
            }
            '"' if in_string => {
                in_string = false;
            }
            '/' if !in_string => {
                if let Some((_, '/')) = chars.next() {
                    return (false, line[..i].to_string());
                }
            }
            _ => {}
        }
    }

    (false, line.to_string())
}

/// 检查内容是否在多行注释 /* ... */ 内（未使用，保留供未来扩展）
#[allow(dead_code)]
fn is_inside_block_comment(_open_before: usize) -> bool {
    _open_before % 2 == 1
}

/// 计算一行中 /* 和 */ 的出现次数
fn count_block_comment_markers(line: &str) -> (usize, usize) {
    let mut open = 0;
    let mut close = 0;
    let mut in_string = false;
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        match chars[i] {
            '"' if !in_string => in_string = true,
            '"' if in_string => in_string = false,
            '/' if !in_string && i + 1 < len && chars[i + 1] == '*' => {
                open += 1;
                i += 2;
                continue;
            }
            '*' if !in_string && i + 1 < len && chars[i + 1] == '/' => {
                close += 1;
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    (open, close)
}

/// 检查一行中是否包含完整单词形式的 forbidden 关键词
fn contains_business_keyword(line: &str) -> Option<&'static str> {
    // `extension` 是 Navis/Cordis 的通用运行时边界术语，不代表具体产品业务；
    // crate::extension 反向依赖仍由导入边界测试单独禁止。
    let forbidden = [
        "tool",
        "session",
        "agent",
        "provider",
        "mcp",
        "permission",
        "thinking",
        "terminal",
        "gateway",
        "sandbox",
    ];

    // 分词：按非字母数字字符分割
    let mut start = 0;
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();

    while start < len {
        // 找到单词开头（字母字符）
        while start < len && !chars[start].is_ascii_alphabetic() {
            start += 1;
        }
        if start >= len {
            break;
        }

        // 找到单词结尾
        let mut end = start;
        while end < len && chars[end].is_ascii_alphanumeric() {
            end += 1;
        }

        let word: String = chars[start..end].iter().collect();
        let word_lower = word.to_lowercase();

        for &kw in &forbidden {
            if word_lower == kw {
                return Some(kw);
            }
        }

        start = end;
    }

    None
}

// ============================================================================
// E1: 导入边界测试
// ============================================================================

/// kernel 模块禁止导入以下业务层模块路径（反向依赖检测）。
///
/// 任何 kernel/ 下的 .rs 文件不得包含以下 `use` 语句：
/// - `use crate::agent`
/// - `use crate::gateway`
/// - `use crate::mcp`
/// - `use crate::extension`
/// - `use crate::ui`
/// - `use crate::session`
/// - `use crate::terminal`
/// - `use crate::file`
/// - `use crate::tool`
/// - `use crate::extension`
/// - `use crate::project`
/// - `use crate::ai`
/// - `use crate::security`
/// - `use crate::foundation`
#[test]
fn kernel_does_not_import_business_modules() {
    let files = collect_kernel_rs_files();
    assert!(
        !files.is_empty(),
        "未找到 kernel/ 下的 .rs 文件，路径配置可能有误"
    );

    let forbidden = [
        "use crate::agent",
        "use crate::gateway",
        "use crate::mcp",
        "use crate::extension",
        "use crate::ui",
        "use crate::session",
        "use crate::terminal",
        "use crate::file",
        "use crate::tool",
        "use crate::extension",
        "use crate::project",
        "use crate::ai",
        "use crate::security",
        "use crate::foundation",
    ];

    let mut violations = Vec::new();

    for file_path in &files {
        // 跳过本测试文件自身
        if file_path.ends_with("boundary_test.rs") {
            continue;
        }
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // 跳过纯注释和测试模块
            if trimmed.starts_with("//") || trimmed.starts_with("#[cfg(test)]") {
                continue;
            }

            for pattern in &forbidden {
                if trimmed.contains(pattern) {
                    violations.push(format!(
                        "文件 {} 第 {} 行包含禁止导入: {}",
                        file_path,
                        line_num + 1,
                        pattern
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!(
            "kernel 边界违规：发现 {} 处反向依赖：\n{}",
            violations.len(),
            violations.join("\n")
        );
    }
}

// ============================================================================
// E2: 业务词汇测试
// ============================================================================

/// kernel 模块禁止在非注释、非字符串字面量的代码中使用业务域词汇。
///
/// 这些词汇属于业务层，不应出现在 kernel 的抽象原语中。
/// 误报会自动记录，供开发者确认是否为合理用法后添加到白名单。
///
/// 禁止词汇（全词匹配）：
/// tool, session, agent, provider, mcp, permission, thinking, terminal,
/// gateway, sandbox。`extension` 是 Cordis 的通用运行时术语，不属于业务词汇。
#[test]
fn kernel_does_not_contain_business_vocabulary() {
    let files = collect_kernel_rs_files();
    assert!(
        !files.is_empty(),
        "未找到 kernel/ 下的 .rs 文件，路径配置可能有误"
    );

    let mut violations = Vec::new();
    let mut block_open_count: usize = 0; // 计数 /* 未闭合数

    for file_path in &files {
        // 跳过本测试文件自身
        if file_path.ends_with("boundary_test.rs") {
            continue;
        }
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;

            // 更新块注释状态
            let (opens, closes) = count_block_comment_markers(line);
            let was_in_comment = block_open_count % 2 == 1;
            block_open_count = block_open_count + opens - closes;

            // 块注释内的行（进入前就已在注释内）跳过
            if was_in_comment {
                continue;
            }

            let (is_comment, code_part) = strip_comment(line);

            // 纯注释行跳过
            if is_comment {
                continue;
            }

            // 去除字符串字面量（双引号内容）
            let mut cleaned = String::new();
            let mut in_string = false;
            let mut chars_iter = code_part.chars().peekable();
            while let Some(ch) = chars_iter.next() {
                match ch {
                    '\\' if in_string => {
                        // 转义字符，跳过下一个字符
                        cleaned.push(ch);
                        if let Some(next) = chars_iter.next() {
                            cleaned.push(next);
                        }
                    }
                    '"' => {
                        in_string = !in_string;
                    }
                    _ if !in_string => {
                        cleaned.push(ch);
                    }
                    _ => {}
                }
            }

            // 对清理后的代码部分检查禁止词汇（全词匹配）
            if let Some(keyword) = contains_business_keyword(&cleaned) {
                violations.push(format!(
                    "{}:{}: 包含禁止词汇 \"{}\" — 原始行: {}",
                    file_path,
                    line_num,
                    keyword,
                    line.trim()
                ));
            }
        }
    }

    if !violations.is_empty() {
        panic!(
            "kernel 业务词汇违规：发现 {} 处禁止用法：\n\n{}\n\n\
             若确需使用，请在设计文档 design/kernel.md §2.7 白名单中补充说明。",
            violations.len(),
            violations.join("\n")
        );
    }
}
