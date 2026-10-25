use std::collections::HashMap;

use self::expr::{Expr, FuncCall, InfixOp, PrefixOp};

use super::{State, Value};
use super::token::Parser;

mod expr;

#[derive(Debug)]
enum Instruction {
    Literal(Value),
    Value(super::Path),
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    Contains,
    Matches,
    And,
    Or,
    Xor,
    Not,
    Apply(smol_str::SmolStr),
    BuildMap(usize),
}

struct Compiler {
    instrs: Vec<Instruction>,
}

impl<'src> Compiler {
    fn new() -> Self {
        Self { instrs: Vec::new() }
    }

    fn compile_expr(&mut self, expr: Expr<'src>) {
        match expr {
            Expr::String(s) => {
                self.instrs.push(Instruction::Literal(Value::Str(smol_str::SmolStr::new(s))));
            }
            Expr::Regex(r) => {
                self.instrs.push(Instruction::Literal(Value::Regex(r)));
            }
            Expr::Path(p) => {
                self.instrs.push(Instruction::Value(p));
            }
            Expr::Func(FuncCall { name, args }) => {
                match args {
                    None => self.instrs.push(Instruction::BuildMap(0)),
                    Some(args) => {
                        let arity = args.0.len();
                        for (k, v) in args.0 {
                            self.instrs.push(Instruction::Literal(Value::Str(k)));
                            self.compile_expr(v);
                        }
                        self.instrs.push(Instruction::BuildMap(arity));
                    }
                }
                self.instrs.push(Instruction::Apply(name));
            }
            Expr::InfixOp(lhs, op, rhs) => {
                self.compile_expr(*lhs);
                self.compile_expr(*rhs);
                match op {
                    InfixOp::Equal => self.instrs.push(Instruction::Equal),
                    InfixOp::NotEqual => self.instrs.push(Instruction::NotEqual),
                    InfixOp::GreaterThan => self.instrs.push(Instruction::GreaterThan),
                    InfixOp::LessThan => self.instrs.push(Instruction::LessThan),
                    InfixOp::GreaterEqual => self.instrs.push(Instruction::GreaterEqual),
                    InfixOp::LessEqual => self.instrs.push(Instruction::LessEqual),
                    InfixOp::Contains => self.instrs.push(Instruction::Contains),
                    InfixOp::Matches => self.instrs.push(Instruction::Matches),
                    InfixOp::And => self.instrs.push(Instruction::And),
                    InfixOp::Or => self.instrs.push(Instruction::Or),
                    InfixOp::Xor => self.instrs.push(Instruction::Xor),
                }
            }
            Expr::PrefixOp(op, expr) => {
                self.compile_expr(*expr);
                match op {
                    PrefixOp::Not => self.instrs.push(Instruction::Not),
                }
            }
            Expr::Test(expr::Test { name, args, receiver }) => {
                self.compile_expr(*receiver);
                match args {
                    None => self.instrs.push(Instruction::BuildMap(0)),
                    Some(args) => {
                        let arity = args.0.len();
                        for (k, v) in args.0 {
                            self.instrs.push(Instruction::Literal(Value::Str(k)));
                            self.compile_expr(v);
                        }
                        self.instrs.push(Instruction::BuildMap(arity));
                    }
                }
                self.instrs.push(Instruction::Apply(smol_str::SmolStr::new(name)));
            }
        }
    }
}

#[derive(Debug)]
pub struct Condition {
    instrs: Vec<Instruction>
}

impl Condition {
    pub fn compile(src: &str) -> Self {
        let parser = Parser::new(src);
        let mut compiler = Compiler::new();
        compiler.compile_expr(Expr::parse(&parser, 0));

        Self {
            instrs: compiler.instrs,
        }
    }

    pub fn run(&self, state: &State) -> bool {
        let mut working_stack = Vec::new();
        let mut cond_stack = Vec::new();
        for cmd in &self.instrs {
            match cmd {
                Instruction::Literal(lit) => {
                    working_stack.push(lit.clone());
                }
                Instruction::Value(path) => {
                    let val = state.context.get_with_path(path).unwrap();
                    working_stack.push(val.clone());
                }
                Instruction::Equal => {
                    let rhs = working_stack.pop().unwrap();
                    let lhs = working_stack.pop().unwrap();
                    cond_stack.push(lhs == rhs);
                }
                Instruction::NotEqual => {
                    let rhs = working_stack.pop().unwrap();
                    let lhs = working_stack.pop().unwrap();
                    cond_stack.push(lhs != rhs);
                }
                Instruction::GreaterThan => {
                    let rhs = working_stack.pop().unwrap();
                    let lhs = working_stack.pop().unwrap();
                    cond_stack.push(lhs > rhs);
                }
                Instruction::LessThan => {
                    let rhs = working_stack.pop().unwrap();
                    let lhs = working_stack.pop().unwrap();
                    cond_stack.push(lhs < rhs);
                }
                Instruction::GreaterEqual => {
                    let rhs = working_stack.pop().unwrap();
                    let lhs = working_stack.pop().unwrap();
                    cond_stack.push(lhs >= rhs);
                }
                Instruction::LessEqual => {
                    let rhs = working_stack.pop().unwrap();
                    let lhs = working_stack.pop().unwrap();
                    cond_stack.push(lhs <= rhs);
                }
                Instruction::Contains => {
                    let rhs = working_stack.pop().unwrap();
                    let lhs = working_stack.pop().unwrap().as_array().unwrap();
                    cond_stack.push(lhs.contains(&rhs));
                }
                Instruction::Matches => {
                    let rhs = working_stack.pop().unwrap().as_regex().unwrap();
                    let lhs = working_stack.pop().unwrap().as_string().unwrap();
                    cond_stack.push(rhs.is_match(&lhs));
                }
                Instruction::And => {
                    let rhs = cond_stack.pop().unwrap();
                    let lhs = cond_stack.pop().unwrap();
                    cond_stack.push(lhs && rhs);
                }
                Instruction::Or => {
                    let rhs = cond_stack.pop().unwrap();
                    let lhs = cond_stack.pop().unwrap();
                    cond_stack.push(lhs || rhs);
                }
                Instruction::Xor => {
                    let rhs = cond_stack.pop().unwrap();
                    let lhs = cond_stack.pop().unwrap();
                    cond_stack.push(lhs ^ rhs);
                }
                Instruction::Not => {
                    let cond = cond_stack.pop().unwrap();
                    cond_stack.push(!cond);
                }
                Instruction::BuildMap(size) => {
                    let mut map_elems = Vec::with_capacity(*size);
                    for _ in 0..*size {
                        let val = working_stack.pop().unwrap();
                        let key = working_stack.pop().unwrap();
                        map_elems.push((key.as_string().unwrap(), val));
                    }
                    map_elems.reverse();
                    let map = HashMap::from_iter(map_elems);
                    working_stack.push(Value::Map(map));
                }
                Instruction::Apply(name) => {
                    if let Some(test) = state.tests.get(name.as_str()) {
                        let kwargs = working_stack.pop().unwrap();
                        let args = super::filter::Kwargs::new(kwargs.as_map().unwrap());
                        let op_val = working_stack.pop().unwrap();

                        let res = test.call(&op_val, args, state.context);
                        cond_stack.push(res);
                    } else {
                        let func = state.functions.get(name.as_str()).unwrap();

                        let kwargs = working_stack.pop().unwrap();
                        let args = super::filter::Kwargs::new(kwargs.as_map().unwrap());

                        let res = func.call(args, state.context).as_bool().unwrap();
                        cond_stack.push(res);
                    }
                }
            }
        }
        // there should be *one* boolean value on the condition stack at the end
        cond_stack.pop().unwrap()
    }
}
