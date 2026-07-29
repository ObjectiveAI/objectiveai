//! Starlark expression evaluation engine.
//! Provides a sandboxed Starlark runtime for evaluating expressions.
//! Variables `input`, `output`, and `map` are injected into the global scope.

use starlark::environment::{Globals, GlobalsBuilder, Module};
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::dict::DictRef;
use starlark::values::float::UnpackFloat;
use starlark::values::list::ListRef;
use starlark::values::{Heap, UnpackValue, Value as StarlarkValue};
use std::sync::LazyLock;

use super::{ExpressionError, OneOrMany};

/// Global Starlark globals with custom functions.
pub static STARLARK_GLOBALS: LazyLock<Globals> = LazyLock::new(|| {
    let mut builder = GlobalsBuilder::standard();
    register_custom_functions(&mut builder);
    builder.build()
});

/// Register custom functions that extend Starlark's standard library.
#[starlark_module]
fn register_custom_functions(builder: &mut GlobalsBuilder) {
    /// Sum of a list of numbers. Returns 0 for empty list.
    fn sum<'v>(
        #[starlark(require = pos)] xs: &ListRef<'v>,
    ) -> starlark::Result<f64> {
        let mut total = 0.0;
        for x in xs.iter() {
            let n = UnpackFloat::unpack_value(x)
                .map_err(|e| {
                    starlark::Error::new_other(anyhow::anyhow!("{}", e))
                })?
                .ok_or_else(|| {
                    starlark::Error::new_other(anyhow::anyhow!(
                        "sum: expected number, got {}",
                        x.get_type()
                    ))
                })?;
            total += n.0;
        }
        Ok(total)
    }

    /// Absolute value of a number.
    fn abs(#[starlark(require = pos)] x: UnpackFloat) -> starlark::Result<f64> {
        Ok(x.0.abs())
    }

    /// Convert to float.
    fn float(
        #[starlark(require = pos)] x: UnpackFloat,
    ) -> starlark::Result<f64> {
        Ok(x.0)
    }

    /// Round a number to the nearest integer.
    fn round(
        #[starlark(require = pos)] x: UnpackFloat,
    ) -> starlark::Result<i64> {
        Ok(x.0.round() as i64)
    }
}

/// Trait for direct conversion to Starlark values (bypassing serde_json).
pub trait ToStarlarkValue {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v>;
}

impl ToStarlarkValue for str {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        heap.alloc_str(self).to_value()
    }
}

impl ToStarlarkValue for String {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        heap.alloc_str(self).to_value()
    }
}

impl ToStarlarkValue for i32 {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        heap.alloc(*self as i64)
    }
}

impl ToStarlarkValue for i64 {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        heap.alloc(*self)
    }
}

impl ToStarlarkValue for u32 {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        heap.alloc(*self as i64)
    }
}

impl ToStarlarkValue for u64 {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        heap.alloc(*self as i64)
    }
}

impl ToStarlarkValue for f64 {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        heap.alloc(*self)
    }
}

impl ToStarlarkValue for bool {
    fn to_starlark_value<'v>(&self, _heap: &'v Heap) -> StarlarkValue<'v> {
        StarlarkValue::new_bool(*self)
    }
}

impl ToStarlarkValue for rust_decimal::Decimal {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        use rust_decimal::prelude::ToPrimitive;
        heap.alloc(self.to_f64().unwrap_or(0.0))
    }
}

impl<T: ToStarlarkValue> ToStarlarkValue for Vec<T> {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        let items: Vec<StarlarkValue> =
            self.iter().map(|v| v.to_starlark_value(heap)).collect();
        heap.alloc(items)
    }
}

impl<T: ToStarlarkValue> ToStarlarkValue for [T] {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        let items: Vec<StarlarkValue> =
            self.iter().map(|v| v.to_starlark_value(heap)).collect();
        heap.alloc(items)
    }
}

impl<T: ToStarlarkValue> ToStarlarkValue for Option<T> {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        match self {
            Some(v) => v.to_starlark_value(heap),
            None => StarlarkValue::new_none(),
        }
    }
}

impl<T: ToStarlarkValue> ToStarlarkValue for indexmap::IndexMap<String, T> {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        let pairs: Vec<(&str, StarlarkValue)> = self
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_starlark_value(heap)))
            .collect();
        heap.alloc(starlark::values::dict::AllocDict(pairs))
    }
}

impl ToStarlarkValue for serde_json::Value {
    fn to_starlark_value<'v>(&self, heap: &'v Heap) -> StarlarkValue<'v> {
        match self {
            serde_json::Value::Null => StarlarkValue::new_none(),
            serde_json::Value::Bool(b) => b.to_starlark_value(heap),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i.to_starlark_value(heap)
                } else {
                    n.as_f64().unwrap_or(0.0).to_starlark_value(heap)
                }
            }
            serde_json::Value::String(s) => s.to_starlark_value(heap),
            serde_json::Value::Array(arr) => {
                let items: Vec<StarlarkValue> =
                    arr.iter().map(|v| v.to_starlark_value(heap)).collect();
                heap.alloc(items)
            }
            serde_json::Value::Object(obj) => {
                let pairs: Vec<(&str, StarlarkValue)> = obj
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.to_starlark_value(heap)))
                    .collect();
                heap.alloc(starlark::values::dict::AllocDict(pairs))
            }
        }
    }
}
/// Trait for converting a Starlark runtime value into a Rust type.
///
/// Used by [`Expression`](super::Expression) to compile Starlark expressions
/// directly from `starlark::values::Value` to the target type.
pub trait FromStarlarkValue: Sized {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError>;
}

// Primitives and common types
impl FromStarlarkValue for rust_decimal::Decimal {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if let Ok(Some(i)) = i64::unpack_value(*value) {
            return Ok(rust_decimal::Decimal::from(i));
        }
        if let Ok(Some(UnpackFloat(f))) = UnpackFloat::unpack_value(*value) {
            return rust_decimal::Decimal::try_from(f).map_err(|e| {
                ExpressionError::StarlarkConversionError(format!(
                    "Decimal: {}",
                    e
                ))
            });
        }
        Err(ExpressionError::StarlarkConversionError(
            "Decimal: expected number".into(),
        ))
    }
}

impl FromStarlarkValue for bool {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        bool::unpack_value(*value)
            .map_err(|e| {
                ExpressionError::StarlarkConversionError(e.to_string())
            })
            .and_then(|o| {
                o.ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "expected bool".to_string(),
                    )
                })
            })
    }
}

impl FromStarlarkValue for i64 {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        i64::unpack_value(*value)
            .map_err(|e| {
                ExpressionError::StarlarkConversionError(e.to_string())
            })
            .and_then(|o| {
                o.ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "expected int".to_string(),
                    )
                })
            })
    }
}

impl FromStarlarkValue for u64 {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let i = i64::unpack_value(*value)
            .map_err(|e| {
                ExpressionError::StarlarkConversionError(e.to_string())
            })?
            .ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "expected int".to_string(),
                )
            })?;
        if i < 0 {
            return Err(ExpressionError::StarlarkConversionError(
                "expected non-negative int".to_string(),
            ));
        }
        Ok(i as u64)
    }
}

impl FromStarlarkValue for f64 {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if let Ok(Some(i)) = i64::unpack_value(*value) {
            return Ok(i as f64);
        }
        UnpackFloat::unpack_value(*value)
            .map_err(|e| {
                ExpressionError::StarlarkConversionError(e.to_string())
            })
            .and_then(|o| {
                o.ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "expected number".to_string(),
                    )
                })
            })
            .map(|u| u.0)
    }
}

impl FromStarlarkValue for String {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        <&str as UnpackValue>::unpack_value(*value)
            .map_err(|e| {
                ExpressionError::StarlarkConversionError(e.to_string())
            })?
            .map(|s| s.to_owned())
            .ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "expected string".to_string(),
                )
            })
    }
}

impl<T: FromStarlarkValue> FromStarlarkValue for Option<T> {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if value.is_none() {
            return Ok(None);
        }
        T::from_starlark_value(value).map(Some)
    }
}

impl<T: FromStarlarkValue> FromStarlarkValue for Vec<T> {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let list = ListRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "expected list".to_string(),
            )
        })?;
        let mut out = Vec::with_capacity(list.len());
        for v in list.iter() {
            out.push(T::from_starlark_value(&v)?);
        }
        Ok(out)
    }
}

impl<V: FromStarlarkValue> FromStarlarkValue for indexmap::IndexMap<String, V> {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = DictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "expected dict".to_string(),
            )
        })?;
        let mut map = indexmap::IndexMap::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "expected string key".to_string(),
                    )
                })?
                .to_owned();
            map.insert(key, V::from_starlark_value(&v)?);
        }
        Ok(map)
    }
}

impl FromStarlarkValue for serde_json::Value {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if value.is_none() {
            return Ok(serde_json::Value::Null);
        }
        if let Ok(Some(b)) = bool::unpack_value(*value) {
            return Ok(serde_json::Value::Bool(b));
        }
        if let Ok(Some(i)) = i64::unpack_value(*value) {
            return Ok(serde_json::Value::Number(serde_json::Number::from(i)));
        }
        if let Ok(Some(UnpackFloat(f))) = UnpackFloat::unpack_value(*value) {
            if let Some(n) = serde_json::Number::from_f64(f) {
                return Ok(serde_json::Value::Number(n));
            }
        }
        if let Ok(Some(s)) = <&str as UnpackValue>::unpack_value(*value) {
            return Ok(serde_json::Value::String(s.to_owned()));
        }
        if let Some(list) = ListRef::from_value(*value) {
            let mut items = Vec::with_capacity(list.len());
            for v in list.iter() {
                items.push(serde_json::Value::from_starlark_value(&v)?);
            }
            return Ok(serde_json::Value::Array(items));
        }
        if let Some(dict) = DictRef::from_value(*value) {
            let mut obj = serde_json::Map::with_capacity(dict.len());
            for (k, v) in dict.iter() {
                let key = <&str as UnpackValue>::unpack_value(k)
                    .map_err(|e| {
                        ExpressionError::StarlarkConversionError(e.to_string())
                    })?
                    .ok_or_else(|| {
                        ExpressionError::StarlarkConversionError(
                            "expected string key".to_string(),
                        )
                    })?;
                obj.insert(
                    key.to_owned(),
                    serde_json::Value::from_starlark_value(&v)?,
                );
            }
            return Ok(serde_json::Value::Object(obj));
        }
        Err(ExpressionError::StarlarkConversionError(format!(
            "unsupported type: {}",
            value.get_type()
        )))
    }
}

/// Run a Starlark expression and pass the result (while still valid) to `f`.
pub(crate) fn with_eval_result<F, R>(
    code: &str,
    params: &super::Params,
    f: F,
) -> Result<R, ExpressionError>
where
    F: FnOnce(&StarlarkValue) -> Result<R, ExpressionError>,
{
    let module = Module::new();
    {
        let heap = module.heap();
        match params {
            super::Params::Owned(owned) => {
                module.set("input", owned.input.to_starlark_value(heap));
                module.set(
                    "output",
                    owned
                        .output
                        .as_ref()
                        .map_or(StarlarkValue::new_none(), |o| {
                            o.to_starlark_value(heap)
                        }),
                );
                module.set(
                    "map",
                    owned.map.map_or(StarlarkValue::new_none(), |m| {
                        heap.alloc(m as i64)
                    }),
                );
            }
            super::Params::Ref(r) => {
                module.set("input", r.input.to_starlark_value(heap));
                module.set(
                    "output",
                    r.output.as_ref().map_or(StarlarkValue::new_none(), |o| {
                        o.to_starlark_value(heap)
                    }),
                );
                module.set(
                    "map",
                    r.map.map_or(StarlarkValue::new_none(), |m| {
                        heap.alloc(m as i64)
                    }),
                );
            }
        }
    }
    let ast =
        AstModule::parse("expression", code.to_string(), &Dialect::Extended)
            .map_err(|e| ExpressionError::StarlarkParseError(e.to_string()))?;
    let mut eval = Evaluator::new(&module);
    let result = eval
        .eval_module(ast, &STARLARK_GLOBALS)
        .map_err(|e| ExpressionError::StarlarkEvalError(e.to_string()))?;
    f(&result)
}

fn svalue_to_one_or_many<T: FromStarlarkValue>(
    value: &StarlarkValue,
) -> Result<OneOrMany<T>, ExpressionError> {
    if value.is_none() {
        return Ok(OneOrMany::Many(Vec::new()));
    }
    if let Ok(v) = T::from_starlark_value(value) {
        return Ok(OneOrMany::One(v));
    }
    if let Some(list) = ListRef::from_value(*value) {
        let mut vs: Vec<T> = Vec::with_capacity(list.len());
        for v in list.iter() {
            if let Some(opt) = Option::<T>::from_starlark_value(&v)? {
                vs.push(opt);
            }
        }
        return Ok(if vs.is_empty() {
            OneOrMany::Many(Vec::new())
        } else if vs.len() == 1 {
            OneOrMany::One(vs.into_iter().next().unwrap())
        } else {
            OneOrMany::Many(vs)
        });
    }
    match Option::<T>::from_starlark_value(value)? {
        Some(v) => Ok(OneOrMany::One(v)),
        None => Ok(OneOrMany::Many(Vec::new())),
    }
}

impl<T: FromStarlarkValue> FromStarlarkValue for OneOrMany<T> {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        svalue_to_one_or_many(value)
    }
}

impl<T: FromStarlarkValue> OneOrMany<T> {
    pub fn from_starlark(
        code: &str,
        params: &super::Params,
    ) -> Result<Self, ExpressionError> {
        with_eval_result(code, params, svalue_to_one_or_many)
    }
}
