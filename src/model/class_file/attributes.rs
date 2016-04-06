use super::u1;
use super::u2;
use super::constant_pool_index;
use super::access_flags::inner_class_access_flags;
use super::access_flags::parameter_access_flags;

#[derive(Debug)]
pub struct ExceptionTableEntry {
    /// Indicates the (inclusive) start of the range in the `code` array at
    /// which the exception handler is active. The value of `start_pc` must be a
    /// valid index into the `code` array of the opcode of an instruction. The
    /// exception handler is active in the range `[start_pc, end_pc)`.
    start_pc: u2,
    /// Indicates the (exclusive) end of the range in the `code` array at which
    /// the exception handler is active. The value of `end_pc` must be a valid
    /// index into the `code` array of the opcode of an instruction or must be
    /// equal to the length of the `code` array. The exception handler is active
    /// in the range `[start_pc, end_pc)`.
    end_pc: u2,
    /// The value of the `handler_pc` item indicates the start of the exception
    /// handler. The value of the item must be a valid index into the code array
    /// and must be the index of the opcode of an instruction.
    handler_pc: u2,
    /// If the value of the `catch_type` item is nonzero, it must be a valid
    /// index into the `constant_pool` table. The `constant_pool` entry at that
    /// index must be a `ConstantPoolInfo::Class` structure representing a class
    /// of exceptions that this exception handler is designated to catch. The
    /// exception handler will be called only if the thrown exception is an
    /// instance of the given class or one of its subclasses.
    catch_type: constant_pool_index,
}

#[derive(Debug)]
pub enum VerificationTypeInfo {
    Top,
    Integer,
    Float,
    Long,
    Double,
    Null,
    UninitializedThis,
    Object { class_index: constant_pool_index },
    Uninitialized {
        /// The offset in the `code` array of the `Code` attribute that contains
        /// this `StackMapTable` attribute, of the _new_ instruction that
        /// created the object stored in the location.
        offset: u2,
    },
}

#[derive(Debug)]
pub enum StackMapFrame {
    SameFrame { offset_delta: u1 },
    SameLocals1StackItemFrame { offset_delta: u1, stack_item: VerificationTypeInfo },
    SameLocals1StackItemFrameExtended { offset_delta: u2, stack_item: VerificationTypeInfo },
    ChopFrame { offset_delta: u2 },
    AppendFrame { offset_delta: u2, locals: Vec<VerificationTypeInfo> },
    FullFrame { offset_delta: u2, locals: Vec<VerificationTypeInfo>, stack: Vec<VerificationTypeInfo> },
}

#[derive(Debug)]
pub struct BootstrapMethod {
    /// An index into the `constant_pool` to a `ConstantPoolInfo::MethodHandle` structure.
    bootstrap_method_ref: constant_pool_index,
    /// The indices into the `constant_pool` to `ConstantPoolInfo::String`,
    /// `ConstantPoolInfo::Class`, `ConstantPoolInfo::Integer`,
    /// `ConstantPoolInfo::Long`, `ConstantPoolInfo::Float`,
    /// `ConstantPoolInfo::Double`, `ConstantPoolInfo::MethodHandle`, or
    /// `ConstantPoolInfo::MethodType`.
    bootstrap_arguments: Vec<constant_pool_index>,
}

#[derive(Debug)]
pub struct InnerClass {
    inner_class_info_index: constant_pool_index,
    outer_class_info_index: constant_pool_index,
    inner_name_index: constant_pool_index,
    inner_class_access_flags: inner_class_access_flags::t,
}

#[derive(Debug)]
pub enum ElementValue {
    Byte { const_value_index: constant_pool_index },
    Char { const_value_index: constant_pool_index },
    Double { const_value_index: constant_pool_index },
    Float { const_value_index: constant_pool_index },
    Int { const_value_index: constant_pool_index },
    Long { const_value_index: constant_pool_index },
    Short { const_value_index: constant_pool_index },
    Boolean { const_value_index: constant_pool_index },
    String { const_value_index: constant_pool_index },
    Enum { type_name_index: constant_pool_index, const_name_index: constant_pool_index },
    Class { class_info_index: constant_pool_index },
    Annotation { annotation_value: Annotation },
    Array { values: Vec<ElementValue> },
}

#[derive(Debug)]
pub struct ElementValuePair {
    element_name_index: constant_pool_index,
    element_value: ElementValue,
}

#[derive(Debug)]
pub struct Annotation {
    /// An index into the `constant_pool` table for a `ConstantPoolInfo::Utf8` structure.
    type_index: constant_pool_index,
    element_value_pairs: Vec<ElementValuePair>,
}

#[derive(Debug)]
pub struct Parameter {
    name_index: constant_pool_index,
    access_flags: parameter_access_flags::t,
}

#[derive(Debug)]
pub enum AttributeInfo {
    ConstantValue { constant_value_index: constant_pool_index },
    Code {
        max_stack: u2,
        max_locals: u2,
        code: Vec<u1>,
        exception_table: Vec<ExceptionTableEntry>,
        attributes: Vec<AttributeInfo>,
    },
    StackMapTable {
        entries: Vec<StackMapFrame>,
    },
    Exceptions {
        /// Contains indices into the `constant_pool` table for the class type
        /// that the method is declared to throw.
        exception_index_table: Vec<constant_pool_index>,
    },
    BootstrapMethods {
        bootstrap_methods: Vec<BootstrapMethod>
    },

    InnerClasses {
        classes: Vec<InnerClass>
    },
    EnclosingMethod {
        class_index: constant_pool_index,
        method_index: constant_pool_index,
    },
    Synthetic,
    Signature {
        /// A valid index into the `constant_pool` table for a `ConstantPoolInfo::Utf8` structure.
        signature_index: constant_pool_index,
    },
    RuntimeVisibleAnnotations {
        attribute_name_index: constant_pool_index,
        annotations: Vec<Annotation>,
    },
    RuntimeInvisibleAnnotations {
        attribute_name_index: constant_pool_index,
        annotations: Vec<Annotation>,
    },
    RuntimeVisibleParameterAnnotations {
        attribute_name_index: constant_pool_index,
        parameter_annotations: Vec<Vec<Annotation>>,
    },
    RuntimeInvisibleParameterAnnotations {
        attribute_name_index: constant_pool_index,
        parameter_annotations: Vec<Vec<Annotation>>,
    },
    RuntimeVisibleTypeAnnotations {
        attribute_name_index: constant_pool_index,
        annotations: Vec<Annotation>,
    },
    RuntimeInvisibleTypeAnnotations {
        attribute_name_index: constant_pool_index,
        annotations: Vec<Annotation>,
    },
    AnnotationDefault {
        attribute_name_index: constant_pool_index,
        default_value: ElementValue,
    },
    MethodParameters {
        attribute_name_index: constant_pool_index,
        parameters: Vec<Parameter>,
    },

    /// TODO: debug-related attributes
    Unknown {
        /// A valid index into the `constant_pool` table. The `constant_pool`
        /// entry at that index must be a valid `ConstantPoolInfo::Utf8`
        /// structure representing the name of the attribute.
        attribute_name_index: constant_pool_index,
        /// The data for this attribute.
        info: Vec<u1>,
    },
}
