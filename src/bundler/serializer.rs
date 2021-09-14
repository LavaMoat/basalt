//! Convert a `Serialize` implementation into `swc_ecma_ast` nodes.
//!
//! Primitives `i64` and `u64` will be serialized to the `BigInt` type.
//!
use serde::ser::{self, Serialize};
use std::error::Error as StdError;
use std::fmt;

use num_bigint::BigInt as BigIntValue;

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

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    ObjectKeyType,
    Custom(String),
}

impl ser::Error for Error {
    #[cold]
    fn custom<T: fmt::Display>(msg: T) -> Self {
        let msg = format!("{}", msg);
        Self::Custom(msg)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectKeyType => write!(f, "object literal key type is not supported, expecting string, number or bigint"),
            Self::Custom(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
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
    pub fn into_boxed_expr(self) -> Box<Expr> {
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

    fn skip_field(&mut self, _key: &'static str) -> Result<(), Self::Error> {
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
            Value::Number(num) => PropName::Num(num),
            Value::BigInt(num) => PropName::BigInt(num),
            _ => {
                // TODO: convert other key types to use `Map`
                return Err(Error::ObjectKeyType);
            }
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
pub struct SerializeTupleVariant<'a> {
    literal: ObjectLit,
    ser: &'a mut Serializer,
}

impl<'a> ser::SerializeTupleVariant for SerializeTupleVariant<'a> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let prop = self.literal.props.get_mut(0).unwrap();
        let entry = value.serialize(&mut *self.ser)?;

        if let PropOrSpread::Prop(prop) = prop {
            if let Prop::KeyValue(KeyValueProp { value, .. }) = &mut **prop {
                if let Expr::Array(ArrayLit { elems, .. }) = &mut **value {
                    elems.push(Some(ExprOrSpread {
                        spread: None,
                        expr: entry.into_boxed_expr(),
                    }));
                }
            }
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Object(self.literal))
    }
}

#[doc(hidden)]
pub struct SerializeStructVariant<'a> {
    literal: ObjectLit,
    ser: &'a mut Serializer,
}
impl<'a> ser::SerializeStructVariant for SerializeStructVariant<'a> {
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
        let prop = self.literal.props.get_mut(0).unwrap();
        let entry = value.serialize(&mut *self.ser)?;

        if let PropOrSpread::Prop(prop) = prop {
            if let Prop::KeyValue(KeyValueProp { value, .. }) = &mut **prop {
                if let Expr::Object(ObjectLit { props, .. }) = &mut **value {
                    let key = PropName::Str(str_lit(key));
                    let prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(
                        KeyValueProp {
                            key,
                            value: entry.into_boxed_expr(),
                        },
                    )));
                    props.push(prop);
                }
            }
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Object(self.literal))
    }

    fn skip_field(&mut self, _key: &'static str) -> Result<(), Self::Error> {
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
    type SerializeTupleVariant = SerializeTupleVariant<'a>;
    type SerializeMap = SerializeObject<'a>;
    type SerializeStruct = SerializeObject<'a>;
    type SerializeStructVariant = SerializeStructVariant<'a>;

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
        Ok(Value::BigInt(BigInt {
            span: DUMMY_SP,
            value: BigIntValue::from(v),
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
        Ok(Value::BigInt(BigInt {
            span: DUMMY_SP,
            value: BigIntValue::from(v),
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

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        use ser::SerializeMap;
        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
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

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeObject {
            ser: self,
            literal: ObjectLit {
                span: DUMMY_SP,
                props: {
                    if let Some(size) = len {
                        Vec::with_capacity(size)
                    } else {
                        vec![]
                    }
                },
            },
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        use ser::SerializeSeq;

        let seq = self.serialize_seq(Some(len))?;
        let val = seq.end()?;

        let key = PropName::Str(str_lit(variant));
        let prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key,
            value: val.into_boxed_expr(),
        })));

        Ok(SerializeTupleVariant {
            ser: self,
            literal: ObjectLit {
                span: DUMMY_SP,
                props: vec![prop],
            },
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        use ser::SerializeMap;

        let map = self.serialize_map(Some(len))?;
        let val = map.end()?;

        let key = PropName::Str(str_lit(variant));
        let prop = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key,
            value: val.into_boxed_expr(),
        })));

        Ok(SerializeStructVariant {
            ser: self,
            literal: ObjectLit {
                span: DUMMY_SP,
                props: vec![prop],
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serde::Serialize;

    use num_bigint::BigInt as BigIntValue;

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

        let i_8 = 2i8;
        let value = i_8.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 2f64,
            }),
            value
        );

        let i_16 = 4i16;
        let value = i_16.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 4f64,
            }),
            value
        );

        let i_32 = 8i32;
        let value = i_32.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 8f64,
            }),
            value
        );

        let i_64 = 16i64;
        let value = i_64.serialize(&mut serializer)?;
        assert_eq!(
            Value::BigInt(BigInt {
                span: DUMMY_SP,
                value: BigIntValue::from(i_64),
            }),
            value
        );

        let u_8 = 32u8;
        let value = u_8.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 32f64,
            }),
            value
        );

        let u_16 = 64u16;
        let value = u_16.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 64f64,
            }),
            value
        );

        let u_32 = 128u32;
        let value = u_32.serialize(&mut serializer)?;
        assert_eq!(
            Value::Number(Number {
                span: DUMMY_SP,
                value: 128f64,
            }),
            value
        );

        let u_64 = 256u64;
        let value = u_64.serialize(&mut serializer)?;
        assert_eq!(
            Value::BigInt(BigInt {
                span: DUMMY_SP,
                value: BigIntValue::from(u_64),
            }),
            value
        );

        // i128 / u128 ?

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

    #[test]
    fn serialize_unit() -> Result<()> {
        let mut serializer = Serializer {};
        let unit = ();
        let value = unit.serialize(&mut serializer)?;
        assert_eq!(Value::Null(Null { span: DUMMY_SP }), value);
        Ok(())
    }

    #[derive(Serialize)]
    struct Marker {
        marker: std::marker::PhantomData<bool>,
    }

    #[test]
    fn serialize_unit_struct() -> Result<()> {
        let mut serializer = Serializer {};

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

    #[derive(Serialize)]
    enum UnitVariant {
        #[serde(rename = "frozen")]
        Frozen,
    }

    #[derive(Serialize)]
    struct EnvPolicy {
        env: UnitVariant,
    }

    #[test]
    fn serialize_unit_variant() -> Result<()> {
        let mut serializer = Serializer {};
        let variant = EnvPolicy {
            env: UnitVariant::Frozen,
        };
        let value = variant.serialize(&mut serializer)?;
        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Str(str_lit("env")),
                    value: Box::new(Expr::Lit(Lit::Str(str_lit("frozen")))),
                },
            )))],
        });

        assert_eq!(expected, value);
        Ok(())
    }

    #[derive(Serialize)]
    struct Millimeters(u8);

    #[test]
    fn serialize_newtype_struct() -> Result<()> {
        let mut serializer = Serializer {};
        let mm = Millimeters(0);
        let value = mm.serialize(&mut serializer)?;
        let expected = Value::Number(Number {
            span: DUMMY_SP,
            value: 0f64,
        });

        assert_eq!(expected, value);
        Ok(())
    }

    #[derive(Serialize)]
    enum Length {
        Millimeters(u8),
    }

    #[test]
    fn serialize_newtype_variant() -> Result<()> {
        let mut serializer = Serializer {};
        let mm = Length::Millimeters(0);
        let value = mm.serialize(&mut serializer)?;
        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Str(str_lit("Millimeters")),
                    value: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 0f64,
                    }))),
                },
            )))],
        });

        assert_eq!(expected, value);
        Ok(())
    }

    #[test]
    fn serialize_seq() -> Result<()> {
        let mut serializer = Serializer {};
        let list = vec!["1", "2"];
        let value = list.serialize(&mut serializer)?;
        let expected = Value::Array(ArrayLit {
            span: DUMMY_SP,
            elems: vec![
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(str_lit("1")))),
                }),
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(str_lit("2")))),
                }),
            ],
        });

        assert_eq!(expected, value);
        Ok(())
    }

    #[test]
    fn serialize_tuple() -> Result<()> {
        let mut serializer = Serializer {};
        let list = ("3", "4");
        let value = list.serialize(&mut serializer)?;
        let expected = Value::Array(ArrayLit {
            span: DUMMY_SP,
            elems: vec![
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(str_lit("3")))),
                }),
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(str_lit("4")))),
                }),
            ],
        });

        assert_eq!(expected, value);
        Ok(())
    }

    #[derive(Serialize)]
    struct Rgb(u8, u8, u8);

    #[test]
    fn serialize_tuple_struct() -> Result<()> {
        let mut serializer = Serializer {};
        let color = Rgb(0xff, 0x66, 0x00);
        let value = color.serialize(&mut serializer)?;
        let expected = Value::Array(ArrayLit {
            span: DUMMY_SP,
            elems: vec![
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 255f64,
                    }))),
                }),
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 102f64,
                    }))),
                }),
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 0f64,
                    }))),
                }),
            ],
        });

        assert_eq!(expected, value);
        Ok(())
    }

    #[derive(Serialize)]
    enum TupleVariant {
        T(String, String),
    }

    #[test]
    fn serialize_tuple_variant() -> Result<()> {
        let mut serializer = Serializer {};
        let variant = TupleVariant::T("1".into(), "2".into());
        let value = variant.serialize(&mut serializer)?;
        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Str(str_lit("T")),
                    value: Box::new(Expr::Array(ArrayLit {
                        span: DUMMY_SP,
                        elems: vec![
                            Some(ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Lit(Lit::Str(str_lit(
                                    "1",
                                )))),
                            }),
                            Some(ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Lit(Lit::Str(str_lit(
                                    "2",
                                )))),
                            }),
                        ],
                    })),
                },
            )))],
        });

        assert_eq!(expected, value);
        Ok(())
    }

    #[test]
    fn serialize_map() -> Result<()> {
        use std::collections::HashMap;
        let mut serializer = Serializer {};

        // String keys
        let mut map = HashMap::new();
        map.insert("foo", "bar");
        let value = map.serialize(&mut serializer)?;
        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Str(str_lit("foo")),
                    value: Box::new(Expr::Lit(Lit::Str(str_lit("bar")))),
                },
            )))],
        });
        assert_eq!(expected, value);

        // Number keys
        let mut map = HashMap::new();
        map.insert(4, 8);
        let value = map.serialize(&mut serializer)?;
        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Num(Number {
                        span: DUMMY_SP,
                        value: 4f64,
                    }),
                    value: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 8f64,
                    }))),
                },
            )))],
        });
        assert_eq!(expected, value);

        // BigInt keys
        let mut map = HashMap::new();
        map.insert(4u64, 8u64);
        let value = map.serialize(&mut serializer)?;
        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::BigInt(BigInt {
                        span: DUMMY_SP,
                        value: BigIntValue::from(4u64),
                    }),
                    value: Box::new(Expr::Lit(Lit::BigInt(BigInt {
                        span: DUMMY_SP,
                        value: BigIntValue::from(8u64),
                    }))),
                },
            )))],
        });
        assert_eq!(expected, value);

        // Unsupported key type for object literal
        let mut map = HashMap::new();
        map.insert((1, 2), 3);
        let result = map.serialize(&mut serializer);
        println!("{:#?}", result);

        Ok(())
    }

    #[derive(Serialize)]
    struct Container {
        message: String,
        amount: u8,
        flag: bool,
    }

    #[test]
    fn serialize_struct() -> Result<()> {
        let mut serializer = Serializer {};
        let con = Container {
            message: "mock".into(),
            amount: 32,
            flag: true,
        };
        let value = con.serialize(&mut serializer)?;

        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Str(str_lit("message")),
                    value: Box::new(Expr::Lit(Lit::Str(str_lit("mock")))),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Str(str_lit("amount")),
                    value: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: 32f64,
                    }))),
                }))),
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Str(str_lit("flag")),
                    value: Box::new(Expr::Lit(Lit::Bool(Bool {
                        span: DUMMY_SP,
                        value: true,
                    }))),
                }))),
            ],
        });
        assert_eq!(expected, value);

        Ok(())
    }

    #[derive(Serialize)]
    enum StructVariant {
        Rgb { r: String, g: String, b: String },
    }

    #[test]
    fn serialize_struct_variant() -> Result<()> {
        let mut serializer = Serializer {};
        let variant = StructVariant::Rgb {
            r: "ff".into(),
            g: "66".into(),
            b: "00".into(),
        };
        let value = variant.serialize(&mut serializer)?;
        let expected = Value::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(
                KeyValueProp {
                    key: PropName::Str(str_lit("Rgb")),
                    value: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props: vec![
                            PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                KeyValueProp {
                                    key: PropName::Str(str_lit("r")),
                                    value: Box::new(Expr::Lit(Lit::Str(
                                        str_lit("ff"),
                                    ))),
                                },
                            ))),
                            PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                KeyValueProp {
                                    key: PropName::Str(str_lit("g")),
                                    value: Box::new(Expr::Lit(Lit::Str(
                                        str_lit("66"),
                                    ))),
                                },
                            ))),
                            PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                KeyValueProp {
                                    key: PropName::Str(str_lit("b")),
                                    value: Box::new(Expr::Lit(Lit::Str(
                                        str_lit("00"),
                                    ))),
                                },
                            ))),
                        ],
                    })),
                },
            )))],
        });

        assert_eq!(expected, value);
        Ok(())
    }
}
