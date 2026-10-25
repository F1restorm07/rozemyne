use std::collections::HashMap;

use self::expr::{Expr, FuncCall, SimpleExpr};

mod expr;

use super::token::Parser;
use super::{Path, Value};

// ripped from `tinytemplate` + `upon`
//
// do i want in intermediary structure here? or just parse directly into `Instruction`s
// like `tinytemplate`
#[derive(Debug)]
enum Instruction {
    // load a literal value onto the stack
    Literal(Value),
    // build a `Map` value for keyword arguments for a filter or function
    BuildMap(usize),
    // load an existing value onto the stack
    Value(Path),
    // apply a filter to a set of arguments
    // @todo: define how arguments are passed in (argument stack?)
    Apply {
        name: smol_str::SmolStr,
        // arity: usize, // number of arguments to pull from arg stack
        emit: bool, // do we emit the application to the output buffer?
    },
    Emit,
}

#[derive(Debug)]
pub struct Predicate {
    // the output buffer of this pipeline
    instrs: Vec<Instruction>
}

impl Predicate {
    pub fn compile(src: &str) -> Self {
        let parser = Parser::new(src);
        let mut compiler = Compiler::new();
        compiler.compile_expr(Expr::parse(&parser));
        compiler.last_emit_expr();

        Self {
            instrs: compiler.instrs,
        }
    }

    pub fn render(
        &self,
        target_buf: &mut String,
        state: &super::State<'_, '_>,
    ) {
        // emitting *sets* the target buffer to the emitted value
        // one can start the predicate expression with the target buffer, then append/prepend using filters
        let mut working_stack = Vec::new();
        for cmd in &self.instrs {
            match cmd {
                Instruction::Literal(lit) => {
                    working_stack.push(lit.clone());
                }
                Instruction::Value(path) => {
                    let val = state.context.get_with_path(path).unwrap();
                    working_stack.push(val.clone());
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
                Instruction::Apply { name, emit } => {
                    let formatter = state.filters.get(name.as_str()).unwrap();

                    let kwargs = working_stack.pop().unwrap();
                    let args = super::filter::Kwargs::new(kwargs.as_map().unwrap());
                    let op_val = working_stack.pop().unwrap();

                    let res = formatter.call(&op_val, args, state.context);
                    if *emit {
                        *target_buf = res.as_string().unwrap().to_string();
                    } else {
                        working_stack.push(res);
                    }
                }
                Instruction::Emit => {
                    let res = state.default_formatter
                        .call(working_stack.last().unwrap(), super::filter::Kwargs::new(HashMap::new()), state.context);
                    *target_buf = res.as_string().unwrap().to_string();
                }
            }
        }
    }
}

struct Compiler {
    instrs: Vec<Instruction>
}

// ripped from `upon`
impl Compiler {
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

    fn compile_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Simple(simp) => {
                self.compile_simple_expr(simp);
            }
            Expr::Filter(expr::Filter { name, args, receiver }) => {
                self.compile_expr(*receiver);
                match args {
                    None => self.instrs.push(Instruction::BuildMap(0)),
                    Some(args) => {
                        let arity = args.0.len();
                        for (k, v) in args.0 {
                            self.instrs.push(Instruction::Literal(Value::Str(k)));
                            self.compile_simple_expr(v);
                        }
                        self.instrs.push(Instruction::BuildMap(arity));
                    }
                }
                self.instrs.push(Instruction::Apply { name, emit: false });
            }
        }
    }

    fn compile_simple_expr(&mut self, expr: SimpleExpr) {
        match expr {
            SimpleExpr::String(s) => {
                self.instrs.push(Instruction::Literal(Value::Str(s)));
            }
            SimpleExpr::Path(p) => {
                self.instrs.push(Instruction::Value(p));
            }
            SimpleExpr::Block(b) => {
                self.compile_expr(*b);
            }
            SimpleExpr::Func(FuncCall { name, args }) => {
                match args {
                    None => self.instrs.push(Instruction::BuildMap(0)),
                    Some(args) => {
                        let arity = args.0.len();
                        for (k, v) in args.0 {
                            self.instrs.push(Instruction::Literal(Value::Str(k)));
                            self.compile_simple_expr(v);
                        }
                        self.instrs.push(Instruction::BuildMap(arity));
                    }
                }
                self.instrs.push(Instruction::Apply { name, emit: false });
            }
        }
    }
}
