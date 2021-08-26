//! Convert a `Serialize` implementation into AST nodes.
//!
//! Primitives `i64` and `u64` will be serialized to the `BigInt` type.
//!
use serde::ser::{self, Serialize};
use std::convert::TryInto;
use std::error::Error as StdError;
use std::fmt;

use num_bigint::{BigInt as BigIntValue, Sign};

use swc_common::DUMMY_SP;
use swc_ecma_ast::*;

#[inline]
fn str_lit(value: &str) -> Str {
    Str {
        span: DUMMY_SP,
        value: value.into(),
        has_escape: value.contains("\n"),
        kind: StrKind::Normal {
            contains_quote: false,
        },
    }
}

#[derive(Debug)]
pub enum Error {
    Utf8(std::str::Utf8Error),
}

impl ser::Error for Error {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        todo!()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Utf8(e) => Some(e),
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

/// Enumeration of serialized values.
#[derive(Debug, Eq, PartialEq)]
pub enum Value {
    Object(ObjectLit),
    Array(ArrayLit),
    String(Str),
    Bool(Bool),
    Number(Number),
    BigInt(BigInt),
    Null(Null),
}

impl Value {
    fn into_boxed_expr(self) -> Box<Expr> {
        match self {
            Value::Object(obj) => Box::new(Expr::Object(obj)),
            Value::Array(arr) => Box::new(Expr::Array(arr)),
            Value::String(val) => Box::new(Expr::Lit(Lit::Str(val))),
            Value::Bool(flag) => Box::new(Expr::Lit(Lit::Bool(flag))),
            Value::Number(num) => Box::new(Expr::Lit(Lit::Num(num))),
            Value::BigInt(num) => Box::new(Expr::Lit(Lit::BigInt(num))),
            Value::Null(null) => Box::new(Expr::Lit(Lit::Null(null))),
        }
    }
}

/// Serialize to an array literal.
#[doc(hidden)]
pub struct SerializeArray<'a> {
    literal: ArrayLit,
    ser: &'a mut Serializer,
}

impl<'a> ser::SerializeSeq for SerializeArray<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value = value.serialize(&mut *self.ser)?;
        let value = value.into_boxed_expr();

        self.literal.elems.push(Some(ExprOrSpread {
            spread: None,
            expr: value,
        }));
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Array(self.literal))
    }
}

impl<'a> ser::SerializeTuple for SerializeArray<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for SerializeArray<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub struct SerializeTupleVariant;
impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

/// Serialize to an object literal.
#[doc(hidden)]
pub struct SerializeObject<'a> {
    literal: ObjectLit,
    ser: &'a mut Serializer,
}
impl<'a> ser::SerializeStruct for SerializeObject<'a> {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = PropName::Str(str_lit(key));
        let value = value.serialize(&mut *self.ser)?;
        self.literal
            .props
            .push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key,
                value: value.into_boxed_expr(),
            }))));
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Object(self.literal))
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for SerializeObject<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn serialize_value<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Object(self.literal))
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        let key = key.serialize(&mut *self.ser)?;
        let key = match key {
            Value::String(lit) => PropName::Str(lit),
            _ => todo!("Non-string map keys are not supported yet"),
        };

        let value = value.serialize(&mut *self.ser)?;

        self.literal
            .props
            .push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key,
                value: value.into_boxed_expr(),
            }))));

        Ok(())
    }
}

#[doc(hidden)]
pub struct SerializeStructVariant;
impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Value;
    type Error = Error;
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Perform serialization into AST nodes.
pub struct Serializer;

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeArray<'a>;
    type SerializeTuple = SerializeArray<'a>;
    type SerializeTupleStruct = SerializeArray<'a>;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeObject<'a>;
    type SerializeStruct = SerializeObject<'a>;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(Bool {
            span: DUMMY_SP,
            value: v,
        }))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        let digits = v.to_le_bytes();
        let value = BigIntValue::from_signed_bytes_le(&digits);
        Ok(Value::BigInt(BigInt {
            span: DUMMY_SP,
            value,
        }))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        let digits = v.to_le_bytes();
        let value = BigIntValue::from_signed_bytes_le(&digits);
        Ok(Value::BigInt(BigInt {
            span: DUMMY_SP,
            value,
        }))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Number(Number {
            span: DUMMY_SP,
            value: v,
        }))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        // A char encoded as UTF-8 takes 4 bytes at most.
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(str_lit(v)))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        use ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null(Null { span: DUMMY_SP }))
    }

    fn serialize_unit_struct(
        self,
        _name: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeArray {
            ser: self,
            literal: ArrayLit {
                span: DUMMY_SP,
                elems: {
                    if let Some(size) = len {
                        Vec::with_capacity(size)
                    } else {
                        vec![]
                    }
                },
            },
        })
    }

    fn serialize_tuple(
        self,
        len: usize,
    ) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeObject {
            ser: self,
            literal: ObjectLit {
                span: DUMMY_SP,
                props: vec![],
            },
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serde::Serialize;

    use swc_common::DUMMY_SP;
    use swc_ecma_ast::*;

    #[test]
    fn serialize_primitives() -> Result<()> {
        let mut serializer = Serializer {};

        let ch = 'c';
        let value = ch.serialize(&mut serializer)?;
        assert_eq!(Value::String(str_lit("c")), value);

        let boolean = true;
        let value = boolean.serialize(&mut serializer)?;
        assert_eq!(
            Value::Bool(Bool {
                span: DUMMY_SP,
                value: true
            }),
            value
        );

        let i_8 = 42i8;
        let value = i_8.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 42f64,
            }),
            value
        );

        let i_16 = 42i16;
        let value = i_16.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 42f64,
            }),
            value
        );

        let i_32 = 42i32;
        let value = i_32.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 42f64,
            }),
            value
        );

        let i_64 = 42i64;
        let value = i_64.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 42f64,
            }),
            value
        );

        let float32 = 3.14f64;
        let value = float32.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 3.14,
            }),
            value
        );

        let float64 = 3.14f64;
        let value = float64.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 3.14,
            }),
            value
        );

        Ok(())
    }

    #[test]
    fn serialize_string() -> Result<()> {
        let mut serializer = Serializer {};
        let string = String::from("mock");
        let value = string.serialize(&mut serializer)?;
        assert_eq!(Value::String(str_lit("mock")), value);
        Ok(())
    }

    #[test]
    fn serialize_byte_array() -> Result<()> {
        let mut serializer = Serializer {};
        let bytes = [0u8, 0u8];
        let value = (&bytes).serialize(&mut serializer)?;
        assert_eq!(
            Value::Array(ArrayLit {
                span: DUMMY_SP,
                elems: vec![
                    Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Lit(Lit::Num(Number {
                            span: DUMMY_SP,
                            value: 0.0
                        }))),
                    }),
                    Some(ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Lit(Lit::Num(Number {
                            span: DUMMY_SP,
                            value: 0.0
                        }))),
                    }),
                ]
            }),
            value
        );

        Ok(())
    }

    #[test]
    fn serialize_option() -> Result<()> {
        let mut serializer = Serializer {};

        let none: Option<()> = None;
        let value = none.serialize(&mut serializer)?;
        assert_eq!(Value::Null(Null { span: DUMMY_SP }), value);

        let some: Option<bool> = Some(true);
        let value = some.serialize(&mut serializer)?;
        assert_eq!(
            Value::Bool(Bool {
                span: DUMMY_SP,
                value: true
            }),
            value
        );

        Ok(())
    }

    #[derive(Serialize)]
    struct Marker {
        marker: std::marker::PhantomData<bool>,
    }

    #[test]
    fn serialize_unit() -> Result<()> {
        let mut serializer = Serializer {};

        let unit = ();
        let value = unit.serialize(&mut serializer)?;
        assert_eq!(Value::Null(Null { span: DUMMY_SP }), value);

        let unit_struct = Marker {
            marker: std::marker::PhantomData,
        };

        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Str(str_lit("marker")),
                    value: Box::new(Expr::Lit(Lit::Null(Null {
                        span: DUMMY_SP,
                    }))),
                },
            )))],
        });

        let value = unit_struct.serialize(&mut serializer)?;
        assert_eq!(expected, value);

        Ok(())
    }
}
