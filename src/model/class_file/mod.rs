pub mod access_flags;
pub mod attributes;

use self::access_flags::class_access_flags;
use self::access_flags::field_access_flags;
use self::access_flags::method_access_flags;

use self::attributes::AttributeInfo;


#[allow(non_camel_case_types)]
pub type u1 = u8;
#[allow(non_camel_case_types)]
pub type u2 = u16;
#[allow(non_camel_case_types)]
pub type u4 = u32;

#[allow(non_camel_case_types)]
pub type constant_pool_index = u2;

#[derive(Debug)]
pub enum ReferenceKind {
    GetField { reference_index: constant_pool_index },
    GetStatic { reference_index: constant_pool_index },
    PutField { reference_index: constant_pool_index },
    PutStatic { reference_index: constant_pool_index },
    InvokeVirtual { reference_index: constant_pool_index },
    InvokeStatic { reference_index: constant_pool_index },
    InvokeSpecial { reference_index: constant_pool_index },
    NewInvokeSpecial { reference_index: constant_pool_index },
    InvokeInterface { reference_index: constant_pool_index },
}

#[derive(Debug)]
pub enum ConstantPoolInfo {
    Class { name_index: constant_pool_index },
    FieldRef { class_index: constant_pool_index, name_and_type_index: constant_pool_index },
    MethodRef { class_index: constant_pool_index, name_and_type_index: constant_pool_index },
    InterfaceMethodRef {
        class_index: constant_pool_index,
        name_and_type_index: constant_pool_index
    },
    String { string_index: u2 },
    Integer { bytes: u4 },
    Float { bytes: u4 },
    Long { high_bytes: u4, low_bytes: u4 },
    Double { high_bytes: u4, low_bytes: u4 },
    NameAndType {
        name_index: constant_pool_index,
        descriptor_index: constant_pool_index,
    },
    Utf8(String),
    MethodHandle { reference_kind: ReferenceKind, reference_index: constant_pool_index },
    MethodType { descriptor_index: constant_pool_index },
    InvokeDynamic {
        /// A valid index into the `bootstrap_methods` array of the bootstrap
        /// method table.
        bootstrap_method_attr_index: constant_pool_index,
        /// A valid index into the `constant_pool` table. The `constant_pool`
        /// entry at that index must be a valid `ConstantPoolInfo::Utf8` structure
        /// representing the name of the attribute.
        name_and_type_index: constant_pool_index,
    },
}

#[derive(Debug)]
pub struct FieldInfo {
    /// Mask of flags used to denote access permissions to and properties of
    /// this field.
    pub access_flags: field_access_flags::t,
    /// A valid index into the `constant_pool` table. The `constant_pool` entry
    /// at that index must be a `ConstantPoolInfo::Utf8` structure representing
    /// a valid unqualified name denoting a field.
    pub name_index: constant_pool_index,
    /// A valid index into the `constant_pool` table. The `constant_pool` entry
    /// at that index must be a `ConstantPoolInfo::Utf8` structure representing
    /// a valid unqualified name denoting a field.
    pub descriptor_index: constant_pool_index,
    /// The attributes associated with this field.
    pub attributes: Vec<AttributeInfo>,
}

#[derive(Debug)]
pub struct MethodInfo {
    /// Mask of flags used to denote access permissions to and properties of
    /// this class or interface. See the documentation for `ClassAccessFlags`
    /// for the interpretation of each flag.
    access_flags: method_access_flags::t,
    /// A valid index into the `constant_pool` table. The `constant_pool` entry
    /// at that index must be a `ConstantPoolInfo::Utf8` structure representing
    /// a valid unqualified name denoting a method.
    name: u2,
    /// A valid index into the `constant_pool` table. The `constant_pool` entry
    /// at that index must be a `ConstantPoolInfo::Utf8` structure representing
    /// a valid method descriptor.
    descriptor_index: u2,
    /// The attributes associated with this method.
    attributes: Vec<AttributeInfo>,
}

#[derive(Debug)]
pub struct ClassFile {
    /// 0xCAFEBABE
    pub magic: u4,
    /// Minor version number
    pub minor_version: u2,
    /// Major version number
    pub major_version: u2,
    /// Number of entries in `constant_pool` plus one.
    pub constant_pool_count: u2,
    /// Table of structures representing various string constants, class and
    /// interface names, field names, and other constants. The `constant_pool`
    /// table is indexed from 1 to `constant_pool_count - 1`.
    pub constant_pool: Vec<ConstantPoolInfo>,
    /// Mask of flags used to denote access permissions to and properties of
    /// this class or interface. See the documentation for `ClassAccessFlags`
    /// for the interpretation of each flag.
    pub access_flags: class_access_flags::t,
    /// A valid index into the `constant_pool` table. The `constant_pool` entry
    /// at that index must be a `ConstantPoolInfo::Class` structure representing
    /// a valid unqualified name denoting a field.
    pub this_class: constant_pool_index,
    /// For a class, must be either zero or a valid index into the
    /// `constant_pool` table. If the value of `super_class` is non-zero, then
    /// the `constant_pool` entry at that index must be a `ConstantPoolInfo::Class`
    /// structure denoting the direct superclass of the class defined by this
    /// class file. Neither the direct superclass nor any of its superclasses
    /// may have the `ACC_FINAL` flag set in the `access_flags` item of its
    /// `ClassFile` structure.
    pub super_class: constant_pool_index,
    /// Each value in `interfaces` mut be a valid index into the `constant_pool`
    /// table. The `constant_pool` entry at each value of `interfaces[i]`, where
    /// `0 ≤ i < interfaces_count`, must be a `ConstantPoolInfo::Class` structure
    /// representing an interface that is a direct superinterface of this class
    /// or interface type, in the left-to-right order given in the source for
    /// the type.
    pub interfaces: Vec<u2>,
    /// Contains only those fields declared by this class or interface. Does not
    /// include items representing fields that are inherited from superclasses
    /// or superinterfaces.
    pub fields: Vec<FieldInfo>,
    /// Contains only those methods declared by this class or interface. Does
    /// not include items representing methods that are inherited from
    /// superclasses or superinterfaces.
    pub methods: Vec<MethodInfo>,
    /// Contains the attributes of this class.
    pub attributes: Vec<AttributeInfo>,
}