use lexer::Lexer;
use parser::Parser;
use processor::{Function, Processor, Variable};

mod lexer;
mod parser;
mod processor;

#[derive(Debug, PartialEq)]
enum ErrorType {
    Lexer,
    Parser,
    Processor,
}

#[derive(Debug, PartialEq)]
pub struct FormulaError {
    msg: String,
    error_type: ErrorType,
}

/// 数式を解析する
///
/// 例
///
/// - `parse_formula("(1 + 2) * 3", vec![], vec![]) // → 9`
/// - `parse_formula(
///  "Pow(2, 3) + 3",
///  vec![Function::new("Pow", 2, |args| args[0].powf(args[1]))],
///  vec![]
/// ) // → 11.0`
pub fn parse_formula(
    input: &str,
    functions: Vec<Function>,
    variables: Vec<Variable>,
) -> Result<f64, FormulaError> {
    let reserved_functions = vec![
        Function::new("Add", 2, |args| args[0] + args[1]),
        Function::new("Sub", 2, |args| args[0] - args[1]),
        Function::new("Mul", 2, |args| args[0] * args[1]),
        Function::new("Div", 2, |args| args[0] / args[1]),
        Function::new("Mod", 2, |args| args[0] % args[1]),
        Function::new(
            "If",
            3,
            |args| if args[0] == 0.0 { args[2] } else { args[1] },
        ),
    ];

    let mut all_functions = reserved_functions;
    for f in functions {
        all_functions.push(f);
    }

    Lexer::new(input)
        .tokenize()
        .map_err(|e| FormulaError {
            msg: e.msg,
            error_type: ErrorType::Lexer,
        })
        .and_then(|t| {
            Parser::new(t).parse().map_err(|e| FormulaError {
                msg: e.msg,
                error_type: ErrorType::Parser,
            })
        })
        .and_then(|v| {
            Processor::new(v, all_functions, variables)
                .execute()
                .map_err(|e| FormulaError {
                    msg: e.msg,
                    error_type: ErrorType::Processor,
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute() {
        let success_data = [
            ("4", 4.0),
            ("-4", -4.0),
            ("5 - 4 - (1)", 0.0),
            ("4 - 5", -1.0),
            ("(1 - 3) * 3", -6.0),
            ("(-1 + 3) * 3", 6.0),
            ("(3 - 5) % 3", -2.0),
            ("1+2*(3*(4+5)+6)*(7+8)+9==1000<10!=1", 0.0),
            ("1 == 2 * 3 < 1", 1.0),
            ("5 < 2 * 3", 1.0),
            ("5 > 2 * 3", 0.0),
            ("5 != 2 * 3", 1.0),
            ("5 == 2 * 3", 0.0),
            ("Add((2 + 3) + 4, 5) + Sub(2, 3)", 13.0),
            ("If(1 == (2 - 1), 3, 1)", 3.0),
            ("If(1 != (2 - 1), 3, 1)", 1.0),
            ("(1 - (2 * 3)) * (4 + 5)", -45.0),
            ("hoge + fuga * 3 - Add(1, 2)", 11.0),
            ("Pow(2, 3)", 8.0),
        ];
        success_data.map(|(input, expected)| {
            assert_eq!(
                parse_formula(
                    input,
                    vec![Function::new("Pow", 2, |args| args[0].powf(args[1]))],
                    vec![Variable::new("hoge", 2.0,), Variable::new("fuga", 4.0,)]
                ),
                Ok(expected)
            );
        });

        let failure_data = [
            "2(3 + 2)",
            "1 ! 3",
            "1 ==",
            "== 3",
            "=",
            "2+=3",
            "2 - 3 (()) + 3)",
            "()",
            "",
            "hello world",
            "add(2, 3)",
            "Add(2)",
            "add + 2 / 3",
        ];

        failure_data.map(|input| {
            assert_eq!(
                (parse_formula(input, vec![], vec![]).is_err(), input),
                (true, input)
            );
        });
    }
}
