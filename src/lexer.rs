#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    WhiteSpace,
    Number(f64),
    Property(String),

    // Operators
    Plus,               // +
    Minus,              // -
    Asterisk,           // *
    Slash,              // /
    Percent,            // %
    Equal,              // ==
    NotEqual,           // !=
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=

    // Other Symbols
    LeftParenthesis,  // (
    RightParenthesis, // )
    Comma,            // ,
}

#[derive(Debug, PartialEq)]
pub struct LexerError {
    pub msg: String,
}

impl LexerError {
    fn new(msg: &str) -> LexerError {
        LexerError {
            msg: msg.to_string(),
        }
    }
}

pub struct Lexer<'a> {
    /// 読込中の先頭文字列を指す
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &str) -> Lexer {
        Lexer {
            chars: input.chars().peekable(),
        }
    }

    /// 数式の字句解析
    ///
    /// サポートしている数式は以下の通りである
    ///
    /// - <expr>   ::= <term> [ ('+'|'-'|'%'|'=='|'>'|'<'|'>='|'<=') <term> ]*
    /// - <term>   ::= <factor> [ ('*'|'/') <factor> ]*
    /// - <factor> ::= <number> | '(' <expr> ')' | <function> | <variable>
    /// - <function> :== <property> '(' <expr>, [',' <expr> ]* ')' ← ただし、 property の1文字目は [A-Z]
    /// - <variable> := <property> ← ただし、1文字目は [a-z]
    /// - <number> :== ('+'|'-')[0-9]
    /// - <property> := [a-zA-Z]+
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        print!("tokenize");
        let mut tokens = vec![];
        for t in self.expr()? {
            // Whitespace は捨てる
            if t != Token::WhiteSpace {
                tokens.push(t);
            }
        }

        if self.chars.peek().is_some() {
            // 探索が終わっていなければなにかがおかしいので解析エラーとする
            // FIXME: expr 内での判定がおそらく良くないので、修正したい
            Err(LexerError::new("error: syntax error"))
        } else {
            Ok(tokens)
        }
    }

    /// 数式の解析
    /// <expr> ::= <term> [ ('+'|'-') <term> ]*
    fn expr(&mut self) -> Result<Vec<Token>, LexerError> {
        print!("expr");

        let mut tokens = self.term()?;

        loop {
            let w = self.read_whitespace_tokens();
            tokens = Lexer::add_tokens(tokens, w);

            // self.chars.peek(), self.chars.next() あたりで怒られるので仕方なく
            let mut chars = self.chars.clone();
            let cc = chars.peek();
            match cc {
                Some(c) => match c {
                    '>' | '<' | '=' | '!' => {
                        self.chars.next();
                        let token = self.read_comparison_operator(&c)?;
                        tokens.push(token);
                        tokens = Lexer::add_tokens(tokens, self.term()?);
                    }
                    '+' | '-' => {
                        tokens.push(Lexer::operator_to_token(&c.to_string())?);
                        self.chars.next();
                        tokens = Lexer::add_tokens(tokens, self.term()?);
                    }
                    _ => {
                        break;
                    }
                },
                None => {
                    break;
                }
            }
        }

        Ok(tokens)
    }

    /// 項の解析
    /// <term> ::= <factor> [ ('*'|'/') <factor> ]*
    fn term(&mut self) -> Result<Vec<Token>, LexerError> {
        print!("term");

        let mut tokens = self.factor()?;

        loop {
            tokens = Lexer::add_tokens(tokens, self.read_whitespace_tokens());

            match self.chars.peek() {
                Some(c) => match c {
                    '*' | '/' | '%' => {
                        tokens.push(Lexer::operator_to_token(&c.to_string())?);
                        self.chars.next();

                        tokens = Lexer::add_tokens(tokens, self.factor()?);
                    }
                    _ => break,
                },
                None => break,
            }
        }

        Ok(tokens)
    }

    /// 因数の解析
    /// <factor> ::= <number> | '(' <expr> ')' | <function> | <variable>
    fn factor(&mut self) -> Result<Vec<Token>, LexerError> {
        print!("factor");

        let mut tokens = self.read_whitespace_tokens();

        match self.chars.peek() {
            Some(c) => match c {
                '(' => {
                    // '(' <expr> ')'
                    tokens.push(Token::LeftParenthesis);
                    self.chars.next();

                    tokens = Lexer::add_tokens(tokens, self.expr()?);

                    tokens = Lexer::add_tokens(tokens, self.read_whitespace_tokens());

                    match self.chars.peek() {
                        Some(c) => {
                            if *c == ')' {
                                self.chars.next();
                                tokens.push(Token::RightParenthesis);

                                Ok(tokens)
                            } else {
                                Err(LexerError::new(&format!(
                                    "error: unexpected chars, {:?}",
                                    c
                                )))
                            }
                        }
                        None => Err(LexerError::new("error: unexpected end of line")),
                    }
                }
                c if c.is_numeric() || matches!(c, '+' | '-') => {
                    tokens = Lexer::add_tokens(tokens, self.number()?);
                    Ok(tokens)
                }
                c if c.is_uppercase() => {
                    tokens = Lexer::add_tokens(tokens, self.function()?);
                    Ok(tokens)
                }
                c if c.is_lowercase() => {
                    tokens = Lexer::add_tokens(tokens, self.variable()?);
                    Ok(tokens)
                }
                _ => Err(LexerError::new(&format!("error: unexpected char, {:?}", c))),
            },
            None => Err(LexerError::new(&format!("error: unexpected end of line"))),
        }
    }

    /// 関数の解析
    /// <function> :== <property> '(' <expr>, [',' <expr> ]* ')' ← ただし、 property の1文字目は [A-Z]
    fn function(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = self.property()?;

        match self.chars.peek() {
            Some(&c) => {
                if c == '(' {
                    tokens.push(Token::LeftParenthesis);
                    self.chars.next();

                    tokens = Lexer::add_tokens(tokens, self.expr()?);
                    tokens = Lexer::add_tokens(tokens, self.read_whitespace_tokens());

                    while let Some(cc) = self.chars.peek() {
                        match cc {
                            ',' => {
                                tokens.push(Token::Comma);
                                self.chars.next();

                                tokens = Lexer::add_tokens(tokens, self.expr()?);
                                tokens = Lexer::add_tokens(tokens, self.read_whitespace_tokens());
                            }
                            ')' => {
                                tokens.push(Token::RightParenthesis);
                                self.chars.next();

                                break;
                            }
                            _ => {
                                return Err(LexerError::new(&format!(
                                    "error: unexpected char after first argument, {:?}",
                                    cc
                                )));
                            }
                        }
                    }
                } else {
                    return Err(LexerError::new(&format!(
                        "error: unexpected char after property, {:?}",
                        c
                    )));
                }
            }
            None => return Err(LexerError::new("error: unexpected end of line")),
        }

        Ok(tokens)
    }

    /// 変数の解析
    /// <variable> := <property> ← ただし、1文字目は [a-z]
    fn variable(&mut self) -> Result<Vec<Token>, LexerError> {
        self.property()
    }

    /// <property> := [a-zA-Z]+
    fn property(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = self.read_whitespace_tokens();

        let mut property_str = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                self.chars.next();
                property_str.push(c);
            } else {
                break;
            }
        }

        if property_str.is_empty() {
            return Err(LexerError::new("error: property is empty"));
        }

        tokens.push(Token::Property(property_str));
        Ok(tokens)
    }

    /// <number> :== ('+'|'-')[0-9]
    fn number(&mut self) -> Result<Vec<Token>, LexerError> {
        print!("number");

        let mut tokens = self.read_whitespace_tokens();

        let mut number_str = String::new();
        while let Some(&c) = self.chars.peek() {
            // 数字に使われる可能性がある文字は読み込み、そうではない文字の場合は読み込みを終了する
            if c.is_numeric() | matches!(c, '.') | (number_str.is_empty() && matches!(c, '+' | '-'))
            {
                self.chars.next();
                number_str.push(c);
            } else {
                break;
            }
        }

        // 0xx のパターンが parse 時に panic を起こすので除去 (0.xx はOK)
        if number_str.len() > 1
            && number_str.chars().nth(0).unwrap() == '0'
            && number_str.chars().nth(1).unwrap() != '.'
        {
            return Err(LexerError::new("error: invalid numeric string"));
        }

        // 読み込んだ文字列がParseできた場合はTokenを返す
        match number_str.parse::<f64>() {
            Ok(number) => {
                tokens.push(Token::Number(number));
                Ok(tokens)
            }
            Err(e) => Err(LexerError::new(&format!("error: {}", e.to_string()))),
        }
    }

    fn read_whitespace_tokens(&mut self) -> Vec<Token> {
        let mut tokens = vec![];
        while let Some(c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
                tokens.push(Token::WhiteSpace);
            } else {
                break;
            }
        }

        tokens
    }

    fn read_comparison_operator(&mut self, first_char: &char) -> Result<Token, LexerError> {
        match first_char {
            '>' | '<' => match self.chars.peek() {
                // 次が、
                // '=' の場合は (Greater|Less)ThanOrEqual
                // 違う場合は (Greater|Less)Than
                Some(cc) => match cc {
                    '=' => {
                        let token = Lexer::operator_to_token(
                            vec![*first_char, *cc].iter().collect::<String>().as_str(),
                        )?;
                        self.chars.next();
                        Ok(token)
                    }
                    _ => Lexer::operator_to_token(first_char.to_string().as_str()),
                },
                None => Err(LexerError::new("error: unexpected end of line")),
            },
            '=' | '!' => match self.chars.peek() {
                // 次が、
                // '=' の場合は (Equal|NotEqual)
                // 違う場合はエラー
                Some(cc) => match cc {
                    '=' => {
                        let token = Lexer::operator_to_token(
                            vec![*first_char, *cc].iter().collect::<String>().as_str(),
                        )?;
                        self.chars.next();
                        Ok(token)
                    }
                    _ => Err(LexerError::new(&format!(
                        "error: unexpected char after equal, {:?}",
                        cc
                    ))),
                },
                None => Err(LexerError::new("error: unexpected end of line")),
            },
            _ => Err(LexerError::new(&format!(
                "error: unexpected char, {:?}",
                first_char
            ))),
        }
    }

    fn add_tokens(mut tokens: Vec<Token>, added_tokens: Vec<Token>) -> Vec<Token> {
        for t in added_tokens {
            tokens.push(t);
        }

        tokens
    }

    fn operator_to_token(c: &str) -> Result<Token, LexerError> {
        match c {
            "+" => Ok(Token::Plus),
            "-" => Ok(Token::Minus),
            "*" => Ok(Token::Asterisk),
            "/" => Ok(Token::Slash),
            "%" => Ok(Token::Percent),
            ">" => Ok(Token::GreaterThan),
            "<" => Ok(Token::LessThan),
            ">=" => Ok(Token::GreaterThanOrEqual),
            "<=" => Ok(Token::LessThanOrEqual),
            "==" => Ok(Token::Equal),
            "!=" => Ok(Token::NotEqual),
            _ => Err(LexerError::new(&format!("error: unexpected char, {:?}", c))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_whitespace_tokens() {
        assert_eq!(
            Lexer::new("  +30").read_whitespace_tokens(),
            vec![Token::WhiteSpace, Token::WhiteSpace]
        );
    }

    #[test]
    fn test_number() {
        assert_eq!(Lexer::new("30").number(), Ok(vec![Token::Number(30.0)]));
        assert_eq!(Lexer::new("-30").number(), Ok(vec![Token::Number(-30.0)]));
        assert_eq!(
            Lexer::new(" -30 ").number(),
            Ok(vec![Token::WhiteSpace, Token::Number(-30.0)])
        );
        assert_eq!(
            Lexer::new("30 - 20").number(),
            Ok(vec![Token::Number(30.0)])
        );
    }

    #[test]
    fn test_property() {
        assert_eq!(
            Lexer::new("Add(30, 20)").property(),
            Ok(vec![Token::Property("Add".to_string())])
        );
    }

    #[test]
    fn test_tokenize() {
        let success_data = [
            ("30", vec![Token::Number(30.0)]),
            ("-30", vec![Token::Number(-30.0)]),
            (
                "1+(-1)",
                vec![
                    Token::Number(1.0),
                    Token::Plus,
                    Token::LeftParenthesis,
                    Token::Number(-1.0),
                    Token::RightParenthesis,
                ],
            ),
            (
                "30/10+(10+20)",
                vec![
                    Token::Number(30.0),
                    Token::Slash,
                    Token::Number(10.0),
                    Token::Plus,
                    Token::LeftParenthesis,
                    Token::Number(10.0),
                    Token::Plus,
                    Token::Number(20.0),
                    Token::RightParenthesis,
                ],
            ),
            (
                "30 == 2",
                vec![Token::Number(30.0), Token::Equal, Token::Number(2.0)],
            ),
            (
                "30 > 2 <= -2 >= 2 < 1 != 0",
                vec![
                    Token::Number(30.0),
                    Token::GreaterThan,
                    Token::Number(2.0),
                    Token::LessThanOrEqual,
                    Token::Number(-2.0),
                    Token::GreaterThanOrEqual,
                    Token::Number(2.0),
                    Token::LessThan,
                    Token::Number(1.0),
                    Token::NotEqual,
                    Token::Number(0.0),
                ],
            ),
            (
                "1+2*(3*(4+5)+6)*(7+8)+9==1000<10!=1",
                vec![
                    Token::Number(1.0),
                    Token::Plus,
                    Token::Number(2.0),
                    Token::Asterisk,
                    Token::LeftParenthesis,
                    Token::Number(3.0),
                    Token::Asterisk,
                    Token::LeftParenthesis,
                    Token::Number(4.0),
                    Token::Plus,
                    Token::Number(5.0),
                    Token::RightParenthesis,
                    Token::Plus,
                    Token::Number(6.0),
                    Token::RightParenthesis,
                    Token::Asterisk,
                    Token::LeftParenthesis,
                    Token::Number(7.0),
                    Token::Plus,
                    Token::Number(8.0),
                    Token::RightParenthesis,
                    Token::Plus,
                    Token::Number(9.0),
                    Token::Equal,
                    Token::Number(1000.0),
                    Token::LessThan,
                    Token::Number(10.0),
                    Token::NotEqual,
                    Token::Number(1.0),
                ],
            ),
            (
                "Add((1 + 1), 2 * 3)",
                vec![
                    Token::Property("Add".to_string()),
                    Token::LeftParenthesis,
                    Token::LeftParenthesis,
                    Token::Number(1.0),
                    Token::Plus,
                    Token::Number(1.0),
                    Token::RightParenthesis,
                    Token::Comma,
                    Token::Number(2.0),
                    Token::Asterisk,
                    Token::Number(3.0),
                    Token::RightParenthesis,
                ],
            ),
            (
                "(hoge - (2 * 3)) / (4 + 5)",
                vec![
                    Token::LeftParenthesis,
                    Token::Property("hoge".to_string()),
                    Token::Minus,
                    Token::LeftParenthesis,
                    Token::Number(2.0),
                    Token::Asterisk,
                    Token::Number(3.0),
                    Token::RightParenthesis,
                    Token::RightParenthesis,
                    Token::Slash,
                    Token::LeftParenthesis,
                    Token::Number(4.0),
                    Token::Plus,
                    Token::Number(5.0),
                    Token::RightParenthesis,
                ],
            ),
        ];

        success_data.map(|(input, expected)| {
            assert_eq!(Lexer::new(input).tokenize(), Ok(expected));
        });

        let failure_data = ["2(3 + 2)", "Add()", "add(3)"];
        failure_data.map(|input| {
            assert_eq!(
                (Lexer::new(input).tokenize().is_err(), input),
                (true, input)
            );
        });
    }
}
