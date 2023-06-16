// The implementation of path deserialization was started from the corresponding
// implementation of `Path` in `axum`.
// We significantly restricted the range of supported types and adjusted the deserializer
// to be zero-copy.
//
// Copyright (c) 2019 Axum Contributors
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
use std::any::type_name;
use std::borrow::Cow;

use serde::{
    de::{self, DeserializeSeed, EnumAccess, Error, MapAccess, VariantAccess, Visitor},
    forward_to_deserialize_any, Deserializer,
};

use crate::extract::route::errors::{ErrorKind, PathDeserializationError};

macro_rules! unsupported_type {
    ($trait_fn:ident) => {
        fn $trait_fn<V>(self, _: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'request>,
        {
            Err(PathDeserializationError::unsupported_type(type_name::<
                V::Value,
            >()))
        }
    };
}

pub(super) struct PathDeserializer<'server, 'request, 'de> {
    url_params: &'de [(&'server str, Cow<'request, str>)],
}

impl<'server, 'request, 'de> PathDeserializer<'server, 'request, 'de> {
    #[inline]
    pub(super) fn new(url_params: &'de [(&'server str, Cow<'request, str>)]) -> Self {
        PathDeserializer { url_params }
    }
}

impl<'server, 'request, 'de> Deserializer<'request> for PathDeserializer<'server, 'request, 'de>
where
    'server: 'request,
{
    type Error = PathDeserializationError;

    unsupported_type!(deserialize_bytes);
    unsupported_type!(deserialize_option);
    unsupported_type!(deserialize_identifier);
    unsupported_type!(deserialize_ignored_any);
    unsupported_type!(deserialize_unit);
    unsupported_type!(deserialize_seq);
    unsupported_type!(deserialize_bool);
    unsupported_type!(deserialize_i8);
    unsupported_type!(deserialize_i16);
    unsupported_type!(deserialize_i32);
    unsupported_type!(deserialize_i64);
    unsupported_type!(deserialize_i128);
    unsupported_type!(deserialize_u8);
    unsupported_type!(deserialize_u16);
    unsupported_type!(deserialize_u32);
    unsupported_type!(deserialize_u64);
    unsupported_type!(deserialize_u128);
    unsupported_type!(deserialize_f32);
    unsupported_type!(deserialize_f64);
    unsupported_type!(deserialize_string);
    unsupported_type!(deserialize_byte_buf);
    unsupported_type!(deserialize_char);
    unsupported_type!(deserialize_str);
    unsupported_type!(deserialize_any);

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        visitor.visit_map(MapDeserializer {
            params: self.url_params,
            value: None,
            key: None,
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }
}

struct MapDeserializer<'de, 'request> {
    params: &'de [(&'request str, Cow<'request, str>)],
    key: Option<Key<'request>>,
    value: Option<&'de Cow<'request, str>>,
}

impl<'de, 'request> MapAccess<'request> for MapDeserializer<'de, 'request> {
    type Error = PathDeserializationError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'request>,
    {
        match self.params.split_first() {
            Some(((key, value), tail)) => {
                self.value = Some(value);
                self.params = tail;
                self.key = Some(Key(key));
                seed.deserialize(KeyDeserializer {
                    key: Cow::Borrowed(key),
                })
                .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'request>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(ValueDeserializer {
                key: self.key.take(),
                value: value.clone(),
            }),
            None => Err(PathDeserializationError::custom("value is missing")),
        }
    }
}

struct KeyDeserializer<'a> {
    key: Cow<'a, str>,
}

macro_rules! parse_key {
    ($trait_fn:ident) => {
        fn $trait_fn<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            visitor.visit_str(&self.key)
        }
    };
}

impl<'de> Deserializer<'de> for KeyDeserializer<'de> {
    type Error = PathDeserializationError;

    parse_key!(deserialize_identifier);
    parse_key!(deserialize_str);
    parse_key!(deserialize_string);

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathDeserializationError::custom("Unexpected key type"))
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes
        byte_buf option unit unit_struct seq tuple
        tuple_struct map newtype_struct struct enum ignored_any
    }
}

macro_rules! parse_value {
    ($trait_fn:ident, $visit_fn:ident, $ty:literal) => {
        fn $trait_fn<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'request>,
        {
            let v = self.value.parse().map_err(|_| {
                if let Some(key) = self.key.take() {
                    let kind = ErrorKind::ParseErrorAtKey {
                        key: key.0.to_string(),
                        value: self.value.to_string(),
                        expected_type: $ty,
                    };
                    PathDeserializationError::new(kind)
                } else {
                    PathDeserializationError::new(ErrorKind::ParseError {
                        value: self.value.to_string(),
                        expected_type: $ty,
                    })
                }
            })?;
            visitor.$visit_fn(v)
        }
    };
}

#[derive(Debug)]
struct ValueDeserializer<'request> {
    key: Option<Key<'request>>,
    value: Cow<'request, str>,
}

impl<'request> Deserializer<'request> for ValueDeserializer<'request> {
    type Error = PathDeserializationError;

    unsupported_type!(deserialize_map);
    unsupported_type!(deserialize_identifier);

    parse_value!(deserialize_bool, visit_bool, "bool");
    parse_value!(deserialize_i8, visit_i8, "i8");
    parse_value!(deserialize_i16, visit_i16, "i16");
    parse_value!(deserialize_i32, visit_i32, "i32");
    parse_value!(deserialize_i64, visit_i64, "i64");
    parse_value!(deserialize_i128, visit_i128, "i128");
    parse_value!(deserialize_u8, visit_u8, "u8");
    parse_value!(deserialize_u16, visit_u16, "u16");
    parse_value!(deserialize_u32, visit_u32, "u32");
    parse_value!(deserialize_u64, visit_u64, "u64");
    parse_value!(deserialize_u128, visit_u128, "u128");
    parse_value!(deserialize_f32, visit_f32, "f32");
    parse_value!(deserialize_f64, visit_f64, "f64");
    parse_value!(deserialize_string, visit_string, "String");
    parse_value!(deserialize_byte_buf, visit_string, "String");
    parse_value!(deserialize_char, visit_char, "char");

    fn deserialize_any<V>(self, v: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        self.deserialize_str(v)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        match self.value {
            Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
            Cow::Owned(s) => visitor.visit_str(&s),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        match self.value {
            Cow::Borrowed(s) => visitor.visit_borrowed_bytes(s.as_bytes()),
            Cow::Owned(s) => visitor.visit_bytes(s.as_bytes()),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        Err(PathDeserializationError::unsupported_type(type_name::<
            V::Value,
        >()))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        visitor.visit_enum(EnumDeserializer {
            value: self.value.clone(),
        })
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'request>,
    {
        visitor.visit_unit()
    }
}

struct EnumDeserializer<'a> {
    value: Cow<'a, str>,
}

impl<'de> EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = PathDeserializationError;
    type Variant = UnitVariant;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        Ok((
            seed.deserialize(KeyDeserializer { key: self.value })?,
            UnitVariant,
        ))
    }
}

struct UnitVariant;

impl<'de> VariantAccess<'de> for UnitVariant {
    type Error = PathDeserializationError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Err(PathDeserializationError::unsupported_type(
            "newtype enum variant",
        ))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathDeserializationError::unsupported_type(
            "tuple enum variant",
        ))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(PathDeserializationError::unsupported_type(
            "struct enum variant",
        ))
    }
}

#[derive(Debug, Clone)]
struct Key<'a>(&'a str);

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    enum MyEnum {
        A,
        B,
        #[serde(rename = "c")]
        C,
    }

    fn create_url_params<'a>(values: &'a [(&'a str, &'a str)]) -> Vec<(&'a str, Cow<'a, str>)> {
        values
            .iter()
            .map(|(k, v)| {
                (
                    k.as_ref(),
                    percent_encoding::percent_decode_str(v)
                        .decode_utf8()
                        .unwrap(),
                )
            })
            .collect()
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct Struct<'a> {
        a: i32,
        c: String,
        b: bool,
        bool_true: bool,
        bool_false: bool,
        n_i8: i8,
        n_i16: i16,
        n_i32: i32,
        n_i64: i64,
        n_i128: i128,
        n_u8: u8,
        n_u16: u16,
        n_u32: u32,
        n_u64: u64,
        n_u128: u128,
        unencoded_string: String,
        unencoded_str: &'a str,
        unencoded_cow: Cow<'a, str>,
        encoded_cow: Cow<'a, str>,
        encoded_string: String,
        char: char,
    }

    #[test]
    fn test_parse_struct() {
        let raw_params = vec![
            ("a", "1"),
            ("b", "true"),
            ("c", "abc"),
            ("bool_true", "true"),
            ("bool_false", "false"),
            ("n_i8", "-123"),
            ("n_i16", "-123"),
            ("n_i32", "-123"),
            ("n_i64", "-123"),
            ("n_i128", "123"),
            ("n_u8", "123"),
            ("n_u16", "123"),
            ("n_u32", "123"),
            ("n_u64", "123"),
            ("n_u128", "123"),
            ("unencoded_string", "abc"),
            ("unencoded_str", "abc"),
            ("unencoded_cow", "abc"),
            ("encoded_cow", "one%20two"),
            ("encoded_string", "one%20two"),
            ("char", "a"),
        ];
        let url_params = create_url_params(&raw_params);
        assert_eq!(
            Struct::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            Struct {
                c: "abc".to_owned(),
                b: true,
                bool_true: true,
                bool_false: false,
                n_i8: -123,
                n_i16: -123,
                n_i32: -123,
                n_i64: -123,
                n_i128: 123,
                n_u8: 123,
                n_u16: 123,
                n_u32: 123,
                n_u64: 123,
                n_u128: 123,
                unencoded_string: "abc".into(),
                unencoded_str: "abc",
                unencoded_cow: "abc".into(),
                encoded_cow: "one two".to_string().into(),
                encoded_string: "one two".to_string(),
                a: 1,
                char: 'a',
            }
        );
    }

    #[test]
    fn test_parse_struct_ignoring_additional_fields() {
        #[derive(Debug, Deserialize, Eq, PartialEq)]
        struct Struct {
            a: i32,
            c: String,
            b: bool,
        }

        let raw_params = vec![("a", "1"), ("b", "true"), ("c", "abc"), ("d", "false")];
        let url_params = create_url_params(&raw_params);
        assert_eq!(
            Struct::deserialize(PathDeserializer::new(&url_params)).unwrap(),
            Struct {
                c: "abc".to_owned(),
                b: true,
                a: 1,
            }
        );
    }

    macro_rules! test_parse_error {
        (
            $params:expr,
            $ty:ty,
            $expected_error_kind:expr $(,)?
        ) => {
            let raw_params = $params;
            let url_params = create_url_params(&raw_params);
            let actual_error_kind = <$ty>::deserialize(PathDeserializer::new(&url_params))
                .unwrap_err()
                .kind;
            assert_eq!(actual_error_kind, $expected_error_kind);
        };
    }

    macro_rules! check_single_value {
        ($ty:ty, $value_str:literal) => {
            #[allow(clippy::bool_assert_comparison)]
            {
                let raw_params = vec![("value", $value_str)];
                let url_params = create_url_params(&raw_params);
                let deserializer = PathDeserializer::new(&url_params);
                assert!(matches!(
                    <$ty>::deserialize(deserializer).unwrap_err().kind,
                    ErrorKind::UnsupportedType { .. }
                ));
            }
        };
    }

    #[test]
    fn test_parse_errors_for_single_values() {
        check_single_value!(bool, "true");
        check_single_value!(bool, "false");
        check_single_value!(i8, "-123");
        check_single_value!(i16, "-123");
        check_single_value!(i32, "-123");
        check_single_value!(i64, "-123");
        check_single_value!(i128, "123");
        check_single_value!(u8, "123");
        check_single_value!(u16, "123");
        check_single_value!(u32, "123");
        check_single_value!(u64, "123");
        check_single_value!(u128, "123");
        check_single_value!(f32, "123");
        check_single_value!(f64, "123");
        check_single_value!(String, "abc");
        check_single_value!(String, "one%20two");
        check_single_value!(&str, "abc");
        check_single_value!(Cow<'_, str>, "abc");
        check_single_value!(Cow<'_, str>, "one%20two");
        check_single_value!(char, "a");
        check_single_value!(MyEnum, "B");
    }

    #[test]
    fn test_parse_error_for_map() {
        test_parse_error!(
            vec![("a", "1"), ("b", "true"), ("c", "abc")],
            HashMap<String, String>,
            ErrorKind::UnsupportedType {
                name: "std::collections::hash::map::HashMap<alloc::string::String, alloc::string::String>"
            }
        );
    }

    #[test]
    fn test_parse_error_for_percent_encoded_str() {
        #[derive(Debug, Deserialize, Eq, PartialEq)]
        struct Struct<'a> {
            key: &'a str,
        }
        // This can't be deserialized as a &str because it contains a percent-encoded space,
        // which requires an allocation when performing the percent-decoding step.
        let err_msg = "invalid type: string \"one two\", expected a borrowed string".to_string();
        test_parse_error!(
            vec![("key", "one%20two")],
            Struct,
            ErrorKind::Message(err_msg)
        );
    }

    #[test]
    fn test_unsupported_seq() {
        test_parse_error!(
            vec![("a", "1"), ("b", "2"), ("c", "3")],
            Vec<i32>,
            ErrorKind::UnsupportedType {
                name: "alloc::vec::Vec<i32>"
            }
        );

        test_parse_error!(
            vec![("a", "c"), ("a", "B")],
            Vec<MyEnum>,
            ErrorKind::UnsupportedType {
                name: "alloc::vec::Vec<pavex::extract::route::deserializer::tests::MyEnum>"
            }
        );

        #[derive(Debug, Deserialize, Eq, PartialEq)]
        struct TupleStruct(i32, bool, String);
        test_parse_error!(
            vec![("a", "1"), ("b", "true"), ("c", "abc")],
            TupleStruct,
            ErrorKind::UnsupportedType {
                name:
                    "pavex::extract::route::deserializer::tests::test_unsupported_seq::TupleStruct"
            }
        );

        test_parse_error!(
            vec![("a", "1"), ("b", "true"), ("c", "abc")],
            (i32, bool, String),
            ErrorKind::UnsupportedType {
                name: "(i32, bool, alloc::string::String)"
            }
        );
    }

    #[test]
    fn test_unsupported_type_error_seq_tuple() {
        test_parse_error!(
            vec![("a", "foo"), ("b", "bar")],
            Vec<(String, String)>,
            ErrorKind::UnsupportedType {
                name: "alloc::vec::Vec<(alloc::string::String, alloc::string::String)>"
            }
        );
    }

    #[test]
    fn test_tuple_with_wrong_number_of_parameters() {
        test_parse_error!(
            vec![("a", "1")],
            (u32, u32),
            ErrorKind::UnsupportedType { name: "(u32, u32)" }
        );
    }

    #[test]
    fn test_parse_error_at_key_error() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Params {
            a: u32,
        }
        test_parse_error!(
            vec![("a", "false")],
            Params,
            ErrorKind::ParseErrorAtKey {
                key: "a".to_owned(),
                value: "false".to_owned(),
                expected_type: "u32",
            }
        );
    }

    #[test]
    fn test_parse_error_at_key_error_multiple() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Params {
            a: u32,
            b: u32,
        }
        test_parse_error!(
            vec![("a", "false")],
            Params,
            ErrorKind::ParseErrorAtKey {
                key: "a".to_owned(),
                value: "false".to_owned(),
                expected_type: "u32",
            }
        );
    }

    #[test]
    fn test_unsupported_type_error_nested_data_structure() {
        test_parse_error!(
            vec![("a", "false")],
            Vec<Vec<u32>>,
            ErrorKind::UnsupportedType {
                name: "alloc::vec::Vec<alloc::vec::Vec<u32>>",
            }
        );
    }

    #[test]
    fn test_parse_seq_seq() {
        test_parse_error!(
            vec![("a", "false")],
            Vec<Vec<String>>,
            ErrorKind::UnsupportedType {
                name: "alloc::vec::Vec<alloc::vec::Vec<alloc::string::String>>",
            }
        );
    }
}
