use std::borrow::Cow;
use std::collections::HashMap;

pub use self::filter::{Filter, FilterArgs};
use self::parser::{Expr, FuncCall, SimpleExpr};

pub mod filter;
mod parser;
pub use parser::Token;

use super::{Context, Parser, Path};

// ripped from `tinytemplate` + `upon`
//
// do i want in intermediary structure here? or just parse directly into `Instruction`s
// like `tinytemplate`
#[derive(Debug)]
enum Instruction<'src> {
    // emit literal string into output buffer
    Literal(&'src str),
    // emit an existing value into the output buffer
    Value(Path<'src>),
    // apply a filter to a set of arguments
    // @todo: define how arguments are passed in (argument stack?)
    Apply {
        name: &'src str,
        arity: usize, // number of arguments to pull from arg stack
        emit: bool, // do we emit the application to the output buffer?
    },
    Emit,
}

#[derive(Debug)]
pub struct Predicate<'src> {
    // the output buffer of this pipeline
    instrs: Vec<Instruction<'src>>
}

impl<'src> Predicate<'src> {
    pub fn compile(src: &'src str) -> Self {
        let mut parser = Parser::new(src);
        let mut compiler = Compiler::new();
        compiler.compile_expr(Expr::new(&mut parser));
        compiler.last_emit_expr();

        Self {
            instrs: compiler.instrs,
        }
    }

    pub fn render(
        &self,
        target_buf: &mut String,
        filter_registry: HashMap<&str, Box<dyn Filter>>,
        default_formatter: Box<dyn Filter>,
        context: Context<'_, '_>,
    ) {
        // emitting *sets* the target buffer to the emitted value
        // one can start the predicate expression with the target buffer, then append/prepend using filters
        let mut working_stack = Vec::new();
        for cmd in &self.instrs {
            match cmd {
                Instruction::Literal(lit) => {
                    working_stack.push((*lit).into());
                }
                Instruction::Value(path) => {
                    let val = context.get_with_path(path).unwrap();
                    working_stack.push(val.clone());
                }
                Instruction::Apply { name, arity, emit } => {
                    let formatter = filter_registry.get(name).unwrap();
                    // the `+ 1` is to account for the operating_value
                    let args = working_stack.split_off(working_stack.len() - arity + 1);
                    let args = FilterArgs::new(args);
                    let op_val = working_stack.pop().unwrap();
                    let res = formatter.call(&op_val, args);
                    if *emit {
                        *target_buf = res;
                    } else {
                        working_stack.push(Cow::Owned(res));
                    }
                }
                Instruction::Emit => {
                    let res = default_formatter
                        .call(working_stack.last().unwrap(), FilterArgs::new(vec![]));
                    *target_buf = res;
                }
            }
        }
    }
}

struct Compiler<'src> {
    instrs: Vec<Instruction<'src>>
}

// ripped from `upon`
impl<'src> Compiler<'src> {
    fn new() -> Self {
        Self { instrs: Vec::new() }
    }

    fn last_emit_expr(&mut self) {
        match self.instrs.last_mut() {
            Some(Instruction::Apply { emit, .. }) => {
                *emit = true;
            }
            _ => self.instrs.push(Instruction::Emit),
        }
    }

    fn compile_expr(&mut self, expr: Expr<'src>) {
        match expr {
            Expr::Simple(simp) => {
                self.compile_simple_expr(simp);
            }
            Expr::Filter(parser::Filter { name, args, receiver }) => {
                self.compile_expr(*receiver);
                let arity = match args {
                    None => 0,
                    Some(args) => {
                        let arity = args.0.len();
                        for a in args.0 {
                            self.compile_simple_expr(a);
                        }
                        arity
                    }
                } + 1; // add one for receiver (pushed to instruction list first ^^)
                self.instrs.push(Instruction::Apply { name, arity, emit: false });
            }
        }
    }

    fn compile_simple_expr(&mut self, expr: SimpleExpr<'src>) {
        match expr {
            SimpleExpr::String(s) => {
                self.instrs.push(Instruction::Literal(s));
            }
            SimpleExpr::Path(p) => {
                self.instrs.push(Instruction::Value(p));
            }
            SimpleExpr::Block(b) => {
                self.compile_expr(*b);
            }
            SimpleExpr::Func(FuncCall { name, args }) => {
                let arity = match args {
                    None => 0,
                    Some(args) => {
                        let arity = args.0.len();
                        for a in args.0 {
                            self.compile_simple_expr(a);
                        }
                        arity
                    }
                };
                self.instrs.push(Instruction::Apply { name, arity, emit: false });
            }
        }
    }
}
