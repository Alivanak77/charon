#![allow(dead_code)]

pub use crate::gast::TraitItemName;
use crate::meta::Meta;
use crate::names::TypeName;
use crate::regions_hierarchy::RegionGroups;
pub use crate::types_utils::*;
use crate::values::Literal;
use derivative::Derivative;
use macros::{
    generate_index_type, EnumAsGetters, EnumIsA, EnumToGetters, VariantIndexArity, VariantName,
};
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

/// Region as used in a function's signatures (in which case we use region variable
/// ids) and in symbolic variables and projections (in which case we use region
/// ids).
#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, EnumIsA, EnumAsGetters, Serialize,
)]
pub enum Region<Rid> {
    /// Static region
    Static,
    /// Non-static region.
    Var(Rid),
}

/// The type of erased regions. See [`Ty`](Ty) for more explanations.
/// We could use `()`, but having a dedicated type makes things more explicit.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Hash, Ord, PartialOrd)]
pub enum ErasedRegion {
    Erased,
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
    FnPointer(Box<ETy>),
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
    Unsolved(TraitDeclId::Id, RGenericArgs),
    /// For error reporting
    Unknown(String),
}

/// A reference to a trait
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TraitRef<R> {
    pub trait_id: TraitInstanceId,
    pub generics: GenericArgs<R>,
    /// Not necessary, but useful
    pub trait_decl_ref: TraitDeclRef<R>,
}

pub type ETraitRef = TraitRef<ErasedRegion>;
pub type RTraitRef = TraitRef<Region<RegionId::Id>>;

/// Reference to a trait declaration.
///
/// About the generics, if we write:
/// ```text
/// impl Foo<bool> for String { ... }
/// ```
///
/// The substitution is: `[String, bool]`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TraitDeclRef<R> {
    pub trait_id: TraitDeclId::Id,
    pub generics: GenericArgs<R>,
}

pub type ETraitDeclRef = TraitDeclRef<ErasedRegion>;
pub type RTraitDeclRef = TraitDeclRef<Region<RegionId::Id>>;

/// .0 outlives .1
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OutlivesPred<T, U>(pub T, pub U);

pub type RegionOutlives = OutlivesPred<Region<RegionId::Id>, Region<RegionId::Id>>;
pub type TypeOutlives = OutlivesPred<RTy, Region<RegionId::Id>>;

/// A constraint over a trait associated type.
///
/// Example:
/// ```text
/// T : Foo<S = String>
///         ^^^^^^^^^^
/// ```
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TraitTypeConstraint<R> {
    pub trait_ref: TraitRef<R>,
    pub generics: GenericArgs<R>,
    pub type_name: TraitItemName,
    pub ty: Ty<R>,
}

pub type RTraitTypeConstraint = TraitTypeConstraint<Region<RegionId::Id>>;

/// The predicates which apply to a definition
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Predicates {
    /// The first region in the pair outlives the second region
    pub regions_outlive: Vec<RegionOutlives>,
    /// The type outlives the region
    pub types_outlive: Vec<TypeOutlives>,
    /// Constraints over trait associated types
    pub trait_type_constraints: Vec<RTraitTypeConstraint>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Hash, Ord, PartialOrd)]
pub struct GenericArgs<R> {
    pub regions: Vec<R>,
    pub types: Vec<Ty<R>>,
    pub const_generics: Vec<ConstGeneric>,
    // TODO: rename to match [GenericParams]?
    pub trait_refs: Vec<TraitRef<R>>,
}

pub type EGenericArgs = GenericArgs<ErasedRegion>;
pub type RGenericArgs = GenericArgs<Region<RegionId::Id>>;

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
    pub trait_clauses: TraitClauseId::Vector<TraitClause>,
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
    pub generics: RGenericArgs,
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
    pub meta: Meta,
    /// [true] if the type decl is a local type decl, [false] if is comes from
    /// an external crate.
    pub is_local: bool,
    pub name: TypeName,
    pub generics: GenericParams,
    pub preds: Predicates,
    /// The type kind: enum, struct, or opaque.
    pub kind: TypeDeclKind,
    /// The lifetime's hierarchy between the different regions.
    /// We initialize it to a dummy value, then compute it once the whole crate
    /// has been translated.
    ///
    /// TODO: move to Aeneas
    pub regions_hierarchy: RegionGroups,
}

#[derive(Debug, Clone, EnumIsA, EnumAsGetters, Serialize)]
pub enum TypeDeclKind {
    Struct(FieldId::Vector<Field>),
    Enum(VariantId::Vector<Variant>),
    /// An opaque type.
    ///
    /// Either a local type marked as opaque, or an external type.
    Opaque,
}

#[derive(Debug, Clone, Serialize)]
pub struct Variant {
    pub meta: Meta,
    pub name: String,
    pub fields: FieldId::Vector<Field>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub meta: Meta,
    pub name: Option<String>,
    pub ty: RTy,
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
///
/// Types are parameterized by a type parameter used for regions (or lifetimes).
/// The reason is that in MIR, regions are used in the function signatures but
/// are erased in the function bodies. We make this extremely explicit (and less
/// error prone) in our encoding by using two different types: [`Region`](Region)
/// and [`ErasedRegion`](ErasedRegion), the latter being an enumeration with only
/// one variant.
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
pub enum Ty<R> {
    /// An ADT.
    /// Note that here ADTs are very general. They can be:
    /// - user-defined ADTs
    /// - tuples (including `unit`, which is a 0-tuple)
    /// - assumed types (includes some primitive types, e.g., arrays or slices)
    /// The information on the nature of the ADT is stored in (`TypeId`)[TypeId].
    /// The last list is used encode const generics, e.g., the size of an array
    Adt(TypeId, GenericArgs<R>),
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
    /// TODO: but do we really use this type for variables?...
    Never,
    // We don't support floating point numbers on purpose (for now)
    /// A borrow
    Ref(R, Box<Ty<R>>, RefKind),
    /// A raw pointer.
    RawPtr(Box<Ty<R>>, RefKind),
    /// A trait type
    TraitType(TraitRef<R>, GenericArgs<R>, TraitItemName),
    /// Arrow type
    Arrow(Vec<Ty<R>>, Box<Ty<R>>),
}

/// Type with *R*egions.
///
/// Used in function signatures and type definitions.
/// TODO: rename to sty (*signature* type). Region types are used by the
/// interpreter.
pub type RTy = Ty<Region<RegionId::Id>>;

/// Type with *E*rased regions.
///
/// Used in function bodies, "general" value types, etc.
pub type ETy = Ty<ErasedRegion>;

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

/// A function signature.
/// Note that a signature uses unerased lifetimes, while function bodies (and
/// execution) use erased lifetimes.
/// We need the functions' signatures *with* the region parameters in order
/// to correctly abstract those functions (number and signature of the backward
/// functions) - we only use regions for this purpose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FunSig {
    /// Is the function unsafe or not
    pub is_unsafe: bool,
    pub generics: GenericParams,
    pub preds: Predicates,
    /// Optional fields, for trait methods only (see the comments in [ParamsInfo]).
    pub parent_params_info: Option<ParamsInfo>,
    pub inputs: Vec<RTy>,
    pub output: RTy,
    /// The lifetime's hierarchy between the different regions.
    /// We initialize it to a dummy value, and compute it once the whole
    /// crate has been translated from MIR.
    ///
    /// TODO: move to Aeneas.
    pub regions_hierarchy: RegionGroups,
}
