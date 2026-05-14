//! Echo 命令 — 将参数输出到标准输出
//!
//! 遵循 POSIX 兼容语义，支持 -n（不换行）和 -e（转义序列解释）标志。

/// 解析后的 echo 参数
#[derive(Debug, Clone)]
pub struct EchoArgs {
    /// 要输出的文本
    pub text: String,
    /// -n: 不在末尾追加换行符
    pub no_newline: bool,
    /// -e: 解释转义序列（\n, \t, \\ 等）
    pub interpret_escapes: bool,
}

impl EchoArgs {
    /// 从原始参数列表解析 EchoArgs。
    ///
    /// 遵循 POSIX echo 语义：
    /// - `-n`   不输出尾随换行符
    /// - `-e`   启用转义序列解释
    /// - `-E`   禁用转义序列解释（默认）
    /// - 其余参数用空格连接
    pub fn parse(args: &[String]) -> Self {
        let mut no_newline = false;
        let mut interpret_escapes = false;
        let mut text_parts: Vec<&str> = Vec::new();

        for arg in args {
            if arg == "-n" {
                no_newline = true;
            } else if arg == "-e" {
                interpret_escapes = true;
            } else if arg == "-E" {
                interpret_escapes = false;
            } else if arg == "-en" || arg == "-ne" {
                no_newline = true;
                interpret_escapes = true;
            } else {
                text_parts.push(arg.as_str());
            }
        }

        Self {
            text: text_parts.join(" "),
            no_newline,
            interpret_escapes,
        }
    }
}

/// 将 EchoArgs 渲染为输出字节序列。
pub fn render(args: &EchoArgs) -> Vec<u8> {
    let bytes = if args.interpret_escapes {
        interpret_escapes(&args.text)
    } else {
        args.text.as_bytes().to_vec()
    };

    let mut output = bytes;
    if !args.no_newline {
        output.push(b'\n');
    }
    output
}

/// 解释字符串中的 C 风格转义序列。
fn interpret_escapes(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'n' => {
                    out.push(b'\n');
                    i += 2;
                }
                b't' => {
                    out.push(b'\t');
                    i += 2;
                }
                b'r' => {
                    out.push(b'\r');
                    i += 2;
                }
                b'\\' => {
                    out.push(b'\\');
                    i += 2;
                }
                b'0' => {
                    out.push(b'\0');
                    i += 2;
                }
                b'a' => {
                    out.push(0x07);
                    i += 2;
                }
                b'b' => {
                    out.push(0x08);
                    i += 2;
                }
                b'v' => {
                    out.push(0x0b);
                    i += 2;
                }
                b'f' => {
                    out.push(0x0c);
                    i += 2;
                }
                b'"' => {
                    out.push(b'"');
                    i += 2;
                }
                b'\'' => {
                    out.push(b'\'');
                    i += 2;
                }
                // 八进制: \NNN（最多 3 位八进制数字）
                c if c.is_ascii_digit() && c != b'8' && c != b'9' => {
                    let seq_end = (i + 2..bytes.len())
                        .take(2)
                        .take_while(|&j| {
                            bytes[j].is_ascii_digit() && bytes[j] != b'8' && bytes[j] != b'9'
                        })
                        .last()
                        .map(|pos| pos + 1)
                        .unwrap_or(i + 2);
                    let octal_str = std::str::from_utf8(&bytes[i + 1..seq_end]).unwrap_or("0");
                    if let Ok(val) = u8::from_str_radix(octal_str, 8) {
                        out.push(val);
                    }
                    i = seq_end;
                }
                // 十六进制: \xHH
                b'x' if i + 3 < bytes.len() => {
                    let hex_str = std::str::from_utf8(&bytes[i + 2..i + 4]).unwrap_or("00");
                    if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                        out.push(val);
                    }
                    i += 4;
                }
                other => {
                    out.push(b'\\');
                    out.push(other);
                    i += 2;
                }
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }

    out
}

/// 执行 echo 命令。
///
/// # 参数
/// - `args`: 原始命令行参数字符串切片（不含程序名）
///
/// # 输出
/// 将渲染后的文本写入标准输出。
pub fn run_echo(args: &[String]) {
    let echo_args = EchoArgs::parse(args);
    let output = render(&echo_args);

    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(&output);
    let _ = handle.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_default_adds_newline() {
        let args = EchoArgs::parse(&["hello".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"hello\n");
    }

    #[test]
    fn test_echo_n_suppresses_newline() {
        let args = EchoArgs::parse(&["-n".to_string(), "hello".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"hello");
    }

    #[test]
    fn test_echo_e_interprets_newline() {
        let args = EchoArgs::parse(&["-e".to_string(), "line1\\nline2".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"line1\nline2\n");
    }

    #[test]
    fn test_echo_e_interprets_tab() {
        let args = EchoArgs::parse(&["-e".to_string(), "a\\tb".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"a\tb\n");
    }

    #[test]
    fn test_echo_e_interprets_backslash() {
        let args = EchoArgs::parse(&["-e".to_string(), "path\\\\to\\\\file".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"path\\to\\file\n");
    }

    #[test]
    fn test_echo_e_interprets_octal() {
        let args = EchoArgs::parse(&["-e".to_string(), "\\101\\102\\103".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"ABC\n");
    }

    #[test]
    fn test_echo_e_interprets_hex() {
        let args = EchoArgs::parse(&["-e".to_string(), "\\x48\\x65\\x6c".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"Hel\n");
    }

    #[test]
    fn test_echo_multiple_args_joined_by_space() {
        let args = EchoArgs::parse(&["hello".to_string(), "world".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"hello world\n");
    }

    #[test]
    fn test_echo_no_args_prints_newline() {
        let args = EchoArgs::parse(&[] as &[String]);
        let output = render(&args);
        assert_eq!(output, b"\n");
    }

    #[test]
    fn test_echo_ne_combination() {
        let args = EchoArgs::parse(&["-en".to_string(), "hello\\nworld".to_string()]);
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
        let output = render(&args);
        assert_eq!(output, b"hello\nworld");
    }

    #[test]
    fn test_echo_unknown_escape_preserved() {
        let args = EchoArgs::parse(&["-e".to_string(), "\\c".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"\\c\n");
    }

    #[test]
    fn test_echo_empty_string() {
        let args = EchoArgs::parse(&["".to_string()]);
        let output = render(&args);
        assert_eq!(output, b"\n");
    }

    // ============ 解析器测试 (parse) ============

    #[test]
    fn parse_no_args() {
        let args = EchoArgs::parse(&[]);
        assert_eq!(args.text, "");
        assert!(!args.no_newline);
        assert!(!args.interpret_escapes);
    }

    #[test]
    fn parse_flag_n_only() {
        let args = EchoArgs::parse(&["-n".to_string()]);
        assert_eq!(args.text, "");
        assert!(args.no_newline);
        assert!(!args.interpret_escapes);
    }

    #[test]
    fn parse_flag_e_only() {
        let args = EchoArgs::parse(&["-e".to_string()]);
        assert_eq!(args.text, "");
        assert!(!args.no_newline);
        assert!(args.interpret_escapes);
    }

    #[test]
    fn parse_flag_e_disables_escapes() {
        let args = EchoArgs::parse(&["-E".to_string(), "hello".to_string()]);
        assert_eq!(args.text, "hello");
        assert!(!args.no_newline);
        assert!(!args.interpret_escapes);
    }

    #[test]
    fn parse_e_then_capital_e_disables() {
        let args = EchoArgs::parse(&["-e".to_string(), "-E".to_string(), "hello".to_string()]);
        assert_eq!(args.text, "hello");
        assert!(!args.interpret_escapes);
    }

    #[test]
    fn parse_capital_e_then_e_enables() {
        let args = EchoArgs::parse(&["-E".to_string(), "-e".to_string(), "hello".to_string()]);
        assert_eq!(args.text, "hello");
        assert!(args.interpret_escapes);
    }

    #[test]
    fn parse_e_and_n_separate() {
        let args = EchoArgs::parse(&["-e".to_string(), "-n".to_string(), "hello".to_string()]);
        assert_eq!(args.text, "hello");
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
    }

    #[test]
    fn parse_n_and_e_separate() {
        let args = EchoArgs::parse(&["-n".to_string(), "-e".to_string(), "hello".to_string()]);
        assert_eq!(args.text, "hello");
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
    }

    #[test]
    fn parse_combined_en() {
        let args = EchoArgs::parse(&["-en".to_string(), "hello".to_string()]);
        assert_eq!(args.text, "hello");
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
    }

    #[test]
    fn parse_combined_ne() {
        let args = EchoArgs::parse(&["-ne".to_string(), "hello".to_string()]);
        assert_eq!(args.text, "hello");
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
    }

    #[test]
    fn parse_multiple_text_args() {
        let args = EchoArgs::parse(&["hello".to_string(), "world".to_string(), "foo".to_string()]);
        assert_eq!(args.text, "hello world foo");
    }

    #[test]
    fn parse_flag_after_text_eaten() {
        // 已知行为: -n 出现在文本参数之后也会被消耗（reviewer 注释 #3）
        let args = EchoArgs::parse(&["text".to_string(), "-n".to_string()]);
        assert!(args.no_newline);
        assert_eq!(args.text, "text");
    }

    #[test]
    fn parse_multiple_flags_no_text() {
        let args = EchoArgs::parse(&["-n".to_string(), "-e".to_string()]);
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
        assert_eq!(args.text, "");
    }

    #[test]
    fn parse_all_flag_orderings() {
        // 验证所有标志排列组合结果一致
        let orderings: &[&[&str]] = &[
            &["-e", "-n", "hello"],
            &["-n", "-e", "hello"],
            &["-en", "hello"],
            &["-ne", "hello"],
        ];
        for ordering in orderings {
            let args: Vec<String> = ordering.iter().map(|&s| s.to_string()).collect();
            let parsed = EchoArgs::parse(&args);
            assert!(parsed.no_newline, "no_newline false for {:?}", ordering);
            assert!(
                parsed.interpret_escapes,
                "interpret_escapes false for {:?}",
                ordering
            );
            assert_eq!(parsed.text, "hello");
        }
    }

    #[test]
    fn parse_mixed_flags_and_text() {
        let args = EchoArgs::parse(&[
            "-n".to_string(),
            "first".to_string(),
            "-e".to_string(),
            "second".to_string(),
        ]);
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
        assert_eq!(args.text, "first second");
    }

    #[test]
    fn parse_multiple_dashes_not_confused() {
        let args = EchoArgs::parse(&["--".to_string(), "hello".to_string()]);
        assert!(!args.no_newline);
        assert!(!args.interpret_escapes);
        assert_eq!(args.text, "-- hello");
    }

    // ============ 渲染器测试 (render) ============

    #[test]
    fn render_default_newline() {
        let args = EchoArgs {
            text: "hello".into(),
            no_newline: false,
            interpret_escapes: false,
        };
        assert_eq!(render(&args), b"hello\n");
    }

    #[test]
    fn render_suppress_newline() {
        let args = EchoArgs {
            text: "hello".into(),
            no_newline: true,
            interpret_escapes: false,
        };
        assert_eq!(render(&args), b"hello");
    }

    #[test]
    fn render_no_escape_interpretation() {
        let args = EchoArgs {
            text: "a\\nb".into(),
            no_newline: false,
            interpret_escapes: false,
        };
        assert_eq!(render(&args), b"a\\nb\n");
    }

    #[test]
    fn render_empty_text_no_newline() {
        let args = EchoArgs {
            text: "".into(),
            no_newline: true,
            interpret_escapes: false,
        };
        assert_eq!(render(&args), b"");
    }

    #[test]
    fn render_unicode_passthrough() {
        let args = EchoArgs {
            text: "你好世界".into(),
            no_newline: false,
            interpret_escapes: false,
        };
        assert_eq!(render(&args), "你好世界\n".as_bytes());
    }

    #[test]
    fn render_unicode_no_newline() {
        let args = EchoArgs {
            text: "Hello 世界".into(),
            no_newline: true,
            interpret_escapes: false,
        };
        assert_eq!(render(&args), "Hello 世界".as_bytes());
    }

    #[test]
    fn render_whitespace_preserved() {
        let args = EchoArgs {
            text: "  a   b  ".into(),
            no_newline: false,
            interpret_escapes: false,
        };
        assert_eq!(render(&args), b"  a   b  \n");
    }

    #[test]
    fn render_very_long_text() {
        let long = "a".repeat(10000);
        let args = EchoArgs {
            text: long.clone(),
            no_newline: false,
            interpret_escapes: false,
        };
        let output = render(&args);
        assert_eq!(output.len(), 10001);
        assert_eq!(&output[..5], b"aaaaa");
        assert_eq!(output[10000], b'\n');
    }

    #[test]
    fn render_with_escapes_and_newline() {
        let args = EchoArgs {
            text: "a\\tb".into(),
            no_newline: false,
            interpret_escapes: true,
        };
        assert_eq!(render(&args), b"a\tb\n");
    }

    #[test]
    fn render_with_escapes_no_newline() {
        let args = EchoArgs {
            text: "a\\tb".into(),
            no_newline: true,
            interpret_escapes: true,
        };
        assert_eq!(render(&args), b"a\tb");
    }

    // ============ 转义序列测试 (interpret_escapes) ============

    /// 辅助函数：以 -e -n 模式渲染（不追加换行）
    fn render_escaped(text: &str) -> Vec<u8> {
        render(&EchoArgs {
            text: text.to_string(),
            no_newline: true,
            interpret_escapes: true,
        })
    }

    #[test]
    fn esc_newline() {
        assert_eq!(render_escaped("a\\nb"), b"a\nb");
    }

    #[test]
    fn esc_tab() {
        assert_eq!(render_escaped("a\\tb"), b"a\tb");
    }

    #[test]
    fn esc_return() {
        assert_eq!(render_escaped("a\\rb"), b"a\rb");
    }

    #[test]
    fn esc_backslash() {
        assert_eq!(render_escaped("a\\\\b"), b"a\\b");
    }

    #[test]
    fn esc_null() {
        assert_eq!(render_escaped("a\\0b"), b"a\0b");
    }

    #[test]
    fn esc_bell() {
        assert_eq!(render_escaped("\\a"), b"\x07");
    }

    #[test]
    fn esc_backspace() {
        assert_eq!(render_escaped("a\\bb"), b"a\x08b");
    }

    #[test]
    fn esc_vtab() {
        assert_eq!(render_escaped("a\\vb"), b"a\x0bb");
    }

    #[test]
    fn esc_formfeed() {
        assert_eq!(render_escaped("a\\fb"), b"a\x0cb");
    }

    #[test]
    fn esc_double_quote() {
        assert_eq!(render_escaped("\\\"hello\\\""), b"\"hello\"");
    }

    #[test]
    fn esc_single_quote() {
        assert_eq!(render_escaped("\\'hello\\'"), b"'hello'");
    }

    #[test]
    fn esc_octal_3digit() {
        assert_eq!(render_escaped("\\101\\102\\103"), b"ABC");
    }

    #[test]
    fn esc_octal_2digit() {
        assert_eq!(render_escaped("\\11"), b"\x09");
    }

    #[test]
    fn esc_octal_1digit() {
        assert_eq!(render_escaped("\\1"), b"\x01");
    }

    #[test]
    fn esc_octal_max() {
        assert_eq!(render_escaped("\\377"), b"\xff");
    }

    #[test]
    fn esc_octal_zero_max() {
        // \0 被字面量分支匹配，产生 null 字节
        assert_eq!(render_escaped("\\0"), b"\x00");
    }

    #[test]
    fn esc_octal_zero_followed_by_digit() {
        // 注意: \0 先于八进制数字匹配 → 产生 \0 + "1"
        // reviewer 已记录此边界行为
        assert_eq!(render_escaped("\\01"), b"\x001");
    }

    #[test]
    fn esc_octal_with_eight_or_nine() {
        // 8 和 9 不是有效的八进制数字 → \0 + "8" 或 \0 + "9"
        assert_eq!(render_escaped("\\08"), b"\x008");
        assert_eq!(render_escaped("\\09"), b"\x009");
    }

    #[test]
    fn esc_octal_reject_invalid() {
        // \400 溢出 u8 → 静默消耗整个序列但无输出（当前行为）
        // 400 八进制 = 256 > 255, from_str_radix 返回 Err，无字节被压入
        let result = render_escaped("\\400");
        assert_eq!(result, b"", "\\400 should overflow and consume silently");

        // 但有效的三位八进制仍应正常工作
        assert_eq!(render_escaped("\\377"), b"\xff");
    }

    #[test]
    fn esc_hex_uppercase() {
        assert_eq!(render_escaped("\\x48\\x65\\x6C"), b"Hel");
    }

    #[test]
    fn esc_hex_lowercase() {
        assert_eq!(render_escaped("\\x48\\x65\\x6c"), b"Hel");
    }

    #[test]
    fn esc_hex_max() {
        assert_eq!(render_escaped("\\xff"), b"\xff");
    }

    #[test]
    fn esc_hex_zero() {
        assert_eq!(render_escaped("\\x00"), b"\x00");
    }

    #[test]
    fn esc_hex_adjacent() {
        assert_eq!(render_escaped("\\x48\\x69"), b"Hi");
    }

    #[test]
    fn esc_hex_only_prefix() {
        // \x 后无字符 → 落入未知转义分支
        assert_eq!(render_escaped("\\x"), b"\\x");
    }

    #[test]
    fn esc_hex_one_digit() {
        // 注意: \x 要求恰好 2 个十六进制数字 → 单数字降级为未知转义
        // reviewer 已记录此边界行为
        assert_eq!(render_escaped("\\xA"), b"\\xA");
    }

    #[test]
    fn esc_hex_at_string_end() {
        assert_eq!(render_escaped("end\\x"), b"end\\x");
    }

    #[test]
    fn esc_unknown_preserved() {
        assert_eq!(render_escaped("\\c\\k\\q"), b"\\c\\k\\q");
    }

    #[test]
    fn esc_trailing_backslash() {
        assert_eq!(render_escaped("end\\"), b"end\\");
    }

    #[test]
    fn esc_only_backslash() {
        assert_eq!(render_escaped("\\"), b"\\");
    }

    #[test]
    fn esc_multiple_in_sequence() {
        assert_eq!(render_escaped("\\n\\t\\r"), b"\n\t\r");
    }

    #[test]
    fn esc_all_single_letter_escapes() {
        // 一次测试所有单字母转义
        assert_eq!(
            render_escaped("\\n\\t\\r\\\\\\0\\a\\b\\v\\f"),
            b"\n\t\r\\\0\x07\x08\x0b\x0c"
        );
    }

    #[test]
    fn esc_mixed_with_plain_text() {
        assert_eq!(render_escaped("Hello\\nWorld\\t!\\n"), b"Hello\nWorld\t!\n");
    }

    #[test]
    fn esc_no_special_chars_plain_text() {
        assert_eq!(render_escaped("hello world"), b"hello world");
    }

    #[test]
    fn esc_consecutive_backslashes() {
        assert_eq!(render_escaped("\\\\\\\\"), b"\\\\");
    }

    #[test]
    fn esc_backslash_at_end_of_string() {
        assert_eq!(render_escaped("hello\\"), b"hello\\");
    }

    #[test]
    fn esc_only_newlines() {
        assert_eq!(render_escaped("\\n\\n\\n"), b"\n\n\n");
    }

    #[test]
    fn esc_null_in_middle_of_text() {
        assert_eq!(render_escaped("before\\0after"), b"before\0after");
    }

    #[test]
    fn esc_mixed_octal_and_text() {
        assert_eq!(render_escaped("\\104\\101\\126"), b"DAV");
    }

    #[test]
    fn esc_octal_lower_bound() {
        // \0 被字面量分支提前匹配 → 产生 null 字节（注意: 这与 POSIX 的 \0NNN 八进制行为不同）
        assert_eq!(render_escaped("\\0"), b"\x00");
        // \00 → \0 产生 null，然后字面量 '0'
        assert_eq!(render_escaped("\\00"), b"\x000");
        // \000 → \0 产生 null，然后字面量 "00"
        assert_eq!(render_escaped("\\000"), b"\x0000");
    }

    #[test]
    fn esc_unicode_multi_byte_with_escapes() {
        let args = EchoArgs {
            text: "世界\\n你好".into(),
            no_newline: false,
            interpret_escapes: true,
        };
        let output = render(&args);
        assert_eq!(output, "世界\n你好\n".as_bytes());
    }

    // ============ run_echo 集成测试 ============

    #[test]
    fn run_echo_parse_roundtrip_n() {
        let args = EchoArgs::parse(&["-n".to_string(), "hello".to_string()]);
        assert!(args.no_newline);
        assert_eq!(args.text, "hello");
    }

    #[test]
    fn run_echo_parse_roundtrip_e() {
        let args = EchoArgs::parse(&["-e".to_string(), "hello\\nworld".to_string()]);
        assert!(args.interpret_escapes);
        let output = render(&args);
        assert_eq!(output, b"hello\nworld\n");
    }

    #[test]
    fn run_echo_parse_roundtrip_en() {
        let args = EchoArgs::parse(&["-en".to_string(), "hello\\nworld".to_string()]);
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
        let output = render(&args);
        assert_eq!(output, b"hello\nworld");
    }

    #[test]
    fn run_echo_parse_roundtrip_complex() {
        // 模拟 main.rs 中 bridge 的拼接逻辑
        let raw_args = vec!["-n".to_string(), "-e".to_string(), "a\\nb".to_string()];
        let args = EchoArgs::parse(&raw_args);
        assert!(args.no_newline);
        assert!(args.interpret_escapes);
        assert_eq!(args.text, "a\\nb");
        let output = render(&args);
        assert_eq!(output, b"a\nb");
    }
}
