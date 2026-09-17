#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    Say,      // say (like print)
    Let,      // let (variable declaration)
    If,       // if
    Else,     // else
    Elif,     // elif
    While,    // while
    For,      // for
    In,       // in
    Function, // function
    End,      // end
    Return,   // return
    When,     // when (pattern matching)
    Is,       // is
    Then,     // then
    Repeat,   // repeat (infinite loop)
    Break,    // break
    Continue, // continue
    Ask,      // ask (input)
    Try,      // try
    Catch,    // catch
    Finally,  // finally
    Throw,    // throw
    Import,   // import
    As,       // as
    Struct,   // struct
    Enum,     // enum
    Const,    // const
    Extern,   // extern
    From,     // from
    Defer,    // defer
    Pub,      // pub
    Interface,// interface
    Spawn,    // spawn
    Select,   // select
    Assert,   // assert
    Test,     // test
    Bench,    // bench
    SelfKw,   // self
    Weak,     // weak
    Comptime, // comptime
    Sizeof,   // sizeof
    Alignof,  // alignof
    Typeof,   // typeof
    At,       // @

    // Literals
    Number(f64),
    Float(f64),
    String(String),
    Rune(char),
    Identifier(String),
    True,
    False,
    Null,

    // Operators
    Plus,           // +
    Minus,          // -
    Multiply,       // *
    Divide,         // /
    Modulo,         // %
    Arrow,          // ->
    FatArrow,       // =>
    Assign,         // =
    PlusAssign,     // +=
    MinusAssign,    // -=
    MultiplyAssign, // *=
    DivideAssign,   // /=
    ModuloAssign,   // %=
    BitAndAssign,   // &=
    BitOrAssign,    // |=
    BitXorAssign,   // ^=
    ShlAssign,      // <<=
    ShrAssign,      // >>=
    Equal,          // ==
    NotEqual,       // !=
    Less,           // <
    Greater,        // >
    LessEqual,      // <=
    GreaterEqual,   // >=
    And,            // and, &&
    Or,             // or, ||
    Not,            // not, !
    BitAnd,         // &
    BitOr,          // |
    BitXor,         // ^
    BitNot,         // ~
    Shl,            // <<
    Shr,            // >>

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    LeftBrace,    // {
    RightBrace,   // }
    Comma,        // ,
    Colon,        // :
    ColonColon,   // ::
    Question,     // ?
    QuestionDot,  // ?.
    NullCoalesce, // ??
    Dot,          // .
    DotDot,       // ..
    DotDotEqual,  // ..=
    DotDotDot,    // ...
    Newline,      // \n

    // Special
    Eof,
}

impl TokenType {
    pub fn from_identifier(ident: &str) -> Self {
        match ident {
            "say" => TokenType::Say,
            "let" => TokenType::Let,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "elif" => TokenType::Elif,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "in" => TokenType::In,
            "function" | "fn" => TokenType::Function,
            "end" => TokenType::End,
            "return" => TokenType::Return,
            "when" => TokenType::When,
            "is" => TokenType::Is,
            "then" => TokenType::Then,
            "repeat" => TokenType::Repeat,
            "break" => TokenType::Break,
            "continue" => TokenType::Continue,
            "ask" => TokenType::Ask,
            "try" => TokenType::Try,
            "catch" => TokenType::Catch,
            "finally" => TokenType::Finally,
            "throw" => TokenType::Throw,
            "import" => TokenType::Import,
            "as" => TokenType::As,
            "struct" => TokenType::Struct,
            "enum" => TokenType::Enum,
            "const" => TokenType::Const,
            "extern" => TokenType::Extern,
            "from" => TokenType::From,
            "defer" => TokenType::Defer,
            "pub" => TokenType::Pub,
            "interface" => TokenType::Interface,
            "spawn" => TokenType::Spawn,
            "select" => TokenType::Select,
            "assert" => TokenType::Assert,
            "test" => TokenType::Test,
            "bench" => TokenType::Bench,
            "self" => TokenType::SelfKw,
            "weak" => TokenType::Weak,
            "comptime" => TokenType::Comptime,
            "sizeof" => TokenType::Sizeof,
            "alignof" => TokenType::Alignof,
            "typeof" => TokenType::Typeof,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "null" | "nil" => TokenType::Null,
            "and" => TokenType::And,
            "or" => TokenType::Or,
            "not" => TokenType::Not,
            _ => TokenType::Identifier(ident.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenType::Say => write!(f, "'say'"),
            TokenType::Let => write!(f, "'let'"),
            TokenType::If => write!(f, "'if'"),
            TokenType::Else => write!(f, "'else'"),
            TokenType::Elif => write!(f, "'elif'"),
            TokenType::While => write!(f, "'while'"),
            TokenType::For => write!(f, "'for'"),
            TokenType::In => write!(f, "'in'"),
            TokenType::Function => write!(f, "'function'"),
            TokenType::End => write!(f, "'end'"),
            TokenType::Return => write!(f, "'return'"),
            TokenType::When => write!(f, "'when'"),
            TokenType::Is => write!(f, "'is'"),
            TokenType::Then => write!(f, "'then'"),
            TokenType::Repeat => write!(f, "'repeat'"),
            TokenType::Break => write!(f, "'break'"),
            TokenType::Continue => write!(f, "'continue'"),
            TokenType::Ask => write!(f, "'ask'"),
            TokenType::Try => write!(f, "'try'"),
            TokenType::Catch => write!(f, "'catch'"),
            TokenType::Finally => write!(f, "'finally'"),
            TokenType::Throw => write!(f, "'throw'"),
            TokenType::Import => write!(f, "'import'"),
            TokenType::As => write!(f, "'as'"),
            TokenType::Struct => write!(f, "'struct'"),
            TokenType::Enum => write!(f, "'enum'"),
            TokenType::Const => write!(f, "'const'"),
            TokenType::Extern => write!(f, "'extern'"),
            TokenType::From => write!(f, "'from'"),
            TokenType::Defer => write!(f, "'defer'"),
            TokenType::Pub => write!(f, "'pub'"),
            TokenType::Interface => write!(f, "'interface'"),
            TokenType::Spawn => write!(f, "'spawn'"),
            TokenType::Select => write!(f, "'select'"),
            TokenType::Assert => write!(f, "'assert'"),
            TokenType::Test => write!(f, "'test'"),
            TokenType::Bench => write!(f, "'bench'"),
            TokenType::SelfKw => write!(f, "'self'"),
            TokenType::Weak => write!(f, "'weak'"),
            TokenType::Comptime => write!(f, "'comptime'"),
            TokenType::Sizeof => write!(f, "'sizeof'"),
            TokenType::Alignof => write!(f, "'alignof'"),
            TokenType::Typeof => write!(f, "'typeof'"),
            TokenType::At => write!(f, "'@'"),
            TokenType::Number(n) => write!(f, "number '{}'", n),
            TokenType::Float(n) => write!(f, "float '{}'", n),
            TokenType::String(s) => write!(f, "\"{}\"", s),
            TokenType::Rune(c) => write!(f, "'{}'", c),
            TokenType::Identifier(s) => write!(f, "identifier '{}'", s),
            TokenType::True => write!(f, "'true'"),
            TokenType::False => write!(f, "'false'"),
            TokenType::Null => write!(f, "'null'"),
            TokenType::Plus => write!(f, "'+'"),
            TokenType::Minus => write!(f, "'-'"),
            TokenType::Multiply => write!(f, "'*'"),
            TokenType::Divide => write!(f, "'/'"),
            TokenType::Modulo => write!(f, "'%'"),
            TokenType::Arrow => write!(f, "'->'"),
            TokenType::FatArrow => write!(f, "'=>'"),
            TokenType::Assign => write!(f, "'='"),
            TokenType::PlusAssign => write!(f, "'+='"),
            TokenType::MinusAssign => write!(f, "'-='"),
            TokenType::MultiplyAssign => write!(f, "'*='"),
            TokenType::DivideAssign => write!(f, "'/='"),
            TokenType::ModuloAssign => write!(f, "'%='"),
            TokenType::BitAndAssign => write!(f, "'&='"),
            TokenType::BitOrAssign => write!(f, "'|='"),
            TokenType::BitXorAssign => write!(f, "'^='"),
            TokenType::ShlAssign => write!(f, "'<<='"),
            TokenType::ShrAssign => write!(f, "'>>='"),
            TokenType::Equal => write!(f, "'=='"),
            TokenType::NotEqual => write!(f, "'!='"),
            TokenType::Less => write!(f, "'<'"),
            TokenType::Greater => write!(f, "'>'"),
            TokenType::LessEqual => write!(f, "'<='"),
            TokenType::GreaterEqual => write!(f, "'>='"),
            TokenType::And => write!(f, "'and'"),
            TokenType::Or => write!(f, "'or'"),
            TokenType::Not => write!(f, "'not'"),
            TokenType::BitAnd => write!(f, "'&'"),
            TokenType::BitOr => write!(f, "'|'"),
            TokenType::BitXor => write!(f, "'^'"),
            TokenType::BitNot => write!(f, "'~'"),
            TokenType::Shl => write!(f, "'<<'"),
            TokenType::Shr => write!(f, "'>>'"),
            TokenType::LeftParen => write!(f, "'('"),
            TokenType::RightParen => write!(f, "')'"),
            TokenType::LeftBracket => write!(f, "'['"),
            TokenType::RightBracket => write!(f, "']'"),
            TokenType::LeftBrace => write!(f, "'{{'"),
            TokenType::RightBrace => write!(f, "'}}'"),
            TokenType::Comma => write!(f, "','"),
            TokenType::Colon => write!(f, "':'"),
            TokenType::ColonColon => write!(f, "'::'"),
            TokenType::Question => write!(f, "'?'"),
            TokenType::QuestionDot => write!(f, "'?.'"),
            TokenType::NullCoalesce => write!(f, "'??'"),
            TokenType::Dot => write!(f, "'.'"),
            TokenType::DotDot => write!(f, "'..'"),
            TokenType::DotDotEqual => write!(f, "'..='"),
            TokenType::DotDotDot => write!(f, "'...'"),
            TokenType::Newline => write!(f, "newline"),
            TokenType::Eof => write!(f, "end of file"),
        }
    }
}
