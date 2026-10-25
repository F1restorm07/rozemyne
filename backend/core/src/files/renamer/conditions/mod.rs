use regex::Regex;

use self::parser::{Expr, InfixOp, PrefixOp};

use super::{Context, Parser};

mod parser;
pub use parser::Token;

#[derive(Debug)]
enum Instruction<'src> {
    Literal(&'src str),
    Value(super::Path<'src>),
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
}

struct Compiler<'src> {
    instrs: Vec<Instruction<'src>>,
}

impl<'src> Compiler<'src> {
    fn new() -> Self {
        Self { instrs: Vec::new() }
    }

    fn compile_expr(&mut self, expr: Expr<'src>) {
        match expr {
            Expr::String(s) => {
                self.instrs.push(Instruction::Literal(s));
            }
            Expr::Path(p) => {
                self.instrs.push(Instruction::Value(p));
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
        }
    }
}

#[derive(Debug)]
pub struct Condition<'src> {
    instrs: Vec<Instruction<'src>>
}

impl<'src> Condition<'src> {
    pub fn compile(src: &'src str) -> Self {
        let mut parser = Parser::<Token>::new(src);
        let mut compiler = Compiler::new();
        compiler.compile_expr(Expr::new(&mut parser, 0));

        Self {
            instrs: compiler.instrs,
        }
    }

    pub fn run(&self, context: &Context<'_, '_>) -> bool {
        let mut working_stack = Vec::new();
        let mut cond_stack = Vec::new();
        for cmd in &self.instrs {
            match cmd {
                Instruction::Literal(lit) => {
                    working_stack.push((*lit).into());
                }
                Instruction::Value(path) => {
                    let val = context.get_with_path(path).unwrap();
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
                    let lhs = working_stack.pop().unwrap();
                    cond_stack.push(lhs.contains(&*rhs));
                }
                Instruction::Matches => {
                    // @todo: compile the regex at the parsing stage
                    let rhs = Regex::new(&working_stack.pop().unwrap()).unwrap();
                    let lhs = working_stack.pop().unwrap();
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
            }
        }
        // there should be *one* boolean value on the condition stack at the end
        cond_stack.pop().unwrap()
    }
}
