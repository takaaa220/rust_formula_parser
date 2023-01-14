// lexer によって解析された Token のリストを中間表現に落とし込む
// おそらく逆ポーランド記法を採用するはず。

use std::collections::LinkedList;

use crate::lexer::Token;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Number(f64),
    Function(String),
    Variable(String),
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

#[derive(Debug, PartialEq)]
pub struct ParserError {
    pub msg: String,
}

impl ParserError {
    fn new(msg: &str) -> ParserError {
        ParserError {
            msg: msg.to_string(),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser { tokens, index: 0 }
    }

    /// 字句解析によってトークンに変換された数式を、中間表現 (逆ポーランド記法) に変換する
    pub fn parse(&mut self) -> Result<Vec<Value>, ParserError> {
        let tokens = self.parse_expr()?;
        if tokens.is_empty() | self.peek().is_some() {
            // トークンが空 or 探索が終わっていない場合は解析エラーとする
            return Err(ParserError::new("error: syntax error"));
        }

        Ok(tokens)
    }

    /// 操車場アルゴリズムによってトークンを逆ポーランド記法に変換する
    ///
    /// see: https://ja.wikipedia.org/wiki/%E6%93%8D%E8%BB%8A%E5%A0%B4%E3%82%A2%E3%83%AB%E3%82%B4%E3%83%AA%E3%82%BA%E3%83%A0
    pub fn parse_expr(&mut self) -> Result<Vec<Value>, ParserError> {
        let mut values = vec![];
        let mut stack = LinkedList::new();

        loop {
            match self.peek() {
                Some(token) => match token {
                    Token::WhiteSpace => {
                        self.next();
                    }
                    Token::Number(number) => {
                        values.push(Value::Number(*number));
                        self.next();
                    }
                    Token::Plus
                    | Token::Minus
                    | Token::Percent
                    | Token::Equal
                    | Token::NotEqual
                    | Token::GreaterThan
                    | Token::GreaterThanOrEqual
                    | Token::LessThan
                    | Token::LessThanOrEqual => loop {
                        match stack.back() {
                            Some(t) => match t {
                                // o1の優先度がo2以上ではない
                                Token::Plus
                                | Token::Minus
                                | Token::Percent
                                | Token::Asterisk
                                | Token::Slash
                                | Token::Equal
                                | Token::NotEqual
                                | Token::GreaterThan
                                | Token::GreaterThanOrEqual
                                | Token::LessThan
                                | Token::LessThanOrEqual => {
                                    values.push(Parser::token_into_value(t, true)?);
                                    stack.pop_back();
                                }
                                _ => {
                                    stack.push_back(token.clone());
                                    self.next();
                                    break;
                                }
                            },
                            None => {
                                stack.push_back(token.clone());
                                self.next();

                                break;
                            }
                        }
                    },
                    Token::Asterisk | Token::Slash => loop {
                        match stack.back() {
                            Some(t) => match t {
                                Token::Asterisk | Token::Slash => {
                                    // o1の優先度がo2より高くない && o1が左結合性のため、スタックのトップから演算子トークンを取り出して出力キューに追加する
                                    values.push(Parser::token_into_value(t, true)?);
                                    stack.pop_back();
                                }
                                _ => {
                                    stack.push_back(token.clone());
                                    self.next();

                                    break;
                                }
                            },
                            None => {
                                stack.push_back(token.clone());
                                self.next();

                                break;
                            }
                        }
                    },
                    Token::LeftParenthesis => {
                        stack.push_back(token.clone());
                        self.next();
                    }
                    Token::RightParenthesis => {
                        // スタックのトップにあるトークンが左括弧になるまで、スタックからポップした演算子を出力キューに追加する動作を繰り返す。
                        // 左括弧をスタックからポップするが、出力には追加せずに捨てる。
                        loop {
                            match stack.pop_back() {
                                Some(t) => match t {
                                    Token::Plus
                                    | Token::Minus
                                    | Token::Asterisk
                                    | Token::Slash
                                    | Token::Percent
                                    | Token::Equal
                                    | Token::NotEqual
                                    | Token::GreaterThan
                                    | Token::GreaterThanOrEqual
                                    | Token::LessThan
                                    | Token::LessThanOrEqual => {
                                        values.push(Parser::token_into_value(&t, true)?);
                                    }
                                    Token::LeftParenthesis => {
                                        self.next();

                                        // スタックのトップにあるトークンが関数トークンなら、それをポップして出力キューに追加する。
                                        if let Some(tt) = stack.back() {
                                            if let Token::Property(_) = tt {
                                                values.push(Parser::token_into_value(tt, true)?);
                                                stack.pop_back();
                                            }
                                        }

                                        break;
                                    }
                                    _ => {
                                        return Err(ParserError::new(&format!(
                                            "error: unexpected property, token: {:?}",
                                            t
                                        )))
                                    }
                                },
                                None => {
                                    return Err(ParserError::new(
                                        "error: parenthesis is not matchedd",
                                    ))
                                }
                            }
                        }
                    }
                    Token::Property(_) => {
                        let t = token.clone();
                        self.next();

                        // 次が ( → 関数, それ以外 → 変数
                        match self.peek() {
                            Some(tt) => match tt {
                                Token::LeftParenthesis => {
                                    stack.push_back(t);
                                }
                                _ => values.push(Parser::token_into_value(&t, false)?),
                            },
                            None => values.push(Parser::token_into_value(&t, false)?),
                        }
                    }
                    Token::Comma => loop {
                        // スタックのトップにあるトークンが左括弧となるまで、スタックから演算子をポップして出力キューに追加する動作を繰り返す。左括弧が出てこない場合、引数セパレータの位置がおかしいか、左右の括弧が不一致となっている（エラー）。
                        match stack.back() {
                            Some(t) => match t {
                                Token::Plus
                                | Token::Minus
                                | Token::Asterisk
                                | Token::Slash
                                | Token::Percent
                                | Token::Equal
                                | Token::NotEqual
                                | Token::GreaterThan
                                | Token::GreaterThanOrEqual
                                | Token::LessThan
                                | Token::LessThanOrEqual => {
                                    values.push(Parser::token_into_value(&t, true)?);
                                    stack.pop_back();
                                }
                                Token::LeftParenthesis => {
                                    self.next();
                                    break;
                                }
                                _ => {
                                    return Err(ParserError::new(&format!(
                                        "error: unexpected property, token: {:?}",
                                        t
                                    )))
                                }
                            },
                            None => {
                                // ここに入っている模様
                                return Err(ParserError::new("error: parenthesis is not matched"));
                            }
                        }
                    },
                },
                None => break,
            }
        }

        loop {
            match stack.pop_back() {
                Some(t) => match t {
                    Token::Plus
                    | Token::Minus
                    | Token::Percent
                    | Token::Asterisk
                    | Token::Slash
                    | Token::Equal
                    | Token::NotEqual
                    | Token::GreaterThan
                    | Token::GreaterThanOrEqual
                    | Token::LessThan
                    | Token::LessThanOrEqual => {
                        values.push(Parser::token_into_value(&t, true)?);
                    }
                    _ => {
                        return Err(ParserError::new(&format!(
                            "error: unexpected token: {:?}",
                            t
                        )))
                    }
                },
                None => break,
            }
        }

        Ok(values)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<&Token> {
        self.index += 1;
        self.tokens.get(self.index - 1)
    }

    fn token_into_value(token: &Token, is_function: bool) -> Result<Value, ParserError> {
        match token {
            Token::Plus => Ok(Value::Plus),
            Token::Minus => Ok(Value::Minus),
            Token::Percent => Ok(Value::Percent),
            Token::Asterisk => Ok(Value::Asterisk),
            Token::Slash => Ok(Value::Slash),
            Token::Equal => Ok(Value::Equal),
            Token::NotEqual => Ok(Value::NotEqual),
            Token::GreaterThan => Ok(Value::GreaterThan),
            Token::GreaterThanOrEqual => Ok(Value::GreaterThanOrEqual),
            Token::LessThan => Ok(Value::LessThan),
            Token::LessThanOrEqual => Ok(Value::LessThanOrEqual),
            Token::Property(f) => Ok(if is_function {
                Value::Function(f.to_string())
            } else {
                Value::Variable(f.to_string())
            }),
            _ => Err(ParserError::new(&format!(
                "error: unexpected token, {:?}",
                token
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let success_data = [
            (
                // Add((2 + 3) + 4, 5) + Sub(2, 3)
                // → 2 3 + 4 + 2 Add 2 3 Sub +
                vec![
                    Token::Property("Add".to_string()),
                    Token::LeftParenthesis,
                    Token::LeftParenthesis,
                    Token::Number(2.0),
                    Token::Plus,
                    Token::Number(3.0),
                    Token::RightParenthesis,
                    Token::Plus,
                    Token::Number(4.0),
                    Token::Comma,
                    Token::Number(5.0),
                    Token::RightParenthesis,
                    Token::Plus,
                    Token::Property("Sub".to_string()),
                    Token::LeftParenthesis,
                    Token::Number(2.0),
                    Token::Comma,
                    Token::Number(3.0),
                    Token::RightParenthesis,
                ],
                vec![
                    Value::Number(2.0),
                    Value::Number(3.0),
                    Value::Plus,
                    Value::Number(4.0),
                    Value::Plus,
                    Value::Number(5.0),
                    Value::Function("Add".to_string()),
                    Value::Number(2.0),
                    Value::Number(3.0),
                    Value::Function("Sub".to_string()),
                    Value::Plus,
                ],
            ),
            (
                // 1+2*(3*(4+5)+6)*(7+8)+9==1000<10!=1
                // → [1, 2, 3, 4, 5, "+", "*", 6, "+", "*", 7, 8, "+", "*", "+", 9, "+"]
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
                vec![
                    Value::Number(1.0),
                    Value::Number(2.0),
                    Value::Number(3.0),
                    Value::Number(4.0),
                    Value::Number(5.0),
                    Value::Plus,
                    Value::Asterisk,
                    Value::Number(6.0),
                    Value::Plus,
                    Value::Asterisk,
                    Value::Number(7.0),
                    Value::Number(8.0),
                    Value::Plus,
                    Value::Asterisk,
                    Value::Plus,
                    Value::Number(9.0),
                    Value::Plus,
                    Value::Number(1000.0),
                    Value::Equal,
                    Value::Number(10.0),
                    Value::LessThan,
                    Value::Number(1.0),
                    Value::NotEqual,
                ],
            ),
            (
                // (hoge - (2 * 3)) / (4 + 5)
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
                    Token::Plus,
                    Token::LeftParenthesis,
                    Token::Number(4.0),
                    Token::Slash,
                    Token::Number(5.0),
                    Token::RightParenthesis,
                ],
                vec![
                    Value::Variable("hoge".to_string()),
                    Value::Number(2.0),
                    Value::Number(3.0),
                    Value::Asterisk,
                    Value::Minus,
                    Value::Number(4.0),
                    Value::Number(5.0),
                    Value::Slash,
                    Value::Plus,
                ],
            ),
        ];

        success_data.map(|(input, expected)| {
            assert_eq!(Parser::new(input).parse(), Ok(expected));
        });

        let failure_data = [
            // 1+2*(3*(4+5)+6)*(7+8+9
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
                Token::Plus,
                Token::Number(9.0),
            ],
        ];

        failure_data.map(|input| {
            assert_eq!(Parser::new(input).parse().is_err(), true);
        });
    }
}
