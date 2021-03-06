//! A parser for a Java class file.

use nom::{be_u8, be_u16, be_u32, ErrorKind};
use nom;

use model::class_file;
use model::class_file::{AttributeInfo, ClassFile, FieldInfo, MethodInfo};
use model::class_file::attribute;
use model::class_file::constant_pool;
use model::class_file::constant_pool::ConstantPool;
use model::class_file::constant_pool::ConstantPoolInfo;

use util::modified_utf8;

/// The input type to the parser.
pub type Input<'a> = &'a [u8];

/// The return type of a backtracking-controllable parser method.
pub type ParseResult<'a, O> = Result<nom::IResult<Input<'a>, O, Error>, nom::Err<Input<'a>, Error>>;

/// The type of an index into the class file constant pool.
pub type ConstantPoolIndex = class_file::constant_pool_index;

#[derive(Debug)]
pub enum Error {
    ClassFile,
    Magic,
    ConstantPool { constant_pool_count: usize },
    ConstantPoolEntry { index: usize },
    ConstantPoolInfo,
    UnknownConstantPoolTag { tag: u8 },
    ConstantPoolIndexOutOfBounds { index: usize },
    UnexpectedConstantPoolType {
        index: usize,
        expected: constant_pool::Tag,
        actual: constant_pool::Tag
    },

    IllegalModifiedUtf8 { byte: u8 },
    ModifiedUtf8 { length: usize },
    UnknownConstantPoolMethodReferenceTag { tag: u8 },
    Interfaces { interfaces_count: usize },
    Fields { fields_count: usize },
    FieldInfo,
    FieldAttributes { attributes_count: usize},
    Methods { methods_count: usize },
    MethodInfo,
    MethodAttributes { attributes_count: usize },
    ClassAttributes { attributes_count: usize },
    Attribute,
    AttributeInfo { attribute_name: String, attribute_name_index: usize, attribute_length: usize },
    AttributeInfoNameIndexOutOfBounds { attribute_name_index: usize },

    CodeAttributes { attributes_count: usize },
    ExceptionTableEntry,
    StackMapTable { number_of_entries: usize },
    StackMapFrame,
    UnknownStackMapFrameTag { tag: u8 },
    ReservedStackMapFrameTag { tag: u8 },
    VerificationTypeInfo,
    UnknownVerificationTypeInfoTag { tag: u8 },

    InnerClasses { number_of_classes: usize },
    InnerClass,
    Signature,
    MethodParameters { parameters_count: usize },
    MethodParameter,
    ElementValuePair,
    ElementValuePairs { num_element_value_pairs: usize },
    ElementValue,
    UnknownElementValueTag { tag: u8 },
    ElementValueArray { num_values: usize },
    Annotations { num_annotations: usize },
    ParameterAnnotations { num_parameters: usize },
    TypeAnnotations { num_annotations: usize },
    UnknownTargetTypeTag { tag: u8 },
    LocalVariableTarget { table_length: usize },
    TypePath { path_length: usize},

    SourceFile,
    SourceDebugExtension,
    LineNumberTable { table_length: usize },
    LineNumberInfo,
    LocalVariableTable { table_length: usize },
    LocalVariableInfo,
    LocalVariableTypeTable { table_length: usize },
    LocalVariableTypeInfo,
}

macro_rules! p {
    ($i: expr, $($args: tt)*) => (fix_error!($i, Error, $($args)*));
}

n!(magic<Input, &[u8], Error>, p_cut!(Error::Magic, p!(tag!(&[0xCA, 0xFE, 0xBA, 0xBE]))));

n!(cp_info_tag<Input, constant_pool::Tag, Error>, map!(
    p!(be_u8), constant_pool::Tag::from));

n!(cp_index<Input, ConstantPoolIndex, Error>, p!(be_u16));

macro_rules! check_cp_index_tag {
    ($constant_pool: expr, $i: expr, $tag: expr) => ({
        match $constant_pool.get($i) {
            None => p_fail!(Error::ConstantPoolIndexOutOfBounds { index: $i }),
            Some(r) if r.tag() == $tag => Ok(()),
            Some(r) => p_fail!(Error::UnexpectedConstantPoolType {
                index: $i,
                expected: $tag,
                actual: r.tag(),
            }),
        }
    });
}

/// Parses for a constant pool index and verifies that its the entry in the
/// constant pool matches the specified tag.
fn cp_index_tag<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool, tag: constant_pool::Tag)
                        -> ParseResult<'a, ConstantPoolIndex> {
    let (input, i) = p_try!(input, p_wrap_nom!(p!(be_u16)));
    try!(check_cp_index_tag!(constant_pool, i as usize, tag));
    Ok(done!(input, i))
}

/// Parses for a constant pool index that might be zero and verifies that its
/// the entry in the constant pool matches the specified tag.
fn maybe_cp_index_tag<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool, tag: constant_pool::Tag)
                              -> ParseResult<'a, ConstantPoolIndex> {
    let (input, i) = p_try!(input, p_wrap_nom!(p!(be_u16)));
    if i != 0 {
        try!(check_cp_index_tag!(constant_pool, i as usize, tag));
    }
    Ok(done!(input, i))
}

macro_rules! satisfy {
    ($i: expr, $f: expr, $e: expr) => ({
      let res: $crate::nom::IResult<_, _, _> = if $i.len() == 0 {
          $crate::nom::IResult::Incomplete($crate::nom::Needed::Size(1))
      } else {
          let b = $i[0];
          if $f(b) {
              $crate::nom::IResult::Done(&$i[1..], b)
          } else {
              p_fail!($e(b))
          }
      };
      res
    }
  );
}

n!(modified_utf8<Input, u8, Error>, satisfy!(
    |b| ![0x00, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd,
          0xfe, 0xff].contains(&b),
    |b| Error::IllegalModifiedUtf8 { byte: b }));

macro_rules! take_modified_utf8 {
    ($i: expr, $n: expr) => (p_cut!($i, Error::ModifiedUtf8 { length: $n }, count!(c!(modified_utf8), $n)))
}

n!(reference_kind<Input, constant_pool::reference_kind::Tag, Error>, map!(
    p!(be_u8),
    constant_pool::reference_kind::Tag::from));

fn reference(input: Input, tag: constant_pool::reference_kind::Tag)
             -> ParseResult<constant_pool::MethodReference> {
    let r = match tag {
        constant_pool::reference_kind::Tag::GetField => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::GetField {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::GetStatic => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::GetStatic {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::PutField => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::PutField {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::PutStatic => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::PutStatic {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::InvokeVirtual => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::InvokeVirtual {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::InvokeStatic => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::InvokeStatic {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::InvokeSpecial => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::InvokeSpecial {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::NewInvokeSpecial => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::NewInvokeSpecial {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::InvokeInterface => map!(
            input, c!(cp_index), |ri| constant_pool::MethodReference::InvokeInterface {
                reference_index: ri
            }),
        constant_pool::reference_kind::Tag::Unknown(t) =>
            p_fail!(Error::UnknownConstantPoolMethodReferenceTag { tag: t }),
    };
    wrap_nom!(r)
}

fn cp_info_info(input: Input, tag: constant_pool::Tag) -> ParseResult<ConstantPoolInfo> {
    let r = match tag {
        constant_pool::Tag::Class => map!(input, c!(cp_index),
                                          |ci| ConstantPoolInfo::Class { name_index: ci }),

        constant_pool::Tag::FieldRef => chain!(input,
                                               ci: c!(cp_index) ~
                                               nti: c!(cp_index),
                                               || ConstantPoolInfo::FieldRef {
                                                   class_index: ci,
                                                   name_and_type_index: nti,
                                               }),

        constant_pool::Tag::MethodRef => chain!(input,
                                               ci: c!(cp_index) ~
                                               nti: c!(cp_index),
                                               || ConstantPoolInfo::MethodRef {
                                                   class_index: ci,
                                                   name_and_type_index: nti,
                                               }),

        constant_pool::Tag::InterfaceMethodRef => chain!(input,
                                                         ci: c!(cp_index) ~
                                                         nti: c!(cp_index),
                                                         || ConstantPoolInfo::InterfaceMethodRef {
                                                             class_index: ci,
                                                             name_and_type_index: nti,
                                                         }),

        constant_pool::Tag::String => map!(input, c!(cp_index),
                                           |si| ConstantPoolInfo::String { string_index: si }),

        constant_pool::Tag::Integer => map!(input, p!(be_u32),
                                            |bs| ConstantPoolInfo::Integer { bytes: bs }),

        constant_pool::Tag::Float => map!(input, p!(be_u32),
                                          |bs| ConstantPoolInfo::Float { bytes: bs }),

        constant_pool::Tag::Long => chain!(input,
                                           hi: p!(be_u32) ~
                                           lo: p!(be_u32),
                                           || ConstantPoolInfo::Long {
                                               high_bytes: hi,
                                               low_bytes: lo,
                                           }),

        constant_pool::Tag::Double => chain!(input,
                                             hi: p!(be_u32) ~
                                             lo: p!(be_u32),
                                             || ConstantPoolInfo::Double {
                                                 high_bytes: hi,
                                                 low_bytes: lo,
                                             }),

        constant_pool::Tag::NameAndType => chain!(input,
                                                  ni: c!(cp_index) ~
                                                  di: c!(cp_index),
                                                  || ConstantPoolInfo::NameAndType {
                                                      name_index: ni,
                                                      descriptor_index: di,
                                                  }),

        constant_pool::Tag::Utf8 => chain!(input,
                                           len: p!(be_u16) ~
                                           bs: take_modified_utf8!(len as usize),
                                           || ConstantPoolInfo::Utf8 { bytes: bs }),

        constant_pool::Tag::MethodHandle => chain!(input,
                                                   rk: c!(reference_kind) ~
                                                   r: c!(reference, rk),
                                                   || ConstantPoolInfo::MethodHandle {
                                                       reference: r
                                                   }),

        constant_pool::Tag::MethodType => map!(input, c!(cp_index),
                                               |di| ConstantPoolInfo::MethodType {
                                                   descriptor_index: di
                                               }),

        constant_pool::Tag::InvokeDynamic => chain!(input,
                                                    bmai: c!(cp_index) ~
                                                    nti: c!(cp_index),
                                                    || ConstantPoolInfo::InvokeDynamic {
                                                        bootstrap_method_attr_index: bmai,
                                                        name_and_type_index: nti,
                                                    }),

        constant_pool::Tag::Unknown(t) => p_nom_error!(Error::UnknownConstantPoolTag { tag: t }),
    };
    wrap_nom!(r)
}

n!(cp_info<Input, ConstantPoolInfo, Error>, p_cut!(
    Error::ConstantPoolInfo,
    chain!(tag: c!(cp_info_tag) ~
           cp_info: c!(cp_info_info, tag),
           || cp_info)));

fn exception_table<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                           -> ParseResult<'a, attribute::ExceptionTableEntry> {
    let r = p_cut!(
        input,
        Error::ExceptionTableEntry,
        chain!(start_pc: p!(be_u16) ~
               end_pc: p!(be_u16) ~
               handler_pc: p!(be_u16) ~
               catch_type: c!(maybe_cp_index_tag, constant_pool, constant_pool::Tag::Class),
               || attribute::ExceptionTableEntry {
                   start_pc: start_pc,
                   end_pc: end_pc,
                   handler_pc: handler_pc,
                   catch_type: catch_type,
               }));
    Ok(r)
    }

n!(verification_type_info_tag<Input, attribute::stack_map_frame::verification_type_info::Tag, Error>,
   map!(p!(be_u8), attribute::stack_map_frame::verification_type_info::Tag::from));

fn verification_type_info<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                                  -> ParseResult<'a, attribute::stack_map_frame::VerificationTypeInfo> {
    use model::class_file::attribute::stack_map_frame::verification_type_info::Tag;
    use model::class_file::attribute::stack_map_frame::VerificationTypeInfo;
    let action = |input| {
        let (input, tag) = p_try!(input, verification_type_info_tag);
        let r = match tag {
            Tag::Top => done!(input, VerificationTypeInfo::Top),
            Tag::Integer => done!(input, VerificationTypeInfo::Integer),
            Tag::Float => done!(input, VerificationTypeInfo::Float),
            Tag::Long => done!(input, VerificationTypeInfo::Long),
            Tag::Double => done!(input, VerificationTypeInfo::Double),
            Tag::Null => done!(input, VerificationTypeInfo::Null),
            Tag::UninitializedThis => done!(input, VerificationTypeInfo::UninitializedThis),
            Tag::Object => {
                let (input, i) = p_try!(input, cp_index);
                try!(check_cp_index_tag!(constant_pool, i as usize, constant_pool::Tag::Class));
                done!(input, VerificationTypeInfo::Object { class_index: i })
            },
            Tag::Uninitialized => map!(input, p!(be_u16),
                                       |offset| VerificationTypeInfo::Uninitialized {
                                                   offset: offset,
                                       }),
            Tag::Unknown(t) => p_fail!(Error::UnknownVerificationTypeInfoTag { tag: t }),
        };
        Ok(r)
    };
    p_wrap_nom!(input, p_cut!(Error::VerificationTypeInfo, c!(action)))
}

fn stack_map_frame_info<'a, 'b>(input: Input<'a>, tag: attribute::stack_map_frame::Tag,
                                constant_pool: &'b ConstantPool)
                                -> ParseResult<'a, attribute::StackMapFrame> {
    use model::class_file::attribute::stack_map_frame::Tag;
    use model::class_file::attribute::StackMapFrame;
    let r = match tag {
        Tag::SameFrame(t) => done!(input, StackMapFrame::SameFrame { offset_delta: t }),
        Tag::SameLocals1StackItemFrame(t) =>
            chain!(input,
                   stack_item: c!(verification_type_info, constant_pool),
                   || StackMapFrame::SameLocals1StackItemFrame {
                       offset_delta: t - 64,
                       stack_item: stack_item,
                   }),

        Tag::SameLocals1StackItemFrameExtended(_) =>
            chain!(input,
                   offset_delta: p!(be_u16) ~
                   stack_item: c!(verification_type_info, constant_pool),
                   || StackMapFrame::SameLocals1StackItemFrameExtended {
                       offset_delta: offset_delta,
                       stack_item: stack_item,
                   }),

        Tag::ChopFrame(t) => map!(input,
                                  p!(be_u16),
                                  |offset_delta| StackMapFrame::ChopFrame {
                                        offset_delta: offset_delta,
                                        num_chopped: 251 - t
                                  }),

        Tag::SameFrameExtended(_) => map!(input,
                                          p!(be_u16),
                                          |offset_delta| StackMapFrame::SameFrameExtended {
                                              offset_delta: offset_delta
                                          }),

        Tag::AppendFrame(t) =>
            chain!(input,
                   offset_delta: p!(be_u16) ~
                   locals: count!(c!(verification_type_info, constant_pool), t as usize - 251),
                   || StackMapFrame::AppendFrame {
                       offset_delta: offset_delta,
                       locals: locals,
                   }),

        Tag::FullFrame(_) =>
            chain!(input,
                   offset_delta: p!(be_u16) ~
                   num_locals: p!(be_u16) ~
                   locals: count!(c!(verification_type_info, constant_pool), num_locals as usize) ~
                   num_stack: p!(be_u16) ~
                   stack: count!(c!(verification_type_info, constant_pool), num_stack as usize),
                   || StackMapFrame::FullFrame {
                       offset_delta: offset_delta,
                       locals: locals,
                       stack: stack,
                   }),

        Tag::Reserved(t) => p_fail!(Error::ReservedStackMapFrameTag { tag: t }),
        Tag::Unknown(t) => p_fail!(Error::UnknownStackMapFrameTag { tag: t }),
    };
    Ok(r)
}

fn stack_map_frame<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                           -> ParseResult<'a, attribute::StackMapFrame> {
    p_wrap_nom!(input, p_cut!(
        Error::StackMapFrame,
        chain!(tag: map!(p!(be_u8), attribute::stack_map_frame::Tag::from) ~
               frame: c!(stack_map_frame_info, tag, constant_pool),
               || frame)))
}

n!(line_number_info<Input, attribute::LineNumberInfo, Error>, p_cut!(
    Error::LineNumberInfo,
    chain!(start_pc: p!(be_u16) ~
           line_number: p!(be_u16),
           || attribute::LineNumberInfo {
               start_pc: start_pc,
               line_number: line_number,
           })));

n!(local_variable_info<Input, attribute::LocalVariableInfo, Error>, p_cut!(
    Error::LocalVariableInfo,
    chain!(start_pc: p!(be_u16) ~
           length: p!(be_u16) ~
           name_index: p!(be_u16) ~
           descriptor_index: p!(be_u16) ~
           index: p!(be_u16),
           || attribute::LocalVariableInfo {
               start_pc: start_pc,
               length: length,
               name_index: name_index,
               descriptor_index: descriptor_index,
               index: index,
           })));

n!(local_variable_type_info<Input, attribute::LocalVariableTypeInfo, Error>, p_cut!(
    Error::LocalVariableTypeInfo,
    chain!(start_pc: p!(be_u16) ~
           length: p!(be_u16) ~
           name_index: p!(be_u16) ~
           signature_index: p!(be_u16) ~
           index: p!(be_u16),
           || attribute::LocalVariableTypeInfo {
               start_pc: start_pc,
               length: length,
               name_index: name_index,
               signature_index: signature_index,
               index: index,
           })));

fn attribute_info<'a, 'b>(input: Input<'a>, attribute_name_index: ConstantPoolIndex,
                           attribute_length: u32, constant_pool: &'b ConstantPool)
                          -> ParseResult<'a, AttributeInfo> {
    let r = match constant_pool.get(attribute_name_index as usize) {
        None => p_fail!(Error::AttributeInfoNameIndexOutOfBounds {
            attribute_name_index: attribute_name_index as usize
        }),

        Some(cp_entry) => match *cp_entry {
            ConstantPoolInfo::Utf8 { bytes: ref bs } => {
                let name = bs.as_slice();
                p_cut!(
                    input,
                    Error::AttributeInfo {
                        attribute_name: match modified_utf8::from_modified_utf8(name) {
                            Ok(s) => s,
                            Err(_) => String::from_utf8_lossy(name).into_owned(),
                        },
                        attribute_name_index: attribute_name_index as usize,
                        attribute_length: attribute_length as usize,
                    },
                    c!(attribute_info_switch, bs.as_slice(), attribute_name_index, attribute_length,
                       constant_pool))
            },

            ref cp_entry => p_fail!(Error::UnexpectedConstantPoolType {
                index: attribute_name_index as usize,
                expected: constant_pool::Tag::Utf8,
                actual: cp_entry.tag()
            }),
        }
    };
    Ok(r)
}

fn inner_class<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                       -> ParseResult<'a, attribute::InnerClass> {
    wrap_nom!(p_cut!(
        input,
        Error::InnerClass,
        chain!(inner_class_info_index: c!(cp_index_tag, constant_pool, constant_pool::Tag::Class) ~
               outer_class_info_index: c!(maybe_cp_index_tag, constant_pool, constant_pool::Tag::Class) ~
               inner_name_index: c!(maybe_cp_index_tag, constant_pool, constant_pool::Tag::Utf8) ~
               inner_class_access_flags: p!(be_u16),
               || attribute::InnerClass {
                   inner_class_info_index: inner_class_info_index,
                   outer_class_info_index: outer_class_info_index,
                   inner_name_index: inner_name_index,
                   inner_class_access_flags: inner_class_access_flags,
               })))
}

fn method_parameter<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                            -> ParseResult<'a, attribute::MethodParameter> {
    wrap_nom!(p_cut!(
        input,
        Error::MethodParameter,
        chain!(name_index: c!(maybe_cp_index_tag, constant_pool, constant_pool::Tag::Utf8) ~
               access_flags: p!(be_u16),
               || attribute::MethodParameter {
                   name_index: name_index,
                   access_flags: access_flags,
               })))
}

fn element_value_pair<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                              -> ParseResult<'a, attribute::annotation::ElementValuePair> {
    wrap_nom!(p_cut!(input,
                     Error::ElementValuePair,
                     chain!(eni: c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8) ~
                            value: c!(element_value, constant_pool),
                            || attribute::annotation::ElementValuePair {
                                element_name_index: eni,
                                value: value,
                            })))
}

fn element_value_pairs<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                               -> ParseResult<'a, Vec<attribute::annotation::ElementValuePair>> {
    wrap_nom!(
        chain!(input,
               num_pairs: p!(be_u16) ~
               element_value_pairs: p_cut!(
                   Error::ElementValuePairs { num_element_value_pairs: num_pairs as usize },
                   count!(c!(element_value_pair, constant_pool), num_pairs as usize)),
               || element_value_pairs))
}

fn element_value<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                         -> ParseResult<'a, attribute::annotation::ElementValue> {
    use model::class_file::attribute::annotation::element_value::Tag;
    use model::class_file::attribute::annotation::ElementValue;
    let (input, tag) = p_unwrap!(wrap_nom!(p_cut!(
        input,
        Error::ElementValue,
        map!(p!(be_u8), Tag::from))));
    let r = match tag {
        Tag::Byte => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Integer),
                          |i| ElementValue::Byte { const_value_index: i }),

        Tag::Char => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Integer),
                          |i| ElementValue::Char { const_value_index: i }),

        Tag::Double => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Double),
                            |i| ElementValue::Double { const_value_index: i }),

        Tag::Float => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Float),
                           |i| ElementValue::Float { const_value_index: i }),

        Tag::Int => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Integer),
                         |i| ElementValue::Int { const_value_index: i }),

        Tag::Long => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Long),
                          |i| ElementValue::Long { const_value_index: i }),

        Tag::Short => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Integer),
                           |i| ElementValue::Short { const_value_index: i }),

        Tag::Boolean => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Integer),
                             |i| ElementValue::Boolean { const_value_index: i }),

        Tag::String => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8),
                            |i| ElementValue::String { const_value_index: i }),

        Tag::Enum => chain!(input,
                            tni: c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8) ~
                            cni: c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8),
                            || ElementValue::Enum {
                                type_name_index: tni,
                                const_name_index: cni,
                            }),

        Tag::Class => map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8),
                           |i| ElementValue::Class { class_info_index: i }),

        Tag::Annotation => map!(input, c!(annotation, constant_pool),
                                |a| ElementValue::Annotation { annotation_value: a }),

        Tag::Array => chain!(input,
                             num_values: p!(be_u16) ~
                             values: p_cut!(
                                 Error::ElementValueArray { num_values: num_values as usize },
                                 count!(c!(element_value, constant_pool), num_values as usize)),
                             || ElementValue::Array { values: values }),

        Tag::Unknown(t) => p_fail!(Error::UnknownElementValueTag { tag: t }),
    };
    wrap_nom!(r)
}

fn annotation<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                      -> ParseResult<'a, attribute::annotation::Annotation> {
    wrap_nom!(
        chain!(input,
               type_index: c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8) ~
               element_value_pairs: c!(element_value_pairs, constant_pool),
               || attribute::annotation::Annotation {
                   type_index: type_index,
                   element_value_pairs: element_value_pairs,
               }))
}

fn annotations<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                       -> ParseResult<'a, Vec<attribute::annotation::Annotation>> {
    wrap_nom!(
        chain!(input,
               num_annots: p!(be_u16) ~
               annotations: p_cut!(Error::Annotations { num_annotations: num_annots as usize },
                                   count!(c!(annotation, constant_pool), num_annots as usize)),
               || annotations))
}

fn parameter_annotations<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                                 -> ParseResult<'a, Vec<Vec<attribute::annotation::Annotation>>> {
    p_wrap_nom!(
        input,
        chain!(num_parameters: p!(be_u16) ~
               parameter_annotations: p_cut!(
                   Error::ParameterAnnotations { num_parameters: num_parameters as usize },
                   count!(c!(annotations, constant_pool), num_parameters as usize)),
               || parameter_annotations))
}

fn target_info(input: Input) -> ParseResult<attribute::annotation::TargetInfo> {
    use model::class_file::attribute::annotation::target_type::Tag;
    use model::class_file::attribute::annotation::TargetInfo;
    use model::class_file::attribute::annotation::LocalVariableTargetInfo;
    let (input, tag) = p_try!(input, p_wrap_nom!(map!(p!(be_u8), Tag::from)));
    let r = match tag {
        Tag::TypeParameter =>
            map!(input, p!(be_u8), |i| TargetInfo::TypeParameter { type_parameter_index: i }),

        Tag::Supertype =>
            map!(input, p!(be_u16), |i| TargetInfo::Supertype { supertype_index: i }),

        Tag::TypeParameterBound =>
            chain!(input,
                   type_parameter_index: p!(be_u8) ~
                   bound_index: p!(be_u8),
                   || TargetInfo::TypeParameterBound {
                       type_parameter_index: type_parameter_index,
                       bound_index: bound_index,
                   }),

        Tag::Empty => done!(input, TargetInfo::Empty),

        Tag::FormalParameter =>
            map!(input, p!(be_u8), |i| TargetInfo::FormalParameter { formal_parameter_index: i }),

        Tag::Throws => map!(input, p!(be_u16), |i| TargetInfo::Throws { throws_type_index: i }),

        Tag::LocalVariable =>
            chain!(input,
                   table_length: p!(be_u16) ~
                   table: p_cut!(
                       Error::LocalVariableTarget { table_length: table_length as usize },
                       count!(chain!(start_pc: p!(be_u16) ~
                                     length: p!(be_u16) ~
                                     index: p!(be_u16),
                                     || LocalVariableTargetInfo {
                                         start_pc: start_pc,
                                         length: length,
                                         index: index,
                                     }),
                              table_length as usize)),
                   || TargetInfo::LocalVariable { table: table }),

        Tag::Catch => map!(input, p!(be_u16), |i| TargetInfo::Catch { exception_table_index: i }),

        Tag::Offset => map!(input, p!(be_u16), |i| TargetInfo::Offset { offset: i }),

        Tag::TypeArgument => chain!(input,
                                    offset: p!(be_u16) ~
                                    type_argument_index: p!(be_u8),
                                    || TargetInfo::TypeArgument{
                                        offset: offset,
                                        type_argument_index: type_argument_index,
                                    }),
        Tag::Unknown(t) => p_fail!(Error::UnknownTargetTypeTag { tag: t }),
    };
    wrap_nom!(r)
}

n!(type_path<Input, attribute::annotation::TypePath, Error>, chain!(
    path_length: p!(be_u8) ~
    path: p_cut!(
        Error::TypePath { path_length: path_length as usize },
        count!(chain!(type_path_kind: p!(be_u8) ~
                      type_argument_index: p!(be_u8),
                      || attribute::annotation::TypePathPart {
                          type_path_kind: type_path_kind,
                          type_argument_index: type_argument_index,
                      }),
               path_length as usize)),
    || attribute::annotation::TypePath { path: path }));

fn type_annotation<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                           -> ParseResult<'a, attribute::annotation::TypeAnnotation> {
    p_wrap_nom!(
        input,
        chain!(target_info: c!(target_info) ~
               target_path: c!(type_path) ~
               type_index: c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8) ~
               element_value_pairs: c!(element_value_pairs, constant_pool),
               || attribute::annotation::TypeAnnotation {
                   target_info: target_info,
                   target_path: target_path,
                   type_index: type_index,
                   element_value_pairs: element_value_pairs,
               }))
}

fn type_annotations<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                            -> ParseResult<'a, Vec<attribute::annotation::TypeAnnotation>> {
    p_wrap_nom!(
        input,
        chain!(num_annotations: p!(be_u16) ~
               type_annotations: p_cut!(
                   Error::TypeAnnotations { num_annotations: num_annotations as usize },
                   count!(c!(type_annotation, constant_pool), num_annotations as usize)),
               || type_annotations))
}

fn attribute_info_switch<'a, 'b>(input: Input<'a>, attribute_name: &[u8],
                                 attribute_name_index: ConstantPoolIndex, attribute_length: u32,
                                 constant_pool: &'b ConstantPool)
                                 -> ParseResult<'a, AttributeInfo> {
    let r = match attribute_name {
        b"ConstantValue" => chain!(input,
                                   ci: c!(cp_index),
                                   || AttributeInfo::ConstantValue {
                                       constant_value_index: ci
                                   }),

        b"Code" =>
            chain!(input,
                   max_stack: p!(be_u16) ~
                   max_locals: p!(be_u16) ~
                   code_length: p!(be_u32) ~
                   code: p!(map!(take!(code_length as usize), |bs: Input| bs.to_vec())) ~
                   exception_table_length: p!(be_u16) ~
                   exception_table: count!(c!(exception_table, constant_pool),
                                           exception_table_length as usize) ~
                   attributes_count: p!(be_u16) ~
                   attributes: p_cut!(
                       Error::CodeAttributes {
                           attributes_count: attributes_count as usize
                       },
                       count!(c!(attribute, constant_pool),
                              attributes_count as usize)),
                   ||
                   AttributeInfo::Code {
                       max_stack: max_stack,
                       max_locals: max_locals,
                       code: code,
                       exception_table: exception_table,
                       attributes: attributes,
                   }),

        b"StackMapTable" =>
            chain!(input,
                   count: p!(be_u16) ~
                   entries: p_cut!(
                       Error::StackMapTable { number_of_entries: count as usize },
                       count!(c!(stack_map_frame, constant_pool), count as usize)),
                   || AttributeInfo::StackMapTable { entries: entries }),

        b"Exceptions" =>
            chain!(input,
                   exceptions_count: p!(be_u16) ~
                   exception_index_table: count!(
                       c!(cp_index_tag, constant_pool, constant_pool::Tag::Class),
                       exceptions_count as usize),
                   || AttributeInfo::Exceptions {
                       exception_index_table: exception_index_table,
                   }),

        b"InnerClasses" =>
            chain!(input,
                   number_of_classes: p!(be_u16) ~
                   classes: p_cut!(
                       Error::InnerClasses { number_of_classes: number_of_classes as usize },
                       count!(c!(inner_class, constant_pool), number_of_classes as usize)),
                   || AttributeInfo::InnerClasses { classes: classes }),

        b"EnclosingMethod" =>
            chain!(input,
                   class_index: c!(cp_index_tag, constant_pool, constant_pool::Tag::Class) ~
                   method_index: c!(cp_index_tag, constant_pool, constant_pool::Tag::NameAndType),
                   || AttributeInfo::EnclosingMethod {
                       class_index: class_index,
                       method_index: method_index,
                   }),

        b"Synthetic" => done!(input, AttributeInfo::Synthetic),

        b"Signature" => p_cut!(
            input, Error::Signature,
            map!(c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8),
                 |si| AttributeInfo::Signature {
                     signature_index: si
                 })),

        b"RuntimeVisibleAnnotations" =>
            map!(input, c!(annotations, constant_pool),
                 |annots| AttributeInfo::RuntimeVisibleAnnotations { annotations: annots }),

        b"RuntimeInvisibleAnnotations" =>
            map!(input, c!(annotations, constant_pool),
                 |annots| AttributeInfo::RuntimeInvisibleAnnotations { annotations: annots }),

        b"RuntimeVisibleParameterAnnotations" =>
            map!(input, c!(parameter_annotations, constant_pool),
                 |param_annots| AttributeInfo::RuntimeVisibleParameterAnnotations {
                     parameter_annotations: param_annots,
                 }),

        b"RuntimeInvisibleParameterAnnotations" =>
            map!(input, c!(parameter_annotations, constant_pool),
                 |param_annots| AttributeInfo::RuntimeInvisibleParameterAnnotations {
                     parameter_annotations: param_annots,
                 }),

        b"RuntimeVisibleTypeAnnotations" =>
            map!(input, c!(type_annotations, constant_pool),
                 |type_annots| AttributeInfo::RuntimeVisibleTypeAnnotations {
                     annotations: type_annots,
                 }),

        b"RuntimeInvisibleTypeAnnotations" =>
            map!(input, c!(type_annotations, constant_pool),
                 |type_annots| AttributeInfo::RuntimeInvisibleTypeAnnotations {
                     annotations: type_annots,
                 }),

        b"AnnotationDefault" => map!(input, c!(element_value, constant_pool),
                                     |ev| AttributeInfo::AnnotationDefault { default_value: ev } ),

        b"MethodParameters" =>
            chain!(input,
                   parameters_count: p!(be_u16) ~
                   parameters: p_cut!(
                       Error::MethodParameters { parameters_count: parameters_count as usize },
                       count!(c!(method_parameter, constant_pool), parameters_count as usize)),
                   || AttributeInfo::MethodParameters { parameters: parameters }),

        b"SourceFile" =>
            map!(input, c!(cp_index_tag, constant_pool, constant_pool::Tag::Utf8),
                 |si| AttributeInfo::SourceFile {
                     sourcefile_index: si
                 }),

        b"SourceDebugExtension" => p_cut!(
            input,
            Error::SourceDebugExtension,
            map!(p!(take!(attribute_length)),
                 |bs: Input| AttributeInfo::SourceDebugExtension {
                     debug_extension: bs.to_vec(),
                 })),

        b"LineNumberTable" =>
            chain!(input,
                   table_length: p!(be_u16) ~
                   table: p_cut!(
                       Error::LineNumberTable { table_length: table_length as usize },
                       count!(c!(line_number_info), table_length as usize)),
                   || AttributeInfo::LineNumberTable {
                       line_number_table: table,
                   }),

        b"LocalVariableTable" =>
            chain!(input,
                   table_length: p!(be_u16) ~
                   table: p_cut!(
                       Error::LocalVariableTable { table_length: table_length as usize },
                       count!(c!(local_variable_info), table_length as usize)),
                   || AttributeInfo::LocalVariableTable {
                       local_variable_table: table,
                   }),

        b"LocalVariableTypeTable" =>
            chain!(input,
                   table_length: p!(be_u16) ~
                   table: p_cut!(
                       Error::LocalVariableTypeTable { table_length: table_length as usize },
                       count!(c!(local_variable_type_info), table_length as usize)),
                   || AttributeInfo::LocalVariableTypeTable {
                       local_variable_type_table: table,
                   }),

        b"Deprecated" => done!(input, AttributeInfo::Deprecated),

        _ => map!(input, p!(take!(attribute_length)), |bs: Input| AttributeInfo::Unknown {
            attribute_name_index: attribute_name_index,
            info: bs.to_vec()
        }),
    };
    Ok(r)
}

fn attribute<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                     -> ParseResult<'a, AttributeInfo> {
    p_wrap_nom!(input, p_cut!(
        Error::Attribute,
        chain!(attribute_name_index: c!(cp_index) ~
               attribute_len: p!(be_u32) ~
               attribute: c!(attribute_info, attribute_name_index, attribute_len, constant_pool),
               || attribute)))
}

fn field<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                 -> ParseResult<'a, FieldInfo> {
    p_wrap_nom!(input, p_cut!(
        Error::FieldInfo,
        chain!(access_flags: p!(be_u16) ~
               name_index: c!(cp_index) ~
               descriptor_index: c!(cp_index) ~
               attributes_count: p!(be_u16) ~
               attributes: p_cut!(
                   Error::FieldAttributes { attributes_count: attributes_count as usize },
                   count!(c!(attribute, constant_pool), attributes_count as usize)),
               || FieldInfo {
                   access_flags: access_flags,
                   name_index: name_index,
                   descriptor_index: descriptor_index,
                   attributes: attributes,
               })))
}

fn method<'a, 'b>(input: Input<'a>, constant_pool: &'b ConstantPool)
                 -> ParseResult<'a, MethodInfo> {
    p_wrap_nom!(input, p_cut!(
        Error::MethodInfo,
        chain!(access_flags: p!(be_u16) ~
               name_index: c!(cp_index) ~
               descriptor_index: c!(cp_index) ~
               attributes_count: p!(be_u16) ~
               attributes: p_cut!(
                   Error::MethodAttributes { attributes_count: attributes_count as usize },
                   count!(c!(attribute, constant_pool), attributes_count as usize)),
               || MethodInfo {
                   access_flags: access_flags,
                   name_index: name_index,
                   descriptor_index: descriptor_index,
                   attributes: attributes,
               })))
}

macro_rules! constant_pool_special_count {
    ($input: expr, $submac:ident ! ( $($args: tt)* ), $count: expr) => ({
        let mut entries = Vec::with_capacity($count);
        let mut i = 0;
        let mut input = $input;
        while i < $count {
            let (next_input, entry) = p_try!(input, p_wrap_nom!(
                p_cut!(Error::ConstantPoolEntry { index: i }, $submac!($($args)*))));
            input = next_input;
            match entry {
                ConstantPoolInfo::Long { .. } | ConstantPoolInfo::Double { .. } => {
                    entries.push(entry);
                    entries.push(ConstantPoolInfo::Unusable);
                    i += 2;
                },
                _ => {
                    entries.push(entry);
                    i += 1;
                },
            }
        }
        done!(input, entries)
    });
}

/// `parser::class_file::parse_class_file(&[u8]) -> ParseResult<model::class_file::ClassFile>)`
n!(class_file_parser<Input, ClassFile, Error>, p_cut!(
    Error::ClassFile,
    chain!(c!(magic) ~
           minor_version: p!(be_u16) ~
           major_version: p!(be_u16) ~
           constant_pool_count: p!(be_u16) ~
           constant_pool: p_cut!( // TODO: Verify validity of constant pool
               Error::ConstantPool {
                   constant_pool_count: constant_pool_count as usize
               },
               map!(constant_pool_special_count!(c!(cp_info), constant_pool_count as usize - 1),
                    ConstantPool::from_zero_indexed_vec)) ~
           access_flags: p!(be_u16) ~
           this_class: c!(cp_index_tag, &constant_pool, constant_pool::Tag::Class) ~
           super_class: c!(maybe_cp_index_tag, &constant_pool, constant_pool::Tag::Class) ~
           interfaces_count: p!(be_u16) ~
           interfaces: p_cut!(Error::Interfaces { interfaces_count: interfaces_count as usize },
                              count!(c!(cp_index), interfaces_count as usize)) ~
           fields_count: p!(be_u16) ~
           fields: p_cut!(Error::Fields { fields_count: fields_count as usize },
                          count!(c!(field, &constant_pool), fields_count as usize)) ~
           methods_count: p!(be_u16) ~
           methods: p_cut!(Error::Methods { methods_count: methods_count as usize },
                           count!(c!(method, &constant_pool), methods_count as usize)) ~
           attributes_count: p!(be_u16) ~
           attributes: p_cut!(
               Error::ClassAttributes { attributes_count: attributes_count as usize },
               count!(c!(attribute, &constant_pool), attributes_count as usize)),
           || ClassFile {
               minor_version: minor_version,
               major_version: major_version,
               constant_pool: constant_pool,
               access_flags: access_flags,
               this_class: this_class,
               super_class: super_class,
               interfaces: interfaces,
               fields: fields,
               methods: methods,
               attributes: attributes,
           })));

/// Parses a Java class file.
pub fn parse_class_file(input: Input) -> nom::IResult<Input, ClassFile, Error> {
    match class_file_parser(input) {
        Ok(r) => r,
        Err(e) => nom::IResult::Error(e),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hello_world() {
        let data = include_bytes!("../../data/HelloWorld.class");
        assert!(parse_class_file(data).is_done());
    }

    #[test]
    fn test_java_lang_string() {
        let data = include_bytes!("../../data/String.class"); // java.lang.String
        match parse_class_file(data) {
            ::nom::IResult::Done(_, class) => assert_eq!(536, class.constant_pool.len()),
            _ => panic!("Failed to parse."),
        }
    }

}
