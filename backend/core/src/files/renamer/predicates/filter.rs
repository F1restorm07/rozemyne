use std::borrow::Cow;

// ripped from `tera`
pub trait Filter {
    fn call(&self, val: &str, args: FilterArgs<'_>) -> String;
    // figure out a way to make `Cow<str>` work in the signature
    // right now, i'm running into an ownership issue when passing the `val` reference
    // into this function (see super::Pipeline::render)
    //
    // however, i cannot just pass in `val` by-value, as there's ownership issues on functions like `trim()`
    // which produce a `&str` that references `val`
    // fn call<'o>(&self, val: &'o Cow<'_, str>, args: FormatterArgs<'_>) -> Cow<'o, str>;
}

impl<Func> Filter for Func
where
    Func: Fn(&str, FilterArgs<'_>) -> String,
    // Func: for<'o> Fn(&'o Cow<'_, str>, FormatterArgs<'_>) -> Cow<'o, str>,
{
    fn call(&self, val: &str, args: FilterArgs<'_>) -> String {
    // fn call<'o>(&self, val: &'o Cow<'_, str>, args: FormatterArgs<'_>) -> Cow<'o, str> {
        self(val, args)
    }
}

// arguments to the filter (may shift to keyword arguments later like `tera`)
// access the arguments by index
pub struct FilterArgs<'args> {
    args: Vec<Cow<'args, str>>
}

impl<'args> FilterArgs<'args> {
    pub fn new(args: Vec<Cow<'args, str>>) -> Self {
        Self { args }
    }
    pub fn get(&self, idx: usize) -> Option<&Cow<'_, str>> {
        self.args.get(idx)
    }
}

pub fn lower(val: &str, _: FilterArgs<'_>) -> String {
    val.to_lowercase().into()
}
pub fn upper(val: &str, _: FilterArgs<'_>) -> String {
    val.to_uppercase().into()
}
pub fn trim_whitespace(val: &str, _: FilterArgs<'_>) -> String {
    val.trim().to_string()
}
