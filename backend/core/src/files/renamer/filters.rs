use backend_types::renamer::{Context, Value};
use backend_types::renamer::filter::Kwargs;

pub fn lower(val: &str, _: Kwargs, _: &Context) -> String {
    val.to_lowercase()
}
pub fn upper(val: &str, _: Kwargs, _: &Context) -> String {
    val.to_uppercase()
}
pub fn trim_whitespace(val: &str, _: Kwargs, _: &Context) -> String {
    val.trim().to_string()
}
pub fn trim(val: &str, args: Kwargs, _: &Context) -> String {
    let pat = args.get::<char>("pat").unwrap();
    val.trim_matches(pat).to_string()
}

pub fn prepend(val: &str, args: Kwargs, _: &Context) -> String {
    let item = args.get::<&str>("text").unwrap();
    format!("{item}{val}")
}
pub fn append(val: &str, args: Kwargs, _: &Context) -> String {
    let item = args.get::<&str>("text").unwrap();
    format!("{val}{item}")
}

// @todo: support pushing a list of values
pub fn push(mut val: Vec<Value>, args: Kwargs, _: &Context) -> Vec<Value> {
    let item = args.get::<Value>("item").unwrap();
    if let Value::Array(v) = item {
        val.extend_from_slice(&v);
    } else {
        val.push(item);
    }
    val
}
pub fn insert(mut val: Vec<Value>, args: Kwargs, _: &Context) -> Vec<Value> {
    let item = args.get::<Value>("item").unwrap();
    let idx = args.get::<u64>("idx").unwrap() as usize;
    if let Value::Array(v) = item {
        val.splice(idx..idx, v);
    } else {
        val.insert(idx, item);
    }
    val
}

pub fn kebab_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsKebabCase(val).to_string()
}
pub fn lower_camel_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsLowerCamelCase(val).to_string()
}
pub fn pascal_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsPascalCase(val).to_string()
}
pub fn shouty_kebab_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsShoutyKebabCase(val).to_string()
}
pub fn shouty_snake_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsShoutySnakeCase(val).to_string()
}
pub fn snake_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsSnakeCase(val).to_string()
}
pub fn title_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsTitleCase(val).to_string()
}
pub fn train_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsTrainCase(val).to_string()
}
pub fn upper_camel_case(val: &str, _: Kwargs, _: &Context) -> String {
    heck::AsUpperCamelCase(val).to_string()
}

pub fn unicode_normalize(val: &Value, _: Kwargs, _: &Context) -> String {
    use unicode_normalization::UnicodeNormalization;
    match val {
        Value::Char(v) => v.nfc().collect(),
        Value::Str(v) => v.nfc().collect(),
        v => panic!("unsupported argument for unicode normalization: {v:?}")
    }
}

// @todo: experiment with positional format values
pub fn format(val: &Value, args: Kwargs, ctx: &Context) -> String {
    use interpolator::Formattable;
    let val = match val {
        Value::Int(v) => Formattable::integer(v),
        Value::Uint(v) => Formattable::integer(v),
        Value::Float(v) => Formattable::float(v),
        Value::Bool(v) => Formattable::display(v),
        Value::Char(v) => Formattable::display(v),
        Value::Str(v) => Formattable::display(v),
        v => panic!("unformattable value: {v:?}"),
    };
    let mut ctx = ctx.items().iter().map(|(p, v)| {
        let val = match v {
        Value::Int(v) => Formattable::integer(v),
        Value::Uint(v) => Formattable::integer(v),
        Value::Float(v) => Formattable::float(v),
        Value::Bool(v) => Formattable::display(v),
        Value::Char(v) => Formattable::display(v),
        Value::Str(v) => Formattable::display(v),
        v => panic!("unformattable value: {v:?}"),
        };
        (format!("{p}"), val)
    }).collect::<std::collections::HashMap<_, _>>();
    // @todo: bikeshed this
    ctx.insert("input".to_string(), val);

    interpolator::format(args.get::<&str>("fmt").unwrap(), &ctx).unwrap()
}

pub fn is_int(val: &Value, _: Kwargs, _: &Context) -> bool {
    matches!(val, Value::Int(_))
}
pub fn is_unsigned_int(val: &Value, _: Kwargs, _: &Context) -> bool {
    matches!(val, Value::Uint(_))
}
pub fn is_float(val: &Value, _: Kwargs, _: &Context) -> bool {
    matches!(val, Value::Float(_))
}
pub fn is_char(val: &Value, _: Kwargs, _: &Context) -> bool {
    matches!(val, Value::Char(_))
}
pub fn is_bool(val: &Value, _: Kwargs, _: &Context) -> bool {
    matches!(val, Value::Bool(_))
}

pub fn is_odd(val: &Value, _: Kwargs, _: &Context) -> bool {
    match val {
        Value::Int(v) => v % 2 == 0,
        Value::Uint(v) => v % 2 == 0,
        Value::Float(v) => v % 2.0 == 0.0,
        _ => false
    }
}

pub fn any(val: &Value, args: Kwargs, _: &Context) -> bool {
    let set = args.get::<Vec<Value>>("set").unwrap();
    set.contains(val)
}
pub fn all(val: &Value, args: Kwargs, _: &Context) -> bool {
    let set = args.get::<Vec<Value>>("set").unwrap();
    set.iter().all(|x| x == val)
}

// i may refactor this to a test
pub fn exists(args: Kwargs, ctx: &Context) -> bool {
    let val = args.get::<&str>("val").unwrap();
    ctx.get(val).is_some()
}
