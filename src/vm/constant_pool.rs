use std::ops::Index;

pub use model::class_file::constant_pool::constant_pool_index;
use model::class_file::constant_pool::{ConstantPool, ConstantPoolInfo};
use vm::{self, symref};
use util::one_indexed_vec::OneIndexedVec;

/// Descriptors for things in the runtime constant pool.
pub mod handle {
    use vm::Value;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum Type {
        Byte,
        Char,
        Double,
        Float,
        Int,
        Long,
        Short,
        Boolean,
        Reference(Class),
    }

    impl Type {
        pub fn new(type_str: &str) -> Self {
            let (ty, rest) = Self::new_partial(type_str).unwrap();
            if rest.len() > 0 {
                panic!("extra content at end of type descriptor")
            } else {
                ty
            }
        }

        pub fn new_multi(multi_type_str: &str) -> (Vec<Self>, &str) {
            let mut types = vec![];
            let mut remainder = multi_type_str;
            {
                let mut result = Ok(&mut types);
                while let Ok(types) = result {
                    result = Self::new_partial(remainder).map(|(ty, new_remainder)| {
                        types.push(ty);
                        remainder = new_remainder;
                        types
                    });
                }
            }
            (types, remainder)
        }

        fn new_partial(type_str: &str) -> Result<(Self, &str), ()> {
            let (specifier, rest) = type_str.split_at(1);
            match specifier {
                "B" => Ok((Type::Byte, rest)),
                "C" => Ok((Type::Char, rest)),
                "D" => Ok((Type::Double, rest)),
                "F" => Ok((Type::Float, rest)),
                "I" => Ok((Type::Int, rest)),
                "J" => Ok((Type::Long, rest)),
                "S" => Ok((Type::Short, rest)),
                "Z" => Ok((Type::Boolean, rest)),
                "L" => {
                    let end_index = rest.find(';').unwrap();
                    let (name_slice, rest) = rest.split_at(end_index);
                    let name = String::from(name_slice);
                    let scalar_type = Type::Reference(Class::Scalar(name));
                    Ok((scalar_type, rest.split_at(1).1))
                },
                "[" => {
                    let (component_type, rest) = try!(Self::new_partial(rest));
                    let array_type = Type::Reference(Class::Array(Box::new(component_type)));
                    Ok((array_type, rest))
                },
                _ => Err(())
            }
        }

        pub fn default_value(&self) -> Value {
            match *self {
                Type::Byte | Type::Char | Type::Int | Type::Short | Type::Boolean => Value::Int(0),
                Type::Double => Value::Double(0.0),
                Type::Float => Value::Float(0.0),
                Type::Long => Value::Long(0),
                Type::Reference(_) => Value::NullReference,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum Class {
        Scalar(String),
        Array(Box<Type>),
    }

    impl Class {
        pub fn new(name: &str) -> Self {
            if name.starts_with('[') {
                let (_, component_type_str) = name.split_at(1);
                let component_type = Type::new(component_type_str);
                Class::Array(Box::new(component_type))
            } else {
                Class::Scalar(String::from(name))
            }
        }

        pub fn get_package(&self) -> Option<String> {
            match *self {
                Class::Scalar(ref name) => {
                    match name.rfind('/') {
                        None => Some(String::from("")),
                        Some(index) => Some(String::from(name.split_at(index).0)),
                    }
                },
                Class::Array(_) => None,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Field {
        pub name: String,
        pub ty: Type,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Method {
        pub name: String,
        pub params: Vec<Type>,
        pub return_ty: Option<Type>,
    }

    impl Method {
        pub fn new(name: &str, descriptor: &str) -> Self {
            if !descriptor.starts_with('(') {
                panic!("invalid method descriptor");
            }
            let params_str = descriptor.split_at(1).1;
            let (params, remainder) = Type::new_multi(params_str);
            let (close_paren, return_ty_str) = remainder.split_at(1);
            if close_paren != ")" {
                panic!("invalid method descriptor");
            }
            let return_ty =
                if return_ty_str == "V" {
                    None
                } else {
                    Some(Type::new(return_ty_str))
                };
            Method {
                name: String::from(name),
                params: params,
                return_ty: return_ty
            }
        }
    }
}

#[derive(Debug)]
pub enum RuntimeConstantPoolEntry {
    ClassRef(symref::Class),
    MethodRef(symref::Method),
    FieldRef(symref::Field),
    PrimitiveLiteral(vm::Value),
    StringLiteralRef(constant_pool_index),
    StringValue(ModifiedUtf8String),
}

#[derive(Debug)]
pub struct RuntimeConstantPool {
    entries: OneIndexedVec<Option<RuntimeConstantPoolEntry>>,
}

impl Index<constant_pool_index> for RuntimeConstantPool {
    type Output = Option<RuntimeConstantPoolEntry>;

    fn index(&self, index: constant_pool_index) -> &Self::Output {
        &self.entries[index as usize]
    }
}

impl RuntimeConstantPool {
    pub fn new(constant_pool: &ConstantPool) -> Self {
        let mut entries = vec![];
        for info in constant_pool {
            let entry = match *info {
                ConstantPoolInfo::Class { .. } => {
                    let class_symref = Self::force_class_ref(&constant_pool, &info);
                    Some(RuntimeConstantPoolEntry::ClassRef(class_symref))
                },

                ConstantPoolInfo::FieldRef { class_index, name_and_type_index } => {
                    let class_symref =
                        Self::force_class_ref(&constant_pool,
                                              &constant_pool[class_index as usize]);
                    let (name, descriptor) =
                        Self::force_name_and_type(&constant_pool,
                                                  &constant_pool[name_and_type_index as usize]);
                    let ty = handle::Type::new(&descriptor);
                    let handle = handle::Field { name: name, ty: ty };
                    let field_symref = symref::Field { class: class_symref, handle: handle };
                    Some(RuntimeConstantPoolEntry::FieldRef(field_symref))
                },

                ConstantPoolInfo::MethodRef { class_index, name_and_type_index } => {
                    let class_symref =
                        Self::force_class_ref(&constant_pool, &constant_pool[class_index as usize]);
                    let (name, descriptor) =
                        Self::force_name_and_type(&constant_pool,
                                                  &constant_pool[name_and_type_index as usize]);
                    let handle = handle::Method::new(&name, &descriptor);
                    let method_symref = symref::Method { class: class_symref, handle: handle };
                    Some(RuntimeConstantPoolEntry::MethodRef(method_symref))
                },

                ConstantPoolInfo::String { string_index } => {
                    Some(RuntimeConstantPoolEntry::StringLiteralRef(string_index))
                },

                ConstantPoolInfo::Integer { bytes } => {
                    let value = vm::Value::Int(bytes as i32);
                    Some(RuntimeConstantPoolEntry::PrimitiveLiteral(value))
                },

                ConstantPoolInfo::Float { bytes } => {
                    let value = vm::Value::Float(bytes as f32);
                    Some(RuntimeConstantPoolEntry::PrimitiveLiteral(value))
                },

                ConstantPoolInfo::Long { high_bytes, low_bytes } => {
                    let bits = ((high_bytes as i64) << 32) & (low_bytes as i64);
                    let value = vm::Value::Long(bits);
                    Some(RuntimeConstantPoolEntry::PrimitiveLiteral(value))
                },

                ConstantPoolInfo::Double { high_bytes, low_bytes } => {
                    let bits = ((high_bytes as u64) << 32) & (low_bytes as u64);
                    let value = vm::Value::Double(bits as f64);
                    Some(RuntimeConstantPoolEntry::PrimitiveLiteral(value))
                },

                ConstantPoolInfo::NameAndType { .. } => None,

                ConstantPoolInfo::Utf8 { ref bytes } => {
                    let modified_utf8 = ModifiedUtf8String::new(bytes.to_vec());
                    Some(RuntimeConstantPoolEntry::StringValue(modified_utf8))
                },

                ConstantPoolInfo::Unusable => None,

                _ => panic!("unsupported ConstantPoolInfo"),
            };
            entries.push(entry);
        }
        RuntimeConstantPool { entries: OneIndexedVec::from(entries) }
    }

    fn force_class_ref(constant_pool: &ConstantPool, info: &ConstantPoolInfo) -> symref::Class {
        match *info {
            ConstantPoolInfo::Class { name_index } => {
                let name = Self::force_string(&constant_pool[name_index as usize]).to_string();
                symref::Class { handle: handle::Class::new(&name) }
            },
            _ => panic!("expected ConstantPoolInfo::Class"),
        }
    }

    fn force_name_and_type(constant_pool: &ConstantPool, info: &ConstantPoolInfo)
            -> (String, String) {
        match *info {
            ConstantPoolInfo::NameAndType { name_index, descriptor_index } => {
                let ref name_info = constant_pool[name_index as usize];
                let ref descriptor_info = constant_pool[descriptor_index as usize];
                let name_string = Self::force_string(name_info).to_string();
                let descriptor_string = Self::force_string(descriptor_info).to_string();
                (name_string, descriptor_string)
            },
            _ => panic!("expected ConstantPoolInfo::NameAndType"),
        }
    }

    fn force_string(info: &ConstantPoolInfo) -> ModifiedUtf8String {
        match *info {
            ConstantPoolInfo::Utf8 { ref bytes } => {
                ModifiedUtf8String::new(bytes.to_vec())
            },
            _ => panic!("attempting to coerce non-UTF8 constant to String"),
        }
    }

    pub fn lookup_raw_string(&self, index: constant_pool_index) -> String {
        match self.entries[index as usize] {
            Some(RuntimeConstantPoolEntry::StringValue(ref modified_utf8)) =>
                modified_utf8.to_string(),
            _ => panic!("expected RuntimeConstantPoolInfo::StringValue"),
        }
    }
}

#[derive(Debug)]
pub struct ModifiedUtf8String {
    bytes: Vec<u8>,
}

impl ModifiedUtf8String {
    fn new(bytes: Vec<u8>) -> Self {
        ModifiedUtf8String { bytes: bytes }
    }

    fn to_string(&self) -> String {
        let mut utf8 = vec![];
        let mut i = 0;
        while i < self.bytes.len() {
            match self.bytes[i] {
                0x01 ... 0x7f => {
                    utf8.push(self.bytes[i]);
                    i += 1;
                },
                0xc0 ... 0xdf => {
                    if self.bytes.len() < i + 2 {
                        panic!("error decoding modified UTF-8: invalid sequence");
                    } else if self.bytes[i] == 0xc0 && self.bytes[i + 1] == 0x80 {
                        // this is the encoding of a null character
                        utf8.push(0x00);
                    } else {
                        utf8.push(self.bytes[i]);
                        utf8.push(self.bytes[i + 1]);
                    }
                    i += 2;
                },
                0xe0 ... 0xef => {
                    if self.bytes.len() < i + 3 {
                        panic!("error decoding modified UTF-8: invalid sequence");
                    } else if self.bytes[i] == 0xed && self.bytes[i + 1] >= 0xa0
                            && self.bytes[i + 1] <= 0xaf {
                        // this sequence encodes a high surrogate
                        // check that the following sequence encodes a low surrogate
                        if self.bytes.len() < i + 6 || self.bytes[i + 3] != 0xed
                                || self.bytes[i + 4] < 0xb0 || self.bytes[i + 4] > 0xbf {
                            panic!("error decoding modified UTF-8: invalid surrogate pair");
                        } else {
                            // decode the surrogate pair into a code point
                            let code_point = (((self.bytes[i + 1] & 0x0f) as u32) << 16)
                                & (((self.bytes[i + 2] & 0x3f) as u32) << 10)
                                & (((self.bytes[i + 4] & 0x0f) as u32) << 6)
                                & ((self.bytes[i + 5] & 0x3f) as u32)
                                + 0x10000;
                            // encode the code point in UTF-8
                            utf8.push(0xf0 & ((code_point & 0x001c0000 >> 18) as u8));
                            utf8.push(0x80 & ((code_point & 0x0003f000 >> 12) as u8));
                            utf8.push(0x80 & ((code_point & 0x00000fc0 >> 6) as u8));
                            utf8.push(0x80 & ((code_point & 0x0000003f) as u8));
                            // skip past the entire surrogate pair
                            i += 6;
                        }
                    } else {
                        utf8.push(self.bytes[i]);
                        utf8.push(self.bytes[i + 1]);
                        utf8.push(self.bytes[i + 2]);
                        i += 3;
                    }
                },
                0x80 ... 0xbf => panic!("error decoding modified UTF-8: invalid continuation byte"),
                _ => panic!("error decoding modified UTF-8: illegal byte"),
            }
        }
        String::from_utf8(utf8).expect("unexpected error decoding modified UTF-8")
    }

    fn to_utf16(&self) -> Vec<u16> {
        let mut utf16 = vec![];
        let mut i = 0;
        while i < self.bytes.len() {
            match self.bytes[i] {
                0x01 ... 0x7f => {
                    utf16.push(self.bytes[i] as u16);
                    i += 1;
                },
                0xc0 ... 0xdf => {
                    if self.bytes.len() < i + 2 {
                        panic!("error decoding modified UTF-8: invalid sequence");
                    } else if self.bytes[i] == 0xc0 && self.bytes[i + 1] == 0x80 {
                        // this is the encoding of a null character
                        utf16.push(0x0000);
                    } else {
                        let code_point =
                            (((self.bytes[i] & 0x1f) as u16) << 6)
                               & ((self.bytes[i + 1] & 0x3f) as u16);
                        utf16.push(code_point);
                    }
                    i += 2;
                },
                0xe0 ... 0xef => {
                    if self.bytes.len() < i + 3 {
                        panic!("error decoding modified UTF-8: invalid sequence");
                    } else {
                        let code_point =
                            (((self.bytes[i] & 0x0f) as u16) << 12)
                                & (((self.bytes[i + 1] & 0x3f) as u16) << 6)
                                & ((self.bytes[i + 2] & 0x3f) as u16);
                        utf16.push(code_point);
                        i += 3;
                    }
                },
                0x80 ... 0xbf => panic!("error decoding modified UTF-8: invalid continuation byte"),
                _ => panic!("error decoding modified UTF-8: illegal byte"),
            }
        }
        utf16
    }
}
