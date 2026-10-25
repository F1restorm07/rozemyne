use std::collections::HashMap;
use std::sync::Arc;

use crate::renamer::Value;

use super::Context;

// vvv ripped from `tera`

pub trait Filter<Arg, Res>: Send + Sync + 'static {
    fn call(&self, val: Arg, args: Kwargs, ctx: &Context) -> Res;
}

pub trait FilterResult {
    fn into_result(self) -> Value;
}

impl<I: Into<Value>> FilterResult for I {
    fn into_result(self) -> Value {
        self.into()
    }
}

pub trait IntoArg<'v> {
    type Output;

    fn from_value(value: &'v Value) -> Self::Output;
}

impl<T: TryFrom<Value>> IntoArg<'_> for T {
    type Output = T;

    fn from_value(value: &'_ Value) -> Self::Output {
        value.clone().try_into().ok().expect("incorrect argument type")
    }
}

impl<'v> IntoArg<'v> for &str {
    type Output = &'v str;

    fn from_value(value: &'v Value) -> Self::Output {
        if let Value::Str(s) = value { s.as_str() } else { panic!("incorrect argument type") }
    }
}
impl<'v> IntoArg<'v> for &Value {
    type Output = &'v Value;

    fn from_value(value: &'v Value) -> Self::Output {
        value
    }
}

impl<Func, Arg, Res> Filter<Arg, Res> for Func
where
    Func: Fn(Arg, Kwargs, &Context) -> Res + Send + Sync + 'static,
    Arg: for<'v> IntoArg<'v>,
    Res: FilterResult
{
    fn call(&self, val: Arg, args: Kwargs, ctx: &Context) -> Res {
        self(val, args, ctx)
    }
}

pub struct Kwargs {
    args: HashMap<smol_str::SmolStr, Value>,
}

impl Kwargs {
    pub fn new(args: HashMap<smol_str::SmolStr, Value>) -> Self {
        Self { args }
    }
    pub fn get<'v, T: IntoArg<'v, Output = T>>(&'v self, idx: &str) -> Option<T> {
        self.args.get(idx).map(T::from_value)
    }
}

type FilterFunc = dyn Fn(&Value, Kwargs, &Context) -> Value + Send + Sync + 'static;
pub struct StoredFilter {
    func: Arc<FilterFunc>,
}

impl StoredFilter {
    pub fn new<Func, Arg, Res>(f: Func) -> Self
    where
        Func: Filter<Arg, Res> + for<'a> Filter<<Arg as IntoArg<'a>>::Output, Res>,
        Arg: for<'a> IntoArg<'a>,
        Res: FilterResult
    {
        Self { func: Arc::new(move |val: &Value, args: Kwargs, ctx: &Context| -> Value {
            f.call(Arg::from_value(val), args, ctx).into_result()
        }) }
    }

    pub fn call(&self, val: &Value, args: Kwargs, ctx: &Context) -> Value {
        (self.func)(val, args, ctx)
    }
}

pub trait Test<Arg, Res: TestResult>: Send + Sync + 'static {
    fn call(&self, val: Arg, args: Kwargs, ctx: &Context) -> Res;
}
pub trait TestResult {
    fn into_result(self) -> bool;
}

impl TestResult for bool {
    fn into_result(self) -> bool {
        self
    }
}

impl<Func, Arg, Res> Test<Arg, Res> for Func
where
    Func: Fn(Arg, Kwargs, &Context) -> Res + Send + Sync + 'static,
    Arg: for<'v> IntoArg<'v>,
    Res: TestResult
{
    fn call(&self, val: Arg, args: Kwargs, ctx: &Context) -> Res {
        self(val, args, ctx)
    }
}

type TestFunc = dyn Fn(&Value, Kwargs, &Context) -> bool + Send + Sync + 'static;
pub struct StoredTest {
    func: Arc<TestFunc>,
}

impl StoredTest {
    pub fn new<Func, Arg, Res>(f: Func) -> Self
    where
        Func: Test<Arg, Res> + for<'a> Test<<Arg as IntoArg<'a>>::Output, Res>,
        Arg: for<'a> IntoArg<'a>,
        Res: TestResult
    {
        Self { func: Arc::new(move |val: &Value, args: Kwargs, ctx: &Context| -> bool {
            f.call(Arg::from_value(val), args, ctx).into_result()
        }) }
    }

    pub fn call(&self, val: &Value, args: Kwargs, ctx: &Context) -> bool {
        (self.func)(val, args, ctx)
    }
}

pub trait Function<Res: FilterResult>: Send + Sync + 'static {
    fn call(&self, args: Kwargs, ctx: &Context) -> Res;
}

impl<Func, Res> Function<Res> for Func
where
    Func: Fn(Kwargs, &Context) -> Res + Send + Sync + 'static,
    Res: FilterResult
{
    fn call(&self, args: Kwargs, ctx: &Context) -> Res {
        self(args, ctx)
    }
}

type FunctionFunc = dyn Fn(Kwargs, &Context) -> Value + Send + Sync + 'static;
pub struct StoredFunction {
    func: Arc<FunctionFunc>,
}

impl StoredFunction {
    pub fn new<Func, Res>(f: Func) -> Self
    where
        Func: Function<Res>,
        Res: FilterResult
    {
        Self { func: Arc::new(move |args: Kwargs, ctx: &Context| -> Value {
            f.call(args, ctx).into_result()
        }) }
    }

    pub fn call(&self, args: Kwargs, ctx: &Context) -> Value {
        (self.func)(args, ctx)
    }
}
