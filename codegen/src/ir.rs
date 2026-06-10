//! Definition IR: plain Rust data describing a MAC message layout,
//! one item per spec figure row.

/// One message definition; the generator emits one module per def.
#[derive(Clone, Copy)]
pub struct MessageDef {
    /// Generated module name, e.g. "route_info".
    pub module: &'static str,
    /// Rust type name, e.g. "RouteInfoParts".
    pub name: &'static str,
    /// Spec reference, e.g.
    /// "ETSI TS 103 636-4, clause 6.4.3.2, Figure 6.4.3.2-1".
    pub spec: &'static str,
    /// Rustdoc headline for the struct.
    pub doc: &'static str,
    /// IEType6bit variant name for a MessageBody impl.
    pub ie_type: Option<&'static str>,
    /// ShortIeType expression for a ShortMessageBody impl.
    pub short_ie: Option<&'static str>,
    /// Extra `use` lines for types that live outside `crate::types`,
    /// e.g. a variant enum kept in the handwritten wrapper module.
    pub imports: &'static [&'static str],
    /// Context parameters: threaded into `parse()` after the buffer
    /// and stored as leading struct fields (e.g. `mu` selecting a
    /// field width on the wire).
    pub ctx: &'static [Ctx],
    /// Struct fields composed from several prefix leaf fields
    /// (e.g. a capability block interleaved with parent flags).
    pub field_groups: &'static [FieldGroup],
    /// The layout, in wire order.
    pub items: &'static [Item],
}

/// One struct field built from multiple prefix leaves. The leaves
/// stay in the wire layout; the struct stores them as one value.
#[derive(Clone, Copy)]
pub struct FieldGroup {
    /// Struct field name.
    pub name: &'static str,
    /// Stored type (declared outside the generated module).
    pub ty: &'static str,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
    /// Leaf field names composing the value, in wire order.
    pub leaves: &'static [&'static str],
    /// Construction expression over the leaf-name locals.
    pub construct: &'static str,
    /// Serialize access for each leaf (e.g.
    /// "self.base_phy.rx_gain"), parallel to `leaves`.
    pub accessors: &'static [&'static str],
}

/// One parse-context parameter, stored as a struct field.
#[derive(Clone, Copy)]
pub struct Ctx {
    /// Rust parameter / field name.
    pub name: &'static str,
    /// Rust type name.
    pub ty: &'static str,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
}

/// One layout item (spec figure row or part of one).
#[derive(Clone, Copy)]
pub enum Item {
    /// A plain field.
    Field(Field),
    /// Reserved bits: serialized as zero, ignored on parse.
    Reserved { bits: u8 },
    /// One bitmap bit: set iff the named [`Item::Optional`] is
    /// present. Not stored; serialize derives it from
    /// `Option::is_some`.
    PresenceFlag { of: &'static str, fig: &'static str },
    /// Count bits for the named [`Item::Repeat`]. The on-wire value
    /// is `len - bias`.
    Count {
        of: &'static str,
        bits: u8,
        fig: &'static str,
    },
    /// Selector bit for the named [`Item::Switch`] (wide iff the
    /// value overflows the narrow form) or [`Item::Variant`] (set
    /// selects the wide arm). Not stored.
    Selector { of: &'static str, fig: &'static str },
    /// Optional region, present iff its [`Item::PresenceFlag`] bit
    /// is set.
    Optional(Group),
    /// Repeated region; element count comes from its [`Item::Count`].
    Repeat(Repeat),
    /// A field stored as the wide integer type but serialized in a
    /// narrow or wide on-wire form.
    Switch(Switch),
    /// Two-arm enum field whose arms have different widths, selected
    /// by an [`Item::Selector`] bit.
    Variant(Variant),
    /// Constant bits (an arm's discriminant value): serialized as the
    /// given value, ignored on parse (the dispatch already matched).
    Const {
        bits: u8,
        value: u8,
        fig: &'static str,
    },
    /// Mandatory composite group: an optional leading [`Item::Switch`]
    /// followed by static fields, stored as one composite value.
    Inline(Group),
    /// Mode bits in the bitmap gating an [`Item::Optional`]: zero
    /// means absent, a reserved code is rejected, and any other value
    /// is stored inside the optional's composite (so the composite's
    /// construct expression can use the `mode` local).
    ModeTag(ModeTag),
    /// Zero-copy tail: the rest of the buffer borrowed as a slice of
    /// a `#[repr(transparent)]`-over-u8 element type. Must be the
    /// last item; gives the struct a lifetime parameter.
    TailSlice(TailSlice),
    /// Zero-copy slice whose length comes from an [`Item::Count`].
    /// Must be the last item; gives the struct a lifetime parameter.
    CountSlice(CountSlice),
    /// Enum-shaped body: leading discriminant bits select an arm with
    /// its own layout. Must be the only item. With ctx parameters the
    /// generated struct stores the enum as a field; without, the
    /// wrapper-defined enum itself carries the codec impl.
    VariantBody(VariantBody),
}

/// An enum body dispatched on leading discriminant bits.
#[derive(Clone, Copy)]
pub struct VariantBody {
    /// Struct field name when ctx parameters exist (e.g. "kind").
    pub name: &'static str,
    /// Enum type (declared in the wrapper module).
    pub ty: &'static str,
    /// Rustdoc for the struct field (unused in whole-enum form).
    pub doc: &'static str,
    /// Discriminant width in bits, at the start of byte 0.
    pub bits: u8,
    /// Figure label for the discriminant.
    pub fig: &'static str,
    pub arms: &'static [VArm],
}

/// One [`VariantBody`] arm.
#[derive(Clone, Copy)]
pub struct VArm {
    /// Discriminant value selecting this arm.
    pub value: u8,
    /// Match pattern for serialization, binding the payload (e.g.
    /// "ResourceAllocationKind::Downlink { pair, options }").
    pub pattern: &'static str,
    /// Match pattern for encoded_len, binding only what the length
    /// computation reads (e.g. "..Downlink { options, .. }").
    pub len_pattern: &'static str,
    /// Stored-name to binding-path substitutions applied to the
    /// arm's serialize / encoded_len code (e.g. ("add",
    /// "options.add")). Names not listed stay `self.`-rooted.
    pub binds: &'static [(&'static str, &'static str)],
    /// Parse-side construction over the arm's locals, e.g.
    /// "ResourceAllocationKind::ReleaseAll".
    pub construct: &'static str,
    /// The arm's wire layout. Must start with an [`Item::Const`]
    /// carrying the discriminant.
    pub items: &'static [Item],
}

/// Nonzero mode bits gating an optional region.
#[derive(Clone, Copy)]
pub struct ModeTag {
    /// Name of the gated [`Item::Optional`].
    pub of: &'static str,
    /// Bit width of the mode field.
    pub bits: u8,
    /// Short label for the ASCII figure.
    pub fig: &'static str,
    /// Mode type and fallible constructor (rejecting reserved codes).
    pub ty: &'static str,
    pub ctor: &'static str,
    /// Raw-value access on the stored composite binding, e.g.
    /// "mode.as_u8()".
    pub accessor: &'static str,
}

/// A borrowed `&'a [T]` of [`Item::Count`]-determined length.
#[derive(Clone, Copy)]
pub struct CountSlice {
    /// Struct field name.
    pub name: &'static str,
    /// Short label for the ASCII figure.
    pub fig: &'static str,
    /// Element type: `#[repr(transparent)]` over `u8` with no
    /// validity invariant. Declared outside the generated module and
    /// pulled in via [`MessageDef::imports`].
    pub ty: &'static str,
    /// Method recovering the raw byte for serialization.
    pub getter: &'static str,
    /// What the all-ones count value means.
    pub all_ones: AllOnes,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
}

/// Owned all-escape mapping for a [`Repeat`].
#[derive(Clone, Copy)]
pub struct AllEscape {
    pub ty: &'static str,
    pub all: &'static str,
    pub specific: &'static str,
}

/// Meaning of the all-ones value of a [`CountSlice`]'s count field.
#[derive(Clone, Copy)]
pub enum AllOnes {
    /// Reserved: parse rejects it; serialize rejects a slice that
    /// long. The struct field is the bare slice.
    Reserved,
    /// An "all" escape: the struct field is a two-variant enum
    /// (`<ty><'a>`); all-ones maps to `all`, any other count to
    /// `specific` holding the slice.
    All {
        ty: &'static str,
        all: &'static str,
        specific: &'static str,
    },
}

/// A borrowed `&'a [T]` covering the remainder of the body.
#[derive(Clone, Copy)]
pub struct TailSlice {
    /// Struct field name.
    pub name: &'static str,
    /// Short label for the ASCII figure.
    pub fig: &'static str,
    /// Element type: `#[repr(transparent)]` over `u8` with no
    /// validity invariant (any byte is representable). Declared in
    /// the wrapper module and pulled in via [`MessageDef::imports`].
    pub ty: &'static str,
    /// Method recovering the raw byte for serialization.
    pub getter: &'static str,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
}

/// A single fixed-width field.
#[derive(Clone, Copy)]
pub struct Field {
    /// Rust field name.
    pub name: &'static str,
    /// Short label for the ASCII figure; falls back to `name`.
    pub fig: Option<&'static str>,
    /// Bit width (1..=32). Widths 9..=15 must end on a byte boundary
    /// and share their bytes only with reserved bits.
    pub bits: u8,
    /// Rust-type mapping.
    pub ty: Ty,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
}

/// An optional region: zero or more reserved runs plus either one or
/// more fields (composite) or a single width-switched field.
#[derive(Clone, Copy)]
pub struct Group {
    /// Struct field name; the stored type is `Option<...>`.
    pub name: &'static str,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
    /// Field / Reserved items, or exactly one [`Item::Switch`], or
    /// Field / Reserved items followed by single-field nested
    /// [`Item::Optional`]s (whose presence flags use the dotted
    /// `"parent.child"` form).
    pub items: &'static [Item],
    /// Composite mapping when the group holds more than one field.
    pub composite: Option<Composite>,
}

/// Maps a multi-field group to one stored composite value.
#[derive(Clone, Copy)]
pub struct Composite {
    /// Stored type, e.g. "RadioDeviceClass" or
    /// "(LoadPercentage, LoadPercentage)".
    pub ty: &'static str,
    /// Construction expression over the parsed field-name locals,
    /// e.g. "RadioDeviceClass { mu, beta }" or "(free, busy)".
    pub construct: &'static str,
    /// Per-field access from the group binding `v` for serialization,
    /// parallel to the group's fields, e.g. &["v.mu", "v.beta"].
    pub accessors: &'static [&'static str],
}

/// A repeated region holding a `heapless::Vec` of one element field.
#[derive(Clone, Copy)]
pub struct Repeat {
    /// Struct field name.
    pub name: &'static str,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
    /// Element layout: Field / Reserved items with exactly one field.
    pub items: &'static [Item],
    /// Composite mapping when the element holds more than one field
    /// (accessors are rooted at the loop binding `v`).
    pub composite: Option<Composite>,
    /// On-wire count = `len - bias`; a nonzero bias makes the empty
    /// vector unrepresentable (serialize rejects it).
    pub bias: u8,
    /// The all-ones count value is reserved: parse rejects it and the
    /// capacity const shrinks by one.
    pub reserved_max: bool,
    /// All-escape: the struct field is a two-variant enum; the
    /// all-ones count maps to `all` (no elements on the wire), any
    /// other count to `specific` holding the vector. Shrinks the
    /// capacity const by one.
    pub all_escape: Option<AllEscape>,
    /// Name of the emitted `pub const MAX_*: usize` capacity.
    pub max_const: &'static str,
    /// Rustdoc for the capacity const.
    pub max_doc: &'static str,
}

/// A field serialized as either a narrow or a wide integer.
#[derive(Clone, Copy)]
pub struct Switch {
    /// The field; `bits` is the wide width and selects the stored
    /// integer type.
    pub field: Field,
    /// Narrow on-wire width in bits (whole bytes).
    pub narrow_bits: u8,
    /// Wide on-wire width in bits (9..=16; widths below 16 read two
    /// bytes masked, with the spare high bits reserved).
    pub wide_bits: u8,
    /// What selects the wide form.
    pub wide: Wide,
}

/// Wide-form selection rule for a [`Switch`].
#[derive(Clone, Copy)]
pub enum Wide {
    /// Wide iff the value exceeds the narrow maximum. Requires an
    /// [`Item::Selector`] bit; serialization derives both the bit
    /// and the width from the value, so it cannot fail.
    ValueOverflow,
    /// Wide iff this context expression holds. `{name}` placeholders
    /// are replaced with the ctx parameter access. Serializing a
    /// value that overflows the narrow form is an ExcessiveBitsSet
    /// error.
    Ctx(&'static str),
}

/// Two-arm variant field, e.g. a Short / Long RD ID.
#[derive(Clone, Copy)]
pub struct Variant {
    /// Struct field name.
    pub name: &'static str,
    /// Short label for the ASCII figure.
    pub fig: &'static str,
    /// Stored enum type (defined in the wrapper module and pulled in
    /// via [`MessageDef::imports`]).
    pub ty: &'static str,
    /// Rustdoc for the struct field.
    pub doc: &'static str,
    /// Arm when the selector bit is clear.
    pub narrow: Arm,
    /// Arm when the selector bit is set.
    pub wide: Arm,
}

/// One [`Variant`] arm.
#[derive(Clone, Copy)]
pub struct Arm {
    /// Variant path, e.g. "BroadcastRdId::Short".
    pub path: &'static str,
    /// On-wire width in bits (16 or 32).
    pub bits: u8,
    /// Inner-value mapping.
    pub ty: Ty,
}

/// How the raw bits map to the struct field's Rust type.
#[derive(Clone, Copy)]
pub enum Ty {
    /// `bool`; bits must be 1.
    Bool,
    /// Raw unsigned integer (u8/u16/u32 chosen from the bit width).
    Raw,
    /// Fallible constructor: `<ty>::<ctor>(raw)` returns Option;
    /// `None` maps to ParsingError::ReservedValue. `getter` recovers
    /// the raw value for serialization (e.g. "as_u8").
    Fallible {
        ty: &'static str,
        ctor: &'static str,
        getter: &'static str,
    },
    /// Infallible wrapper. `construct` / `deconstruct` are format
    /// snippets where `{}` is the raw value / the field access
    /// expression respectively (e.g. "RouteCost({})" / "{}.0").
    Wrap {
        ty: &'static str,
        construct: &'static str,
        deconstruct: &'static str,
    },
}

impl Field {
    /// Figure label (explicit or field name).
    pub fn fig_label(&self) -> &'static str {
        self.fig.unwrap_or(self.name)
    }

    /// The Rust type name for the struct field.
    pub fn rust_ty(&self) -> String {
        match &self.ty {
            Ty::Bool => "bool".into(),
            Ty::Raw => raw_int(self.bits).into(),
            Ty::Fallible { ty, .. } | Ty::Wrap { ty, .. } => (*ty).into(),
        }
    }
}

/// Smallest unsigned integer type holding `bits`.
pub fn raw_int(bits: u8) -> &'static str {
    match bits {
        1..=8 => "u8",
        9..=16 => "u16",
        17..=32 => "u32",
        _ => panic!("field wider than 32 bits"),
    }
}
