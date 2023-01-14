use std::collections::LinkedList;

use crate::parser::Value;

pub struct Function {
    name: String,
    args_count: usize,
    handler: fn(Vec<f64>) -> f64,
}

impl Function {
    pub fn new(name: &str, args_count: usize, handler: fn(Vec<f64>) -> f64) -> Function {
        Function {
            name: name.to_string(),
            args_count,
            handler,
        }
    }

    fn calc(&self, args: Vec<f64>) -> Result<f64, ProcessorError> {
        // 引数があっていなければエラーとする
        if args.len() != self.args_count {
            Err(ProcessorError::new(&format!(
                "error: args count of {:?} expects {:?}, but provide {:?}",
                self.name,
                self.args_count,
                args.len()
            )))
        } else {
            Ok((self.handler)(args))
        }
    }
}

pub struct Variable {
    name: String,
    value: f64,
}

impl Variable {
    pub fn new(name: &str, value: f64) -> Variable {
        Variable {
            name: name.to_string(),
            value,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ProcessorError {
    pub msg: String,
}

impl ProcessorError {
    fn new(msg: &str) -> ProcessorError {
        ProcessorError {
            msg: msg.to_string(),
        }
    }
}

pub struct Processor {
    values: Vec<Value>,
    functions: Vec<Function>,
    variables: Vec<Variable>,
    index: usize,
}

impl Processor {
    pub fn new(
        values: Vec<Value>,
        functions: Vec<Function>,
        variables: Vec<Variable>,
    ) -> Processor {
        Processor {
            values,
            functions,
            variables,
            index: 0,
        }
    }

    /// 逆ポーランド記法に変換された数式を評価する
    pub fn execute(&mut self) -> Result<f64, ProcessorError> {
        let mut stack = LinkedList::new();

        loop {
            match self.values.get(self.index) {
                Some(vv) => match vv {
                    // 値をスタックにプッシュする
                    Value::Number(num) => stack.push_back(*num),
                    Value::Function(f) => {
                        // 関数の一覧から関数名を元に関数を取得し、実行する
                        match self.functions.iter().find(|ff| ff.name == f.to_string()) {
                            Some(func) => {
                                let mut args = vec![];
                                // 引数の数だけスタックからポップし、関数の引数に指定する
                                for _ in 0..func.args_count {
                                    args.push(
                                        stack
                                            .pop_back()
                                            .ok_or(ProcessorError::new("error: syntax error"))?,
                                    )
                                }
                                // 後ろの値からポップされるので、順番を入れ替える
                                // e.g. 2 3 Add の場合、3 → 2 の順でスタックからポップされる
                                args.reverse();

                                let result = func.calc(args)?;
                                stack.push_back(result);
                            }
                            None => {
                                return Err(ProcessorError::new(&format!(
                                    "error: unknown function, {:?}",
                                    f
                                )))
                            }
                        }
                    }
                    Value::Variable(v) => {
                        // 変数の一覧から変数名を元に変数を取得し、評価する
                        match self.variables.iter().find(|vv| vv.name == v.to_string()) {
                            Some(vv) => {
                                // 引数の値をスタックにプッシュする
                                stack.push_back(vv.value);
                            }
                            None => {
                                return Err(ProcessorError::new(&format!(
                                    "error: unknown variable, {:?}",
                                    v
                                )))
                            }
                        }
                    }
                    _ => {
                        // 二項演算子の評価
                        let v1 = stack
                            .pop_back()
                            .ok_or(ProcessorError::new("error: syntax error"))?;
                        let v2 = stack
                            .pop_back()
                            .ok_or(ProcessorError::new("error: syntax error"))?;

                        stack.push_back(Processor::calc_binary_operator(v2, v1, vv)?);
                    }
                },
                None => break,
            }

            self.next();
        }

        if stack.len() == 1 {
            Ok(stack.pop_back().unwrap())
        } else {
            Err(ProcessorError::new("error: syntax error"))
        }
    }

    fn calc_binary_operator(v1: f64, v2: f64, operator: &Value) -> Result<f64, ProcessorError> {
        match operator {
            Value::Plus => Ok(v1 + v2),
            Value::Minus => Ok(v1 - v2),
            Value::Asterisk => Ok(v1 * v2),
            Value::Slash => Ok(v1 / v2),
            Value::Percent => Ok(v1 % v2),
            Value::Equal => Ok(if v1 == v2 { 1.0 } else { 0.0 }),
            Value::NotEqual => Ok(if v1 != v2 { 1.0 } else { 0.0 }),
            Value::GreaterThan => Ok(if v1 > v2 { 1.0 } else { 0.0 }),
            Value::GreaterThanOrEqual => Ok(if v1 >= v2 { 1.0 } else { 0.0 }),
            Value::LessThan => Ok(if v1 < v2 { 1.0 } else { 0.0 }),
            Value::LessThanOrEqual => Ok(if v1 <= v2 { 1.0 } else { 0.0 }),
            _ => Err(ProcessorError::new(&format!(
                "error: unexpected token, {:?}",
                operator
            ))),
        }
    }

    fn next(&mut self) -> Option<&Value> {
        self.index += 1;
        self.values.get(self.index - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute() {
        let success_data = [
            (
                // Minus(-1.0)
                vec![Value::Number(1.0), Value::Function("Minus".to_string())],
                vec![Function::new("Minus", 1, |args| -1.0 * args[0])],
                Ok(-1.0),
            ),
            (
                // Add((2 + 3) + 4, 5) + Sub(2, 3)
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
                vec![
                    Function::new("Add", 2, |args| args[0] + args[1]),
                    Function::new("Sub", 2, |args| args[0] - args[1]),
                ],
                Ok(13.0),
            ),
            (
                // 1 - 2 * 3
                vec![
                    Value::Number(1.0),
                    Value::Number(2.0),
                    Value::Variable("hoge".to_string()),
                    Value::Asterisk,
                    Value::Minus,
                ],
                vec![],
                Ok(-5.0),
            ),
            (
                // 1+2*(3*(4+5)+6)*(7+8)+9==1000<10!=1
                // [1, 2, 3, 4, 5, "+", "*", 6, "+", "*", 7, 8, "+", "*", "+", 9, "+", 1000, "==", 10, "!=", 1]
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
                vec![],
                Ok(0.0),
            ),
        ];

        success_data.map(|(input, functions, expected)| {
            assert_eq!(
                Processor::new(input, functions, vec![Variable::new("hoge", 3.0)]).execute(),
                expected
            );
        });

        let failure_data = [
            (
                vec![
                    Value::Number(1.0),
                    Value::Number(2.0),
                    Value::Number(3.0),
                    Value::Number(4.0),
                    Value::Plus,
                    Value::Asterisk,
                ],
                vec![],
                vec![],
            ),
            (
                vec![Value::Number(1.0), Value::Function("Add".to_string())],
                vec![Function::new("Add", 2, |args| args[0] + args[1])],
                vec![],
            ),
            (
                vec![
                    Value::Number(1.0),
                    Value::Function("add".to_string()),
                    Value::Number(2.0),
                ],
                vec![],
                vec![Variable::new("not_add", 3.0)],
            ),
        ];

        failure_data.map(|(input, functions, variables)| {
            assert_eq!(
                (Processor::new(input, functions, variables)
                    .execute()
                    .is_err()),
                (true)
            );
        });
    }
}
