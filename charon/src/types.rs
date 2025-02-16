pub use crate::gast::{FunDeclId, TraitItemName};
use crate::meta::{ItemMeta, Meta};
use crate::names::Name;
pub use crate::types_utils::*;
use crate::values::Literal;
use derivative::Derivative;
use macros::{EnumAsGetters, EnumIsA, EnumToGetters, VariantIndexArity, VariantName};
use serde::Serialize;

pub type FieldName = String;

// We need to manipulate a lot of indices for the types, variables, definitions,
// etc. In order not to confuse them, we define an index type for every one of
// them (which is just a struct with a unique usize field), together with some
// utilities like a fresh index generator. Those structures and utilities are
// generated by using macros.
generate_index_type!(TypeVarId);
generate_index_type!(TypeDeclId);
generate_index_type!(VariantId);
generate_index_type!(FieldId);
generate_index_type!(RegionId);
generate_index_type!(ConstGenericVarId);
generate_index_type!(GlobalDeclId);

/// Type variable.
/// We make sure not to mix variables and type variables by having two distinct
/// definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeVar {
    /// Unique index identifying the variable
    pub index: TypeVarId::Id,
    /// Variable name
    pub name: String,
}

/// Region variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash, PartialOrd, Ord)]
pub struct RegionVar {
    /// Unique index identifying the variable
    pub index: RegionId::Id,
    /// Region name
    pub name: Option<String>,
}

/// Const Generic Variable
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstGenericVar {
    /// Unique index identifying the variable
    pub index: ConstGenericVarId::Id,
    /// Const generic name
    pub name: String,
    /// Type of the const generic
    pub ty: LiteralTy,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct DeBruijnId {
    pub index: usize,
}

#[derive(
    Debug, PartialEq, Eq, Copy, Clone, Hash, PartialOrd, Ord, EnumIsA, EnumAsGetters, Serialize,
)]
pub enum Region {
    /// Static region
    Static,
    /// Bound region variable.
    ///
    /// **Important**:
    /// ==============
    /// Similarly to what the Rust compiler does, we use De Bruijn indices to
    /// identify *groups* of bound variables, and variable identifiers to
    /// identity the variables inside the groups.
    ///
    /// For instance, we have the following:
    /// ```text
    ///                     we compute the De Bruijn indices from here
    ///                            VVVVVVVVVVVVVVVVVVVVVVV
    /// fn f<'a, 'b>(x: for<'c> fn(&'a u8, &'b u16, &'c u32) -> u64) {}
    ///      ^^^^^^         ^^       ^       ^        ^
    ///        |      De Bruijn: 0   |       |        |
    ///  De Bruijn: 1                |       |        |
    ///                        De Bruijn: 1  |    De Bruijn: 0
    ///                           Var id: 0  |       Var id: 0
    ///                                      |
    ///                                De Bruijn: 1
    ///                                   Var id: 1
    /// ```
    BVar(DeBruijnId, RegionId::Id),
    /// Erased region
    Erased,
    /// For error reporting.
    Unknown,
}

/// Identifier of a trait instance.
/// This is derived from the trait resolution.
///
/// Should be read as a path inside the trait clauses which apply to the current
/// definition. Note that every path designated by [TraitInstanceId] refers
/// to a *trait instance*, which is why the [Clause] variant may seem redundant
/// with some of the other variants.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TraitInstanceId {
    ///
    /// A specific implementation
    TraitImpl(TraitImplId::Id),
    ///
    /// A specific builtin trait implementation like [core::marker::Sized] or
    /// auto trait implementation like [core::marker::Syn].
    BuiltinOrAuto(TraitDeclId::Id),
    ///
    /// One of the local clauses.
    ///
    /// Example:
    /// ```text
    /// fn f<T>(...) where T : Foo
    ///                    ^^^^^^^
    ///                    Clause(0)
    /// ```
    Clause(TraitClauseId::Id),
    ///
    /// A parent clause
    ///
    /// Remark: the [TraitDeclId::Id] gives the trait declaration which is
    /// implemented by the instance id from which we take the parent clause
    /// (see example below). It is not necessary and included for convenience.
    ///
    /// Example:
    /// ```text
    /// trait Foo1 {}
    /// trait Foo2 { fn f(); }
    ///
    /// trait Bar : Foo1 + Foo2 {}
    ///             ^^^^   ^^^^
    ///                    parent clause 1
    ///     parent clause 0
    ///
    /// fn g<T : Bar>(x : T) {
    ///   x.f()
    ///   ^^^^^
    ///   Parent(Clause(0), Bar, 1)::f(x)
    ///                          ^
    ///                          parent clause 1 of clause 0
    ///                     ^^^
    ///              clause 0 implements Bar
    /// }
    /// ```
    ParentClause(Box<TraitInstanceId>, TraitDeclId::Id, TraitClauseId::Id),
    ///
    /// A clause bound in a trait item (typically a trait clause in an
    /// associated type).
    ///
    /// Remark: the [TraitDeclId::Id] gives the trait declaration which is
    /// implemented by the trait implementation from which we take the item
    /// (see below). It is not necessary and provided for convenience.
    ///
    /// Example:
    /// ```text
    /// trait Foo {
    ///   type W: Bar0 + Bar1 // Bar1 contains a method bar1
    ///                  ^^^^
    ///               this is the clause 1 applying to W
    /// }
    ///
    /// fn f<T : Foo>(x : T::W) {
    ///   x.bar1();
    ///   ^^^^^^^
    ///   ItemClause(Clause(0), Foo, W, 1)
    ///                              ^^^^
    ///                              clause 1 from item W (from local clause 0)
    ///                         ^^^
    ///                local clause 0 implements Foo
    /// }
    /// ```
    ///
    ///
    ItemClause(
        Box<TraitInstanceId>,
        TraitDeclId::Id,
        TraitItemName,
        TraitClauseId::Id,
    ),
    /// Happens when we use a function pointer as an object implementing a
    /// trait like `Fn` or `FnMut`.
    ///
    /// ```text
    /// fn incr(x : u32) -> u32 { ... }
    ///
    /// Example:
    /// fn f(a: [u32; 32]) -> [u32; 32] {
    ///   a.map(incr)
    /// }
    /// ```
    FnPointer(Box<Ty>),
    /// Similar to [FnPointer], but where we use a closure.
    ///
    /// It is important to differentiate the cases, because closures have a
    /// state. Whenever we create a closure, we actually create an aggregated
    /// value with a function pointer and a state.
    Closure(FunDeclId::Id, GenericArgs),
    ///
    /// Self, in case of trait declarations/implementations.
    ///
    /// Putting [Self] at the end on purpose, so that when ordering the clauses
    /// we start with the other clauses (in particular, the local clauses). It
    /// is useful to give priority to the local clauses when solving the trait
    /// obligations which are fullfilled by the trait parameters.
    SelfId,
    /// Clause which hasn't been solved yet.
    /// This happens when we register clauses in the context: solving some
    /// trait obligations/references might require to refer to clauses which
    /// haven't been registered yet. This variant is purely internal: after we
    /// finished solving the trait obligations, all the remaining unsolved
    /// clauses (in case we don't fail hard on error) are converted to [Unknown].
    Unsolved(TraitDeclId::Id, GenericArgs),
    /// For error reporting.
    /// Can appear only if the option [CliOpts::continue_on_failure] is used.
    Unknown(String),
}

/// A reference to a trait
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TraitRef {
    pub trait_id: TraitInstanceId,
    pub generics: GenericArgs,
    /// Not necessary, but useful
    pub trait_decl_ref: TraitDeclRef,
}

/// Reference to a trait declaration.
///
/// About the generics, if we write:
/// ```text
/// impl Foo<bool> for String { ... }
/// ```
///
/// The substitution is: `[String, bool]`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TraitDeclRef {
    pub trait_id: TraitDeclId::Id,
    pub generics: GenericArgs,
}

/// .0 outlives .1
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OutlivesPred<T, U>(pub T, pub U);

pub type RegionOutlives = OutlivesPred<Region, Region>;
pub type TypeOutlives = OutlivesPred<Ty, Region>;

/// A constraint over a trait associated type.
///
/// Example:
/// ```text
/// T : Foo<S = String>
///         ^^^^^^^^^^
/// ```
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TraitTypeConstraint {
    pub trait_ref: TraitRef,
    pub type_name: TraitItemName,
    pub ty: Ty,
}

/// The predicates which apply to a definition
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Predicates {
    /// The first region in the pair outlives the second region
    pub regions_outlive: Vec<RegionOutlives>,
    /// The type outlives the region
    pub types_outlive: Vec<TypeOutlives>,
    /// Constraints over trait associated types
    pub trait_type_constraints: Vec<TraitTypeConstraint>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Hash, Ord, PartialOrd)]
pub struct GenericArgs {
    pub regions: Vec<Region>,
    pub types: Vec<Ty>,
    pub const_generics: Vec<ConstGeneric>,
    // TODO: rename to match [GenericParams]?
    pub trait_refs: Vec<TraitRef>,
}

/// Generic parameters for a declaration.
/// We group the generics which come from the Rust compiler substitutions
/// (the regions, types and const generics) as well as the trait clauses.
/// The reason is that we consider that those are parameters that need to
/// be filled. We group in a different place the predicates which are not
/// trait clauses, because those enforce constraints but do not need to
/// be filled with witnesses/instances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GenericParams {
    pub regions: RegionId::Vector<RegionVar>,
    pub types: TypeVarId::Vector<TypeVar>,
    pub const_generics: ConstGenericVarId::Vector<ConstGenericVar>,
    // TODO: rename to match [GenericArgs]?
    // Remark: we use a regular [Vec], not a [TraitClauseId::Vector], because due to the
    // filtering of some trait clauses (for the marker traits for instance) the indexation
    // is not contiguous (e.g., we may have [clause 0; clause 3; clause 4]).
    pub trait_clauses: Vec<TraitClause>,
}

generate_index_type!(TraitClauseId);
generate_index_type!(TraitDeclId);
generate_index_type!(TraitImplId);

#[derive(Debug, Clone, Serialize, Derivative)]
#[derivative(PartialEq)]
pub struct TraitClause {
    /// We use this id when solving trait constraints, to be able to refer
    /// to specific where clauses when the selected trait actually is linked
    /// to a parameter.
    pub clause_id: TraitClauseId::Id,
    #[derivative(PartialEq = "ignore")]
    pub meta: Option<Meta>,
    pub trait_id: TraitDeclId::Id,
    /// Remark: the trait refs list in the [generics] field should be empty.
    pub generics: GenericArgs,
}

impl Eq for TraitClause {}

/// A type declaration.
///
/// Types can be opaque or transparent.
///
/// Transparent types are local types not marked as opaque.
/// Opaque types are the others: local types marked as opaque, and non-local
/// types (coming from external dependencies).
///
/// In case the type is transparent, the declaration also contains the
/// type definition (see [TypeDeclKind]).
///
/// A type can only be an ADT (structure or enumeration), as type aliases are
/// inlined in MIR.
#[derive(Debug, Clone, Serialize)]
pub struct TypeDecl {
    pub def_id: TypeDeclId::Id,
    /// Meta information associated with the type.
    pub item_meta: ItemMeta,
    /// [true] if the type decl is a local type decl, [false] if it comes from
    /// an external crate.
    pub is_local: bool,
    pub name: Name,
    pub generics: GenericParams,
    pub preds: Predicates,
    /// The type kind: enum, struct, or opaque.
    pub kind: TypeDeclKind,
}

#[derive(Debug, Clone, EnumIsA, EnumAsGetters, Serialize)]
pub enum TypeDeclKind {
    Struct(FieldId::Vector<Field>),
    Enum(VariantId::Vector<Variant>),
    /// An opaque type.
    ///
    /// Either a local type marked as opaque, or an external type.
    Opaque,
    /// Used if an error happened during the extraction, and we don't panic
    /// on error.
    Error(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Variant {
    pub meta: Meta,
    pub name: String,
    pub fields: FieldId::Vector<Field>,
    #[serde(skip)]
    /// The discriminant used at runtime. This is used in `remove_read_discriminant` to match up
    /// `SwitchInt` targets with the corresponding `Variant`.
    pub discriminant: u128,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub meta: Meta,
    pub name: Option<String>,
    pub ty: Ty,
}

#[derive(
    Debug, PartialEq, Eq, Copy, Clone, EnumIsA, VariantName, Serialize, Hash, Ord, PartialOrd,
)]
pub enum IntegerTy {
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Usize,
    U8,
    U16,
    U32,
    U64,
    U128,
}

#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Hash, VariantName, EnumIsA, Serialize, Ord, PartialOrd,
)]
pub enum RefKind {
    Mut,
    Shared,
}

/// Type identifier.
///
/// Allows us to factorize the code for assumed types, adts and tuples
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    VariantName,
    EnumAsGetters,
    EnumIsA,
    Serialize,
    Hash,
    Ord,
    PartialOrd,
)]
pub enum TypeId {
    /// A "regular" ADT type.
    ///
    /// Includes transparent ADTs and opaque ADTs (local ADTs marked as opaque,
    /// and external ADTs).
    Adt(TypeDeclId::Id),
    Tuple,
    /// Assumed type. Either a primitive type like array or slice, or a
    /// non-primitive type coming from a standard library
    /// and that we handle like a primitive type. Types falling into this
    /// category include: Box, Vec, Cell...
    /// The Array and Slice types were initially modelled as primitive in
    /// the [Ty] type. We decided to move them to assumed types as it allows
    /// for more uniform treatment throughout the codebase.
    Assumed(AssumedTy),
}

pub type TypeDecls = TypeDeclId::Map<TypeDecl>;

/// Types of primitive values. Either an integer, bool, char
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    VariantName,
    EnumIsA,
    EnumAsGetters,
    VariantIndexArity,
    Serialize,
    Hash,
    Ord,
    PartialOrd,
)]
pub enum LiteralTy {
    Integer(IntegerTy),
    Bool,
    Char,
}

/// Const Generic Values. Either a primitive value, or a variable corresponding to a primitve value
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    VariantName,
    EnumIsA,
    EnumAsGetters,
    VariantIndexArity,
    Serialize,
    Hash,
    Ord,
    PartialOrd,
)]
pub enum ConstGeneric {
    /// A global constant
    Global(GlobalDeclId::Id),
    /// A const generic variable
    Var(ConstGenericVarId::Id),
    /// A concrete value
    Value(Literal),
}

/// A type.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    VariantName,
    EnumIsA,
    EnumAsGetters,
    EnumToGetters,
    VariantIndexArity,
    Serialize,
    Ord,
    PartialOrd,
)]
pub enum Ty {
    /// An ADT.
    /// Note that here ADTs are very general. They can be:
    /// - user-defined ADTs
    /// - tuples (including `unit`, which is a 0-tuple)
    /// - assumed types (includes some primitive types, e.g., arrays or slices)
    /// The information on the nature of the ADT is stored in (`TypeId`)[TypeId].
    /// The last list is used encode const generics, e.g., the size of an array
    Adt(TypeId, GenericArgs),
    TypeVar(TypeVarId::Id),
    Literal(LiteralTy),
    /// The never type, for computations which don't return. It is sometimes
    /// necessary for intermediate variables. For instance, if we do (coming
    /// from the rust documentation):
    /// ```text
    /// let num: u32 = match get_a_number() {
    ///     Some(num) => num,
    ///     None => break,
    /// };
    /// ```
    /// the second branch will have type `Never`. Also note that `Never`
    /// can be coerced to any type.
    ///
    /// Note that we eliminate the variables which have this type in a micro-pass.
    /// As statements don't have types, this type disappears eventually disappears
    /// from the AST.
    Never,
    // We don't support floating point numbers on purpose (for now)
    /// A borrow
    Ref(Region, Box<Ty>, RefKind),
    /// A raw pointer.
    RawPtr(Box<Ty>, RefKind),
    /// A trait associated type
    ///
    /// Ex.:
    /// ```text
    /// trait Foo {
    ///   type Bar; // type associated to the trait Foo
    /// }
    /// ```
    TraitType(TraitRef, TraitItemName),
    /// Arrow type, used in particular for the local function pointers.
    /// This is essentially a "constrained" function signature:
    /// arrow types can only contain generic lifetime parameters
    /// (no generic types), no predicates, etc.
    Arrow(RegionId::Vector<RegionVar>, Vec<Ty>, Box<Ty>),
}

/// Assumed types identifiers.
///
/// WARNING: for now, all the assumed types are covariant in the generic
/// parameters (if there are). Adding types which don't satisfy this
/// will require to update the code abstracting the signatures (to properly
/// take into account the lifetime constraints).
///
/// TODO: update to not hardcode the types (except `Box` maybe) and be more
/// modular.
/// TODO: move to assumed.rs?
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumIsA,
    EnumAsGetters,
    VariantName,
    Serialize,
    Hash,
    Ord,
    PartialOrd,
)]
pub enum AssumedTy {
    /// Boxes have a special treatment: we translate them as identity.
    Box,
    /// Comes from the standard library. See the comments for [Ty::RawPtr]
    /// as to why we have this here.
    PtrUnique,
    /// Same comments as for [AssumedTy::PtrUnique]
    PtrNonNull,
    /// Primitive type
    Array,
    /// Primitive type
    Slice,
    /// Primitive type
    Str,
}

/// We use this to store information about the parameters in parent blocks.
/// This is necessary because in the definitions we store *all* the generics,
/// including those coming from the outer impl block.
///
/// For instance:
/// ```text
/// impl Foo<T> {
///         ^^^
///       outer block generics
///   fn bar<U>(...)  { ... }
///         ^^^
///       generics local to the function bar
/// }
/// ```
///
/// In `bar` we store the generics: `[T, U]`.
///
/// We however sometimes need to make a distinction between those two kinds
/// of generics, in particular when manipulating traits. For instance:
///
/// ```text
/// impl<T> Foo for Bar<T> {
///   fn baz<U>(...)  { ... }
/// }
///
/// fn test(...) {
///    x.baz(...); // Here, we refer to the call as:
///                // > Foo<T>::baz<U>(...)
///                // If baz hadn't been a method implementation of a trait,
///                // we would have refered to it as:
///                // > baz<T, U>(...)
///                // The reason is that with traits, we refer to the whole
///                // trait implementation (as if it were a structure), then
///                // pick a specific method inside (as if projecting a field
///                // from a structure).
/// }
/// ```
///
/// **Remark**: Rust only allows refering to the generics of the immediately
/// outer block. For this reason, when we need to store the information about
/// the generics of the outer block(s), we need to do it only for one level
/// (this definitely makes things simpler).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParamsInfo {
    pub num_region_params: usize,
    pub num_type_params: usize,
    pub num_const_generic_params: usize,
    pub num_trait_clauses: usize,
    pub num_regions_outlive: usize,
    pub num_types_outlive: usize,
    pub num_trait_type_constraints: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum ClosureKind {
    Fn,
    FnMut,
    FnOnce,
}

/// Additional information for closures.
/// We mostly use it in micro-passes like [crate::update_closure_signature].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClosureInfo {
    pub kind: ClosureKind,
    /// Contains the types of the fields in the closure state.
    /// More precisely, for every place captured by the
    /// closure, the state has one field (typically a ref).
    ///
    /// For instance, below the closure has a state with two fields of type `&u32`:
    /// ```text
    /// pub fn test_closure_capture(x: u32, y: u32) -> u32 {
    ///   let f = &|z| x + y + z;
    ///   (f)(0)
    /// }
    /// ```
    pub state: Vec<Ty>,
}

/// A function signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FunSig {
    /// Is the function unsafe or not
    pub is_unsafe: bool,
    /// `true` if the signature is for a closure.
    ///
    /// Importantly: if the signature is for a closure, then:
    /// - the type and const generic params actually come from the parent function
    ///   (the function in which the closure is defined)
    /// - the region variables are local to the closure
    pub is_closure: bool,
    /// Additional information if this is the signature of a closure.
    pub closure_info: Option<ClosureInfo>,
    pub generics: GenericParams,
    pub preds: Predicates,
    /// Optional fields, for trait methods only (see the comments in [ParamsInfo]).
    pub parent_params_info: Option<ParamsInfo>,
    pub inputs: Vec<Ty>,
    pub output: Ty,
}
