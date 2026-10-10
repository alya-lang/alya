use super::token::TokenType;
use super::Lexer;

#[test]
fn test_tokenize_numbers() {
    let source = "42 3.75 0 100 .5";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(tokens[0].token_type, TokenType::Number(42));
    assert_eq!(tokens[1].token_type, TokenType::Float(3.75));
    assert_eq!(tokens[2].token_type, TokenType::Number(0));
    assert_eq!(tokens[3].token_type, TokenType::Number(100));
    assert_eq!(tokens[4].token_type, TokenType::Float(0.5));
    assert_eq!(tokens[5].token_type, TokenType::Eof);
}

#[test]
fn test_tokenize_keywords() {
    let source = "say let if else while for in function end return break continue";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    let expected = vec![
        TokenType::Say,
        TokenType::Let,
        TokenType::If,
        TokenType::Else,
        TokenType::While,
        TokenType::For,
        TokenType::In,
        TokenType::Function,
        TokenType::End,
        TokenType::Return,
        TokenType::Break,
        TokenType::Continue,
        TokenType::Eof,
    ];

    let actual: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_tokenize_identifiers() {
    let source = "my_var totalCount x1 _private";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(tokens[0].token_type, TokenType::Identifier("my_var".into()));
    assert_eq!(
        tokens[1].token_type,
        TokenType::Identifier("totalCount".into())
    );
    assert_eq!(tokens[2].token_type, TokenType::Identifier("x1".into()));
    assert_eq!(
        tokens[3].token_type,
        TokenType::Identifier("_private".into())
    );
}

#[test]
fn test_tokenize_operators() {
    let source = "+ - * / % == != < <= > >= and or not =";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    let expected = vec![
        TokenType::Plus,
        TokenType::Minus,
        TokenType::Multiply,
        TokenType::Divide,
        TokenType::Modulo,
        TokenType::Equal,
        TokenType::NotEqual,
        TokenType::Less,
        TokenType::LessEqual,
        TokenType::Greater,
        TokenType::GreaterEqual,
        TokenType::And,
        TokenType::Or,
        TokenType::Not,
        TokenType::Assign,
        TokenType::Eof,
    ];

    let actual: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_tokenize_strings_and_escapes() {
    let source = r#""Hello, World!" "Line1\nLine2\t\"quote\"" "\a\b\e\f\v\0""#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(
        tokens[0].token_type,
        TokenType::String("Hello, World!".into())
    );
    assert_eq!(
        tokens[1].token_type,
        TokenType::String("Line1\nLine2\t\"quote\"".into())
    );
    assert_eq!(
        tokens[2].token_type,
        TokenType::String("\x07\x08\x1b\x0c\x0b\0".into())
    );
}

#[test]
fn test_tokenize_comments() {
    let source = "# This is a comment\nsay 42 # inline comment\n";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::Newline,
            TokenType::Say,
            TokenType::Number(42),
            TokenType::Newline,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_unexpected_character() {
    let source = "say $bad";
    let mut lexer = Lexer::new(source);
    let result = lexer.tokenize();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unexpected character '$'"));
}

#[test]
fn test_tokenize_slash_and_multiline_comments() {
    let source = "// single line\nsay 10 /* inline multiline */ + 20\n";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::Newline,
            TokenType::Say,
            TokenType::Number(10),
            TokenType::Plus,
            TokenType::Number(20),
            TokenType::Newline,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_compound_and_logical_operators() {
    let source = "+= -= *= /= && || ! elif";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::PlusAssign,
            TokenType::MinusAssign,
            TokenType::MultiplyAssign,
            TokenType::DivideAssign,
            TokenType::And,
            TokenType::Or,
            TokenType::Not,
            TokenType::Elif,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_brackets() {
    let source = "[1, 2, 3] arr[0]";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::LeftBracket,
            TokenType::Number(1),
            TokenType::Comma,
            TokenType::Number(2),
            TokenType::Comma,
            TokenType::Number(3),
            TokenType::RightBracket,
            TokenType::Identifier("arr".into()),
            TokenType::LeftBracket,
            TokenType::Number(0),
            TokenType::RightBracket,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_struct() {
    let source = "struct Point\n  x: 10,\n  y: 20\nend";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");
    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::Struct,
            TokenType::Identifier("Point".into()),
            TokenType::Newline,
            TokenType::Identifier("x".into()),
            TokenType::Colon,
            TokenType::Number(10),
            TokenType::Comma,
            TokenType::Newline,
            TokenType::Identifier("y".into()),
            TokenType::Colon,
            TokenType::Number(20),
            TokenType::Newline,
            TokenType::End,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_as_and_colon_colon() {
    let source = "import \"math.alya\" as m\nm::calc()";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");
    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::Import,
            TokenType::String("math.alya".into()),
            TokenType::As,
            TokenType::Identifier("m".into()),
            TokenType::Newline,
            TokenType::Identifier("m".into()),
            TokenType::ColonColon,
            TokenType::Identifier("calc".into()),
            TokenType::LeftParen,
            TokenType::RightParen,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_null_nil_and_bitwise_operators() {
    let source = "null nil & | ^ ~ << >> &= |= ^= <<= >>=";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");
    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::Null,
            TokenType::Null,
            TokenType::BitAnd,
            TokenType::BitOr,
            TokenType::BitXor,
            TokenType::BitNot,
            TokenType::Shl,
            TokenType::Shr,
            TokenType::BitAndAssign,
            TokenType::BitOrAssign,
            TokenType::BitXorAssign,
            TokenType::ShlAssign,
            TokenType::ShrAssign,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_multiline_and_raw_strings() {
    let source = "\"\"\"\nHello\nWorld\n\"\"\"\n`raw string\nwith newlines`";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(
        tokens[0].token_type,
        TokenType::String("Hello\nWorld\n".into())
    );
    assert_eq!(tokens[1].token_type, TokenType::Newline);
    assert_eq!(
        tokens[2].token_type,
        TokenType::String("raw string\nwith newlines".into())
    );
    assert_eq!(tokens[3].token_type, TokenType::Eof);

    // Test with CRLF line endings to ensure normalization to LF
    let crlf_source = "\"\"\"\r\nHello\r\nWorld\r\n\"\"\"\r\n`raw\r\nstring`";
    let mut crlf_lexer = Lexer::new(crlf_source);
    let crlf_tokens = crlf_lexer.tokenize().expect("CRLF Tokenization failed");
    assert_eq!(
        crlf_tokens[0].token_type,
        TokenType::String("Hello\nWorld\n".into())
    );
    assert_eq!(crlf_tokens[1].token_type, TokenType::Newline);
    assert_eq!(
        crlf_tokens[2].token_type,
        TokenType::String("raw\nstring".into())
    );
}

#[test]
fn test_tokenize_question_and_null_coalesce() {
    let source = "? ?? ? ??";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");
    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();
    assert_eq!(
        types,
        vec![
            TokenType::Question,
            TokenType::NullCoalesce,
            TokenType::Question,
            TokenType::NullCoalesce,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_all_44_master_keywords() {
    let source = "let const function fn return defer if then elif else when is while for in repeat break continue struct enum interface try catch finally throw pub import as from extern spawn select assert test bench say and or not true false null self weak comptime sizeof alignof typeof end";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");
    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();

    assert_eq!(
        types,
        vec![
            TokenType::Let,
            TokenType::Const,
            TokenType::Function,
            TokenType::Function, // fn aliases to Function
            TokenType::Return,
            TokenType::Defer,
            TokenType::If,
            TokenType::Then,
            TokenType::Elif,
            TokenType::Else,
            TokenType::When,
            TokenType::Is,
            TokenType::While,
            TokenType::For,
            TokenType::In,
            TokenType::Repeat,
            TokenType::Break,
            TokenType::Continue,
            TokenType::Struct,
            TokenType::Enum,
            TokenType::Interface,
            TokenType::Try,
            TokenType::Catch,
            TokenType::Finally,
            TokenType::Throw,
            TokenType::Pub,
            TokenType::Import,
            TokenType::As,
            TokenType::From,
            TokenType::Extern,
            TokenType::Spawn,
            TokenType::Select,
            TokenType::Assert,
            TokenType::Test,
            TokenType::Bench,
            TokenType::Say,
            TokenType::And,
            TokenType::Or,
            TokenType::Not,
            TokenType::True,
            TokenType::False,
            TokenType::Null,
            TokenType::SelfKw,
            TokenType::Weak,
            TokenType::Comptime,
            TokenType::Sizeof,
            TokenType::Alignof,
            TokenType::Typeof,
            TokenType::End,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_runes_attributes_and_prefixes() {
    let source = "@inline @test 'M' '🚀' '\\n' b'A' f\"val: {x}\" r\"raw\\n\" b\"bytes\" ..= ..";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");
    let types: Vec<_> = tokens.into_iter().map(|t| t.token_type).collect();

    assert_eq!(
        types,
        vec![
            TokenType::At,
            TokenType::Identifier("inline".into()),
            TokenType::At,
            TokenType::Test,
            TokenType::Rune('M'),
            TokenType::Rune('🚀'),
            TokenType::Rune('\n'),
            TokenType::Number(65),
            TokenType::FormattedString("val: {x}".into()),
            TokenType::String("raw\\n".into()),
            TokenType::String("bytes".into()),
            TokenType::DotDotEqual,
            TokenType::DotDot,
            TokenType::Eof,
        ]
    );
}

#[test]
fn test_tokenize_integer_precision() {
    // Regression test for alya-lang/alya#49: integer literals above 2^53
    // must not round through f64.
    let source = "2305843009213693951 4611686018427387903 9223372036854775807 1_000_000 0xFF 0b101";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(tokens[0].token_type, TokenType::Number(2305843009213693951));
    assert_eq!(tokens[1].token_type, TokenType::Number(4611686018427387903));
    assert_eq!(tokens[2].token_type, TokenType::Number(9223372036854775807));
    assert_eq!(tokens[3].token_type, TokenType::Number(1000000));
    assert_eq!(tokens[4].token_type, TokenType::Number(255));
    assert_eq!(tokens[5].token_type, TokenType::Number(5));
}

#[test]
fn test_tokenize_u64_max_literal() {
    // The language supports u64 literals up to u64::MAX (spec syntax/types).
    let source = "9223372036854775808 18446744073709551615 0xFFFFFFFFFFFFFFFF";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(tokens[0].token_type, TokenType::Number(9223372036854775808));
    assert_eq!(
        tokens[1].token_type,
        TokenType::Number(18446744073709551615)
    );
    assert_eq!(
        tokens[2].token_type,
        TokenType::Number(18446744073709551615)
    );
}

#[test]
fn test_tokenize_integer_out_of_range() {
    for source in [
        "18446744073709551616",
        "0x10000000000000000",
        "0b11111111111111111111111111111111111111111111111111111111111111111",
    ] {
        let mut lexer = Lexer::new(source);
        let err = lexer
            .tokenize()
            .expect_err("expected out-of-range integer literal to fail");
        assert!(
            err.contains("out of range"),
            "unexpected error for '{}': {}",
            source,
            err
        );
    }
}

#[test]
fn test_tokenize_float_untouched() {
    let source = "2.71 1e3 0.5";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(tokens[0].token_type, TokenType::Float(2.71));
    assert_eq!(tokens[1].token_type, TokenType::Float(1000.0));
    assert_eq!(tokens[2].token_type, TokenType::Float(0.5));
}

#[test]
fn test_tokenize_string_kinds_f_only_formatted() {
    // alya-lang/alya#155: only f-strings carry holes; every other kind
    // is a plain String token even with braces inside.
    let source = "f\"{x}\" \"{x}\" r\"\\d{4}\"";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenization failed");

    assert_eq!(
        tokens[0].token_type,
        TokenType::FormattedString("{x}".into())
    );
    assert_eq!(tokens[1].token_type, TokenType::String("{x}".into()));
    assert_eq!(tokens[2].token_type, TokenType::String("\\d{4}".into()));
}
