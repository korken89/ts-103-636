//! IR -> Rust source emission. Static offsets: every fixed-prefix
//! field resolves to (byte index, shift, mask) at generation time;
//! dynamic regions (optionals, repeats, width switches, variants)
//! advance a running `pos` with per-region bounds checks. The
//! emitted code is direct indexing in the same shape as handwritten
//! codecs.

use crate::figure;
use crate::ir::{
    AllEscape, AllOnes, CountSlice, Field, FieldGroup, Group, Item, MessageDef, ModeTag, Repeat,
    Switch, TailSlice, Ty, VArm, Variant, VariantBody, Wide,
};

/// A field placed at a bit offset (absolute in the prefix, relative
/// to the region start in tail regions).
struct Slot<'a> {
    field: &'a Field,
    bit: usize,
}

/// A bitmap control (presence / count / selector) in the prefix.
enum Ctrl<'a> {
    Flag { of: &'a str, bit: usize },
    Count { of: &'a str, bit: usize, bits: u8 },
    Sel { of: &'a str, bit: usize },
    Mode { tag: &'a ModeTag, bit: usize },
    Const { bits: u8, value: u8, bit: usize },
}

/// A dynamic tail region, in wire order.
enum Region<'a> {
    /// Mandatory fields between dynamic regions.
    Fixed {
        slots: Vec<Slot<'a>>,
        bytes: usize,
    },
    Opt(&'a Group),
    Rep(&'a Repeat),
    Sw(&'a Switch),
    Var(&'a Variant),
    Tail(&'a TailSlice),
    CSlice(&'a CountSlice),
    Inline(&'a Group),
}

struct Layout<'a> {
    prefix: Vec<Slot<'a>>,
    ctrls: Vec<Ctrl<'a>>,
    prefix_bytes: usize,
    regions: Vec<Region<'a>>,
}

/// How a field sits in its byte run.
enum Kind {
    SubByte,
    Word,
    /// 9..=15 bits ending on a byte boundary, sharing its two bytes
    /// only with reserved bits.
    MaskedWord,
}

fn kind(name: &str, f: &Field, bit: usize) -> Kind {
    let o = bit % 8;
    if f.bits < 8 {
        assert!(
            o + f.bits as usize <= 8,
            "{name}::{}: sub-byte field crosses a byte boundary",
            f.name
        );
        Kind::SubByte
    } else if f.bits.is_multiple_of(8) && o == 0 {
        assert!(
            matches!(f.bits, 8 | 16 | 32),
            "{name}::{}: unsupported width {}",
            f.name,
            f.bits
        );
        Kind::Word
    } else if (9..=15).contains(&f.bits) && o + f.bits as usize == 16 {
        Kind::MaskedWord
    } else {
        panic!("{name}::{}: unsupported placement", f.name);
    }
}

fn layout<'a>(def: &'a MessageDef) -> Layout<'a> {
    let mut prefix = Vec::new();
    let mut ctrls = Vec::new();
    let mut regions: Vec<Region<'a>> = Vec::new();
    let mut bit = 0usize;
    let mut in_prefix = true;
    // Pending mandatory run between dynamic regions (relative bits).
    let mut run: Vec<Slot<'a>> = Vec::new();
    let mut run_bit = 0usize;

    let flush_run =
        |regions: &mut Vec<Region<'a>>, run: &mut Vec<Slot<'a>>, run_bit: &mut usize| {
            if !run.is_empty() || *run_bit > 0 {
                assert!(
                    (*run_bit).is_multiple_of(8),
                    "{}: fixed run not byte aligned",
                    def.name
                );
                regions.push(Region::Fixed {
                    slots: std::mem::take(run),
                    bytes: *run_bit / 8,
                });
                *run_bit = 0;
            }
        };

    for item in def.items {
        match item {
            Item::Field(f) => {
                if in_prefix {
                    kind(def.name, f, bit);
                    prefix.push(Slot { field: f, bit });
                    bit += f.bits as usize;
                } else {
                    kind(def.name, f, run_bit);
                    run.push(Slot {
                        field: f,
                        bit: run_bit,
                    });
                    run_bit += f.bits as usize;
                }
            }
            Item::Reserved { bits } => {
                if in_prefix {
                    bit += *bits as usize;
                } else {
                    run_bit += *bits as usize;
                }
            }
            Item::PresenceFlag { of, .. } => {
                assert!(
                    in_prefix,
                    "{}: flag for {of} after a dynamic item",
                    def.name
                );
                ctrls.push(Ctrl::Flag { of, bit });
                bit += 1;
            }
            Item::Count { of, bits, .. } => {
                assert!(
                    in_prefix,
                    "{}: count for {of} after a dynamic item",
                    def.name
                );
                ctrls.push(Ctrl::Count {
                    of,
                    bit,
                    bits: *bits,
                });
                bit += *bits as usize;
            }
            Item::Selector { of, .. } => {
                assert!(
                    in_prefix,
                    "{}: selector for {of} after a dynamic item",
                    def.name
                );
                ctrls.push(Ctrl::Sel { of, bit });
                bit += 1;
            }
            Item::ModeTag(tag) => {
                assert!(
                    in_prefix,
                    "{}: mode tag for {} after a dynamic item",
                    def.name, tag.of
                );
                ctrls.push(Ctrl::Mode { tag, bit });
                bit += tag.bits as usize;
            }
            Item::Const { bits, value, .. } => {
                assert!(in_prefix, "{}: const bits after a dynamic item", def.name);
                ctrls.push(Ctrl::Const {
                    bits: *bits,
                    value: *value,
                    bit,
                });
                bit += *bits as usize;
            }
            Item::VariantBody(_) => {
                panic!("{}: variant body handled by emit_message", def.name)
            }
            Item::Inline(g) => {
                end_prefix(def, &mut in_prefix, bit);
                flush_run(&mut regions, &mut run, &mut run_bit);
                validate_inline(def, g);
                regions.push(Region::Inline(g));
            }
            Item::Optional(g) => {
                end_prefix(def, &mut in_prefix, bit);
                flush_run(&mut regions, &mut run, &mut run_bit);
                validate_group(def, g);
                regions.push(Region::Opt(g));
            }
            Item::Repeat(r) => {
                end_prefix(def, &mut in_prefix, bit);
                flush_run(&mut regions, &mut run, &mut run_bit);
                validate_repeat(def, r);
                regions.push(Region::Rep(r));
            }
            Item::Switch(s) => {
                end_prefix(def, &mut in_prefix, bit);
                flush_run(&mut regions, &mut run, &mut run_bit);
                validate_switch(def, s);
                regions.push(Region::Sw(s));
            }
            Item::Variant(v) => {
                end_prefix(def, &mut in_prefix, bit);
                flush_run(&mut regions, &mut run, &mut run_bit);
                regions.push(Region::Var(v));
            }
            Item::TailSlice(ts) => {
                end_prefix(def, &mut in_prefix, bit);
                flush_run(&mut regions, &mut run, &mut run_bit);
                regions.push(Region::Tail(ts));
            }
            Item::CountSlice(cs) => {
                end_prefix(def, &mut in_prefix, bit);
                flush_run(&mut regions, &mut run, &mut run_bit);
                regions.push(Region::CSlice(cs));
            }
        }
    }
    if in_prefix {
        assert!(
            bit.is_multiple_of(8),
            "{}: layout is not byte aligned",
            def.name
        );
    }
    flush_run(&mut regions, &mut run, &mut run_bit);
    if let Some(i) = regions
        .iter()
        .position(|r| matches!(r, Region::Tail(_) | Region::CSlice(_)))
    {
        assert!(
            i == regions.len() - 1,
            "{}: a borrowed slice must be the last item",
            def.name
        );
    }

    Layout {
        prefix,
        ctrls,
        prefix_bytes: bit / 8,
        regions,
    }
}

fn end_prefix(def: &MessageDef, in_prefix: &mut bool, bit: usize) {
    if *in_prefix {
        assert!(
            bit.is_multiple_of(8),
            "{}: dynamic item at a non-byte boundary",
            def.name
        );
        *in_prefix = false;
    }
}

/// A group's static leaf run plus its trailing nested optionals.
struct GroupParts<'a> {
    statics: &'a [Item],
    nested: Vec<&'a Group>,
}

fn group_parts<'a>(def: &MessageDef, g: &'a Group) -> GroupParts<'a> {
    let split = g
        .items
        .iter()
        .position(|i| matches!(i, Item::Optional(_)))
        .unwrap_or(g.items.len());
    let nested: Vec<&Group> = g.items[split..]
        .iter()
        .map(|i| match i {
            Item::Optional(n) => n,
            _ => panic!(
                "{}::{}: only nested optionals may follow the static run",
                def.name, g.name
            ),
        })
        .collect();
    for n in &nested {
        assert!(
            n.composite.is_none()
                && !n
                    .items
                    .iter()
                    .any(|i| matches!(i, Item::Optional(_) | Item::Switch(_))),
            "{}::{}: nested optional must be a simple field group",
            def.name,
            n.name
        );
    }
    GroupParts {
        statics: &g.items[..split],
        nested,
    }
}

/// An inline group's optional leading switch plus its static run.
struct InlineParts<'a> {
    switch: Option<&'a Switch>,
    statics: &'a [Item],
}

fn inline_parts<'a>(def: &MessageDef, g: &'a Group) -> InlineParts<'a> {
    match g.items {
        [Item::Switch(sw), rest @ ..] => InlineParts {
            switch: Some(sw),
            statics: rest,
        },
        items => {
            assert!(
                !items
                    .iter()
                    .any(|i| !matches!(i, Item::Field(_) | Item::Reserved { .. })),
                "{}::{}: inline group is a leading switch plus static fields",
                def.name,
                g.name
            );
            InlineParts {
                switch: None,
                statics: items,
            }
        }
    }
}

fn validate_inline(def: &MessageDef, g: &Group) {
    let parts = inline_parts(def, g);
    if let Some(sw) = parts.switch {
        assert!(
            matches!(sw.wide, Wide::Ctx(_)),
            "{}::{}: inline switch must be ctx selected",
            def.name,
            g.name
        );
    }
    assert!(
        run_bits(parts.statics).is_multiple_of(8),
        "{}::{}: not byte aligned",
        def.name,
        g.name
    );
    let n_fields = parts
        .statics
        .iter()
        .filter(|i| matches!(i, Item::Field(_)))
        .count()
        + usize::from(parts.switch.is_some());
    let c = g
        .composite
        .as_ref()
        .unwrap_or_else(|| panic!("{}::{}: inline group needs a composite", def.name, g.name));
    assert!(
        c.accessors.len() == n_fields,
        "{}::{}: accessors do not match fields",
        def.name,
        g.name
    );
}

/// How an Optional region's presence is signalled.
enum Gate {
    Flag,
    Mode,
}

fn gate_of(def: &MessageDef, name: &str) -> Gate {
    for item in def.items {
        match item {
            Item::PresenceFlag { of, .. } if *of == name => return Gate::Flag,
            Item::ModeTag(tag) if tag.of == name => return Gate::Mode,
            _ => {}
        }
    }
    panic!("{}: no gate for {name}", def.name)
}

fn validate_group(def: &MessageDef, g: &Group) {
    if let [Item::Switch(s)] = g.items {
        assert!(
            matches!(s.wide, Wide::Ctx(_)),
            "{}::{}: switch inside an optional must be ctx selected",
            def.name,
            g.name
        );
        return;
    }
    let parts = group_parts(def, g);
    let bits = run_bits(parts.statics);
    assert!(
        bits.is_multiple_of(8),
        "{}::{}: not byte aligned",
        def.name,
        g.name
    );
    let n_fields = parts
        .statics
        .iter()
        .filter(|i| matches!(i, Item::Field(_)))
        .count();
    match &g.composite {
        None => assert!(
            n_fields == 1 && parts.nested.is_empty(),
            "{}::{}: multi-field group needs a composite",
            def.name,
            g.name
        ),
        Some(c) => assert!(
            c.accessors.len() == n_fields,
            "{}::{}: accessors do not match static fields",
            def.name,
            g.name
        ),
    }
}

fn validate_repeat(def: &MessageDef, r: &Repeat) {
    let n_fields = r
        .items
        .iter()
        .filter(|i| matches!(i, Item::Field(_)))
        .count();
    match &r.composite {
        None => assert!(
            n_fields == 1,
            "{}::{}: multi-field repeat element needs a composite",
            def.name,
            r.name
        ),
        Some(c) => assert!(
            c.accessors.len() == n_fields,
            "{}::{}: accessors do not match fields",
            def.name,
            r.name
        ),
    }
}

fn validate_switch(def: &MessageDef, s: &Switch) {
    assert!(
        matches!(s.field.ty, Ty::Raw) && s.field.bits == 16 && s.narrow_bits == 8,
        "{}::{}: only a raw 8/16-bit switch is supported",
        def.name,
        s.field.name
    );
}

/// Total bits of a static item run.
fn run_bits(items: &[Item]) -> usize {
    items
        .iter()
        .map(|i| match i {
            Item::Field(f) => f.bits as usize,
            Item::Reserved { bits } => *bits as usize,
            _ => panic!("dynamic item in static run"),
        })
        .sum()
}

/// Slots (relative bit offsets) of a static item run.
fn run_slots(items: &[Item]) -> Vec<Slot<'_>> {
    let mut slots = Vec::new();
    let mut bit = 0usize;
    for item in items {
        match item {
            Item::Field(f) => {
                slots.push(Slot { field: f, bit });
                bit += f.bits as usize;
            }
            Item::Reserved { bits } => bit += *bits as usize,
            _ => panic!("dynamic item in static run"),
        }
    }
    slots
}

/// Buffer index expression: absolute byte `k`, or `pos + k`.
fn idx(rel: bool, k: usize) -> String {
    if !rel {
        k.to_string()
    } else if k == 0 {
        "pos".into()
    } else {
        format!("pos + {k}")
    }
}

fn buf_at(rel: bool, k: usize) -> String {
    format!("buffer[{}]", idx(rel, k))
}

/// Raw read expression for a field at `bit`. `locals` selects the
/// `b{k}` byte locals used in the prefix.
fn read_expr(def: &MessageDef, f: &Field, bit: usize, rel: bool, locals: bool) -> String {
    let k = bit / 8;
    match kind(def.name, f, bit) {
        Kind::Word => match f.bits {
            8 => buf_at(rel, k),
            16 => format!(
                "u16::from_be_bytes([{}, {}])",
                buf_at(rel, k),
                buf_at(rel, k + 1)
            ),
            32 => format!(
                "u32::from_be_bytes([{}, {}, {}, {}])",
                buf_at(rel, k),
                buf_at(rel, k + 1),
                buf_at(rel, k + 2),
                buf_at(rel, k + 3)
            ),
            _ => unreachable!(),
        },
        Kind::MaskedWord => {
            let mask = (1u32 << f.bits) - 1;
            format!(
                "u16::from_be_bytes([{}, {}]) & 0x{mask:04X}",
                buf_at(rel, k),
                buf_at(rel, k + 1)
            )
        }
        Kind::SubByte => {
            let be = if locals {
                format!("b{k}")
            } else {
                buf_at(rel, k)
            };
            let o = bit % 8;
            let shift = 8 - o - f.bits as usize;
            let mask = (1u16 << f.bits) - 1;
            if shift == 0 {
                format!("{be} & 0x{mask:02X}")
            } else if o == 0 {
                format!("{be} >> {shift}")
            } else {
                format!("({be} >> {shift}) & 0x{mask:02X}")
            }
        }
    }
}

/// `bool` bit-test expression for a 1-bit field.
fn bool_test(bit: usize, rel: bool, locals: bool) -> String {
    let k = bit / 8;
    let be = if locals {
        format!("b{k}")
    } else {
        buf_at(rel, k)
    };
    let mask = 1u8 << (7 - bit % 8);
    format!("{be} & 0x{mask:02X} != 0")
}

/// `let {bind} = ...;` statement(s) parsing one field. Fallible
/// fields use `let ... else` (the `?` operator is not const fn).
fn parse_binding(
    def: &MessageDef,
    f: &Field,
    bit: usize,
    rel: bool,
    locals: bool,
    indent: &str,
    bind: &str,
) -> String {
    match &f.ty {
        Ty::Bool => format!("{indent}let {bind} = {};\n", bool_test(bit, rel, locals)),
        Ty::Raw => format!(
            "{indent}let {bind} = {};\n",
            read_expr(def, f, bit, rel, locals)
        ),
        Ty::Fallible { ty, ctor, .. } => format!(
            "{indent}let Some({bind}) = {ty}::{ctor}({}) else {{\n\
             {indent}    return Err(ParsingError::ReservedValue);\n\
             {indent}}};\n",
            read_expr(def, f, bit, rel, locals)
        ),
        Ty::Wrap { construct, .. } => format!(
            "{indent}let {bind} = {};\n",
            construct.replace("{}", &read_expr(def, f, bit, rel, locals))
        ),
    }
}

/// Raw on-wire value from a stored-value access expression.
fn raw_expr(ty: &Ty, access: &str) -> String {
    match ty {
        Ty::Bool => unreachable!("bool handled separately"),
        Ty::Raw => access.into(),
        Ty::Fallible { getter, .. } => format!("{access}.{getter}()"),
        Ty::Wrap { deconstruct, .. } => deconstruct.replace("{}", access),
    }
}

/// Count-field width for the named repeat / slice.
fn count_bits(ctrls: &[Ctrl<'_>], of: &str) -> u8 {
    find_count(ctrls, of).1
}

fn find_count<'a>(ctrls: &'a [Ctrl<'_>], of: &str) -> (&'a Ctrl<'a>, u8) {
    ctrls
        .iter()
        .find_map(|c| match c {
            Ctrl::Count { of: o, bits, .. } if *o == of => Some((c, *bits)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no count for {of}"))
}

/// Find the Switch / Variant a Selector points at.
enum SelTarget<'a> {
    Sw(&'a Switch),
    Var(&'a Variant),
}

fn sel_target<'a>(def: &'a MessageDef, of: &str) -> SelTarget<'a> {
    for item in def.items {
        match item {
            Item::Switch(s) if s.field.name == of => return SelTarget::Sw(s),
            Item::Variant(v) if v.name == of => return SelTarget::Var(v),
            _ => {}
        }
    }
    panic!("{}: selector target {of} not found", def.name)
}

/// Replace `{name}` ctx placeholders with `name` or `self.name`.
fn cond_expr(def: &MessageDef, expr: &str, selfd: bool) -> String {
    let mut s = expr.to_string();
    for c in def.ctx {
        let access = if selfd {
            format!("self.{}", c.name)
        } else {
            c.name.to_string()
        };
        s = s.replace(&format!("{{{}}}", c.name), &access);
    }
    s
}

/// Serialize statements for a static run of bytes. `access` renders
/// the stored-value access for a field (e.g. "self.cause" or "v").
fn run_serialize(
    def: &MessageDef,
    slots: &[Slot<'_>],
    ctrls: &[Ctrl<'_>],
    bytes: usize,
    rel: bool,
    access: &dyn Fn(&Field) -> String,
    indent: &str,
) -> Vec<String> {
    let mut stmts = Vec::new();
    let mut k = 0usize;
    while k < bytes {
        // A byte-aligned multi-byte field starting here?
        if let Some(s) = slots
            .iter()
            .find(|s| s.bit == k * 8 && s.field.bits >= 8 && s.field.bits % 8 == 0)
        {
            let n = s.field.bits as usize / 8;
            let raw = raw_expr(&s.field.ty, &access(s.field));
            if n == 1 {
                stmts.push(format!("{indent}out[{}] = {raw};\n", idx(rel, k)));
            } else {
                // Per-byte writes: copy_from_slice is not const fn.
                stmts.push(format!("{indent}let raw = {raw}.to_be_bytes();\n"));
                for j in 0..n {
                    stmts.push(format!("{indent}out[{}] = raw[{j}];\n", idx(rel, k + j)));
                }
            }
            k += n;
            continue;
        }
        // A masked word (e.g. 13-bit channel) starting in this byte?
        if let Some(s) = slots
            .iter()
            .find(|s| s.bit / 8 == k && matches!(kind(def.name, s.field, s.bit), Kind::MaskedWord))
        {
            assert!(
                slots
                    .iter()
                    .all(|o| std::ptr::eq(o.field, s.field) || o.bit / 8 < k || o.bit / 8 > k + 1),
                "{}::{}: masked word shares bytes with another field",
                def.name,
                s.field.name
            );
            let mask = (1u32 << s.field.bits) - 1;
            let raw = raw_expr(&s.field.ty, &access(s.field));
            stmts.push(format!(
                "{indent}let raw = ({raw} & 0x{mask:04X}).to_be_bytes();\n"
            ));
            stmts.push(format!("{indent}out[{}] = raw[0];\n", idx(rel, k)));
            stmts.push(format!("{indent}out[{}] = raw[1];\n", idx(rel, k + 1)));
            k += 2;
            continue;
        }
        // Compose the byte from its sub-byte fields and control bits
        // (reserved = 0). Cores are unparenthesized; parentheses are
        // added only when several parts are OR-combined.
        let mut cores = Vec::new();
        for s in slots.iter().filter(|s| s.bit / 8 == k) {
            let f = s.field;
            let o = s.bit % 8;
            let shift = 8 - o - f.bits as usize;
            let mask = (1u16 << f.bits) - 1;
            let core = match &f.ty {
                Ty::Bool => {
                    let bitmask = 1u8 << (7 - o);
                    format!("if {} {{ 0x{bitmask:02X} }} else {{ 0 }}", access(f))
                }
                _ => {
                    let raw = raw_expr(&f.ty, &access(f));
                    if shift == 0 {
                        format!("{raw} & 0x{mask:02X}")
                    } else {
                        format!("({raw} & 0x{mask:02X}) << {shift}")
                    }
                }
            };
            cores.push(core);
        }
        for c in ctrls.iter().filter(|c| ctrl_bit(c) / 8 == k) {
            cores.push(ctrl_core(def, c));
        }
        match cores.len() {
            0 => stmts.push(format!("{indent}out[{}] = 0;\n", idx(rel, k))),
            1 => stmts.push(format!("{indent}out[{}] = {};\n", idx(rel, k), cores[0])),
            _ => {
                let joined = cores
                    .iter()
                    .map(|c| format!("({c})"))
                    .collect::<Vec<_>>()
                    .join(" | ");
                stmts.push(format!("{indent}out[{}] = {joined};\n", idx(rel, k)));
            }
        }
        k += 1;
    }
    stmts
}

fn ctrl_bit(c: &Ctrl<'_>) -> usize {
    match c {
        Ctrl::Flag { bit, .. }
        | Ctrl::Count { bit, .. }
        | Ctrl::Sel { bit, .. }
        | Ctrl::Mode { bit, .. }
        | Ctrl::Const { bit, .. } => *bit,
    }
}

/// Serialize core expression for one control.
fn ctrl_core(def: &MessageDef, c: &Ctrl<'_>) -> String {
    match c {
        Ctrl::Flag { of, bit } => {
            let mask = 1u8 << (7 - bit % 8);
            match of.split_once('.') {
                None => format!("if self.{of}.is_some() {{ 0x{mask:02X} }} else {{ 0 }}"),
                Some((parent, child)) => format!(
                    "if match self.{parent} {{ Some(v) => v.{child}.is_some(), None => false }}                      {{ 0x{mask:02X} }} else {{ 0 }}"
                ),
            }
        }
        Ctrl::Count { of, bit, bits } => {
            let o = bit % 8;
            let shift = 8 - o - *bits as usize;
            let mask = (1u16 << bits) - 1;
            let val = match count_target(def, of) {
                CountTarget::Rep(Repeat {
                    all_escape: Some(AllEscape { all, specific, .. }),
                    ..
                }) => format!(
                    "match self.{of} {{ {all} => {mask}, {specific}(ref f) => f.len() as u8 }}"
                ),
                CountTarget::Rep(r) if r.bias > 0 => {
                    format!("(self.{of}.len() as u8 - {})", r.bias)
                }
                CountTarget::Rep(_) => format!("self.{of}.len() as u8"),
                CountTarget::Slice(cs) => match &cs.all_ones {
                    AllOnes::Reserved => format!("self.{of}.len() as u8"),
                    AllOnes::All { all, specific, .. } => format!(
                        "match self.{of} {{ {all} => {mask}, {specific}(f) => f.len() as u8 }}"
                    ),
                },
            };
            if shift == 0 {
                format!("{val} & 0x{mask:02X}")
            } else {
                format!("({val} & 0x{mask:02X}) << {shift}")
            }
        }
        Ctrl::Const { bits, value, bit } => {
            let o = bit % 8;
            let shift = 8 - o - *bits as usize;
            format!("0x{:02X}", (u16::from(*value) << shift) as u8)
        }
        Ctrl::Mode { tag, bit } => {
            let o = bit % 8;
            let shift = 8 - o - tag.bits as usize;
            let mask = (1u16 << tag.bits) - 1;
            let val = format!(
                "match self.{} {{ Some(v) => v.{}, None => 0 }}",
                tag.of, tag.accessor
            );
            if shift == 0 {
                format!("({val}) & 0x{mask:02X}")
            } else {
                format!("(({val}) & 0x{mask:02X}) << {shift}")
            }
        }
        Ctrl::Sel { of, bit } => {
            let mask = 1u8 << (7 - bit % 8);
            match sel_target(def, of) {
                SelTarget::Sw(s) => {
                    let nmax = (1u32 << s.narrow_bits) - 1;
                    format!("if self.{of} > {nmax} {{ 0x{mask:02X} }} else {{ 0 }}")
                }
                SelTarget::Var(v) => format!(
                    "if matches!(self.{of}, {}(_)) {{ 0x{mask:02X} }} else {{ 0 }}",
                    v.wide.path
                ),
            }
        }
    }
}

/// What an Item::Count's bits describe.
enum CountTarget<'a> {
    Rep(&'a Repeat),
    Slice(&'a CountSlice),
}

fn count_target<'a>(def: &'a MessageDef, name: &str) -> CountTarget<'a> {
    def.items
        .iter()
        .find_map(|i| match i {
            Item::Repeat(r) if r.name == name => Some(CountTarget::Rep(r)),
            Item::CountSlice(cs) if cs.name == name => Some(CountTarget::Slice(cs)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{}: count target {name} not found", def.name))
}

/// Parse `let` statement for one field, bound to its own name.
fn parse_stmt(def: &MessageDef, s: &Slot<'_>, rel: bool, locals: bool, indent: &str) -> String {
    parse_binding(def, s.field, s.bit, rel, locals, indent, s.field.name)
}

/// Bounds check against `pos + n`.
fn check_rel(n_bytes: usize, indent: &str) -> String {
    format!(
        "{indent}if buffer.len() < pos + {n_bytes} {{\n\
         {indent}    return Err(ParsingError::Truncated);\n\
         {indent}}}\n"
    )
}

/// Struct fields in declaration order.
enum SField<'a> {
    Ctx(&'a crate::ir::Ctx),
    Plain(&'a Field),
    Opt(&'a Group),
    Rep(&'a Repeat),
    Sw(&'a Switch),
    Var(&'a Variant),
    Tail(&'a TailSlice),
    CSlice(&'a CountSlice),
    FGroup(&'a FieldGroup),
    Inline(&'a Group),
}

/// The field group claiming `leaf`, if any, plus whether `leaf` is
/// its first (wire-order) member.
fn claiming_group<'a>(def: &'a MessageDef, leaf: &str) -> Option<(&'a FieldGroup, bool)> {
    def.field_groups
        .iter()
        .find(|g| g.leaves.contains(&leaf))
        .map(|g| (g, g.leaves[0] == leaf))
}

fn struct_fields<'a>(def: &'a MessageDef) -> Vec<SField<'a>> {
    let mut out: Vec<SField<'a>> = def.ctx.iter().map(SField::Ctx).collect();
    for item in def.items {
        match item {
            Item::Field(f) => match claiming_group(def, f.name) {
                Some((g, true)) => out.push(SField::FGroup(g)),
                Some((_, false)) => {}
                None => out.push(SField::Plain(f)),
            },
            Item::Optional(g) => out.push(SField::Opt(g)),
            Item::Repeat(r) => out.push(SField::Rep(r)),
            Item::Switch(s) => out.push(SField::Sw(s)),
            Item::Variant(v) => out.push(SField::Var(v)),
            Item::TailSlice(ts) => out.push(SField::Tail(ts)),
            Item::CountSlice(cs) => out.push(SField::CSlice(cs)),
            Item::Inline(g) => out.push(SField::Inline(g)),
            _ => {}
        }
    }
    out
}

fn opt_ty(g: &Group) -> String {
    if let [Item::Switch(s)] = g.items {
        return s.field.rust_ty();
    }
    if let Some(c) = &g.composite {
        return c.ty.into();
    }
    g.items
        .iter()
        .find_map(|i| match i {
            Item::Field(f) => Some(f.rust_ty()),
            _ => None,
        })
        .expect("group has one field")
}

fn elem_field(r: &Repeat) -> &Field {
    r.items
        .iter()
        .find_map(|i| match i {
            Item::Field(f) => Some(f),
            _ => None,
        })
        .expect("repeat has one field")
}

fn elem_ty(r: &Repeat) -> String {
    match &r.composite {
        Some(c) => c.ty.into(),
        None => elem_field(r).rust_ty(),
    }
}

fn sfield_decl(sf: &SField<'_>) -> (String, String, String) {
    match sf {
        SField::Ctx(c) => (c.name.into(), c.ty.into(), c.doc.into()),
        SField::Plain(f) => (f.name.into(), f.rust_ty(), f.doc.into()),
        SField::Opt(g) => (
            g.name.into(),
            format!("Option<{}>", opt_ty(g)),
            g.doc.into(),
        ),
        SField::Rep(r) => {
            let ty = match &r.all_escape {
                Some(e) => e.ty.to_string(),
                None => format!("Vec<{}, {}>", elem_ty(r), r.max_const),
            };
            (r.name.into(), ty, r.doc.into())
        }
        SField::Sw(s) => (s.field.name.into(), s.field.rust_ty(), s.field.doc.into()),
        SField::Var(v) => (v.name.into(), v.ty.into(), v.doc.into()),
        SField::Tail(ts) => (ts.name.into(), format!("&'a [{}]", ts.ty), ts.doc.into()),
        SField::CSlice(cs) => {
            let ty = match &cs.all_ones {
                AllOnes::Reserved => format!("&'a [{}]", cs.ty),
                AllOnes::All { ty, .. } => format!("{ty}<'a>"),
            };
            (cs.name.into(), ty, cs.doc.into())
        }
        SField::FGroup(g) => (g.name.into(), g.ty.into(), g.doc.into()),
        SField::Inline(g) => (
            g.name.into(),
            g.composite.as_ref().expect("inline composite").ty.into(),
            g.doc.into(),
        ),
    }
}

/// Does any field anywhere use a fallible constructor?
fn any_fallible(def: &MessageDef) -> bool {
    fn field_fallible(f: &Field) -> bool {
        matches!(f.ty, Ty::Fallible { .. })
    }
    fn items_fallible(items: &[Item]) -> bool {
        items.iter().any(|i| match i {
            Item::Field(f) => field_fallible(f),
            Item::Optional(g) => items_fallible(g.items),
            Item::Repeat(r) => items_fallible(r.items),
            Item::Switch(s) => field_fallible(&s.field),
            Item::Variant(v) => {
                matches!(v.narrow.ty, Ty::Fallible { .. })
                    || matches!(v.wide.ty, Ty::Fallible { .. })
            }
            Item::Inline(g) => items_fallible(g.items),
            Item::ModeTag(_) => true,
            _ => false,
        })
    }
    items_fallible(def.items)
}

/// Wide-form read for a switch (masked when narrower than 16 bits).
fn wide_read(sw: &Switch) -> String {
    if sw.wide_bits == 16 {
        rel_word_read(16)
    } else {
        let mask = (1u32 << sw.wide_bits) - 1;
        format!("u16::from_be_bytes([buffer[pos], buffer[pos + 1]]) & 0x{mask:04X}")
    }
}

/// Serialize-side overflow guard for a wide form narrower than 16 bits.
fn wide_guard(sw: &Switch, acc: &str, indent: &str) -> String {
    if sw.wide_bits == 16 {
        String::new()
    } else {
        let max = (1u32 << sw.wide_bits) - 1;
        format!(
            "{indent}if {acc} > 0x{max:X} {{\n\
             {indent}    return Err(ExcessiveBitsSet);\n\
             {indent}}}\n"
        )
    }
}

/// Word read of `n` bytes at `pos`, plus the stored-value widening.
fn rel_word_read(bits: u8) -> String {
    match bits {
        8 => "buffer[pos]".into(),
        16 => "u16::from_be_bytes([buffer[pos], buffer[pos + 1]])".into(),
        32 => {
            "u32::from_be_bytes([buffer[pos], buffer[pos + 1], buffer[pos + 2], buffer[pos + 3]])"
                .into()
        }
        _ => panic!("unsupported word width {bits}"),
    }
}

/// Emit one generated module.
pub fn emit_message(def: &MessageDef) -> String {
    if let [Item::VariantBody(vb)] = def.items {
        return emit_enum_message(def, vb);
    }
    let lay = layout(def);
    let (fig, legend) = figure::render(def);
    let has_repeat = lay.regions.iter().any(|r| matches!(r, Region::Rep(_)));
    let fields = struct_fields(def);

    let mut out = String::new();
    out.push_str(&format!(
        "// GENERATED by codegen/src/defs/{}.rs - DO NOT EDIT.\n\
         // Regenerate with `make codegen`.\n\n",
        def.module
    ));
    out.push_str(&format!("//! {} generated codec.\n//!\n", def.name));
    out.push_str(&format!("//! {}.\n//!\n", def.spec));
    out.push_str("//! Wire layout (ETSI bit numbering, column 0 = MSB):\n//!\n//! ```text\n");
    for line in fig.lines() {
        if line.is_empty() {
            out.push_str("//!\n");
        } else {
            out.push_str(&format!("//! {line}\n"));
        }
    }
    out.push_str("//! ```\n");
    if !legend.is_empty() {
        out.push_str("//!\n");
        for l in &legend {
            out.push_str(&format!("//! * {l}\n"));
        }
    }
    out.push('\n');

    if def.ie_type.is_some() {
        out.push_str("use crate::mac::pdu::MessageBody;\n");
    }
    if def.short_ie.is_some() {
        out.push_str("use crate::mac::pdu::ShortMessageBody;\n");
    }
    for imp in def.imports {
        out.push_str(imp);
        out.push('\n');
    }
    out.push_str("use crate::types::*;\nuse crate::{ExcessiveBitsSet, ParsingError};\n");
    if has_repeat {
        out.push_str("use heapless::Vec;\n");
    }
    out.push('\n');

    for region in &lay.regions {
        if let Region::Rep(r) = region {
            let (_, bits) = find_count(&lay.ctrls, r.name);
            let max = (1u32 << bits) - 1 + u32::from(r.bias)
                - u32::from(r.reserved_max || r.all_escape.is_some());
            out.push_str(&format!(
                "/// {}\npub const {}: usize = {max};\n\n",
                r.max_doc, r.max_const
            ));
        }
    }

    out.push_str(&format!("/// {}\n", def.doc));
    if has_repeat {
        out.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
    } else {
        out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    }
    out.push_str("#[cfg_attr(feature = \"defmt\", derive(defmt::Format))]\n");
    let lt = if lay
        .regions
        .iter()
        .any(|r| matches!(r, Region::Tail(_) | Region::CSlice(_)))
    {
        "<'a>"
    } else {
        ""
    };
    out.push_str(&format!("pub struct {}{lt} {{\n", def.name));
    for sf in &fields {
        let (name, ty, doc) = sfield_decl(sf);
        out.push_str(&format!("    /// {doc}\n"));
        out.push_str(&format!("    pub {name}: {ty},\n"));
    }
    out.push_str("}\n\n");

    let impl_lt = if lt.is_empty() { "" } else { "<'_>" };
    out.push_str(&format!("impl {}{impl_lt} {{\n", def.name));
    out.push_str(&encoded_len_fn(def, &lay, has_repeat));
    out.push('\n');
    out.push_str(&serialize_fn(def, &lay));
    out.push('\n');
    out.push_str(&parse_fn(def, &lay));
    out.push_str("}\n");

    if let Some(v) = def.ie_type {
        let (tr_intro, tr_args) = if lt.is_empty() {
            ("", "")
        } else {
            ("<'a>", "<'a>")
        };
        out.push_str(&format!(
            "\nimpl{tr_intro} MessageBody for {}{tr_args} {{\n\
             \x20   const IE_TYPE: IEType6bit = IEType6bit::{v};\n\
             \x20   #[inline]\n\
             \x20   fn encoded_len(&self) -> usize {{\n        Self::encoded_len(self)\n    }}\n\
             \x20   #[inline]\n\
             \x20   fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {{\n\
             \x20       Self::serialize(self, out)\n    }}\n}}\n",
            def.name
        ));
    }
    if let Some(expr) = def.short_ie {
        out.push_str(&format!(
            "\nimpl ShortMessageBody for {} {{\n\
             \x20   const IE_TYPE: ShortIeType = {expr};\n\
             \x20   #[inline]\n\
             \x20   fn encoded_len(&self) -> usize {{\n        Self::encoded_len(self)\n    }}\n\
             \x20   #[inline]\n\
             \x20   fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {{\n\
             \x20       Self::serialize(self, out)\n    }}\n}}\n",
            def.name
        ));
    }
    out
}

fn encoded_len_fn(def: &MessageDef, lay: &Layout<'_>, has_repeat: bool) -> String {
    let base: usize = lay.prefix_bytes
        + lay
            .regions
            .iter()
            .map(|r| match r {
                Region::Fixed { bytes, .. } => *bytes,
                _ => 0,
            })
            .sum::<usize>();
    let mut out = String::new();
    if lay.regions.is_empty() {
        out.push_str(&format!(
            "    /// Body length in bytes (always {base}).\n\
             \x20   #[must_use]\n\
             \x20   #[inline]\n\
             \x20   pub const fn encoded_len(&self) -> usize {{\n\
             \x20       {base}\n\
             \x20   }}\n"
        ));
        return out;
    }
    out.push_str("    /// Number of bytes [`Self::serialize`] will write.\n");
    out.push_str("    #[must_use]\n    #[inline]\n");
    let constness = if has_repeat { "" } else { "const " };
    out.push_str(&format!(
        "    pub {constness}fn encoded_len(&self) -> usize {{\n"
    ));
    out.push_str(&format!("        let mut len = {base};\n"));
    out.push_str(&len_regions(def, lay));
    out.push_str("        len\n    }\n");
    out
}

/// `len += ...` statements for every dynamic region.
fn len_regions(def: &MessageDef, lay: &Layout<'_>) -> String {
    let mut out = String::new();
    for region in &lay.regions {
        match region {
            Region::Fixed { .. } => {}
            Region::Opt(g) => {
                if let [Item::Switch(s)] = g.items {
                    let Wide::Ctx(expr) = &s.wide else {
                        unreachable!()
                    };
                    out.push_str(&format!(
                        "        if self.{}.is_some() {{\n\
                         \x20           len += if {} {{ 2 }} else {{ 1 }};\n\
                         \x20       }}\n",
                        g.name,
                        cond_expr(def, expr, true)
                    ));
                } else {
                    let parts = group_parts(def, g);
                    if parts.nested.is_empty() {
                        out.push_str(&format!(
                            "        if self.{}.is_some() {{\n            len += {};\n        }}\n",
                            g.name,
                            run_bits(parts.statics) / 8
                        ));
                    } else {
                        out.push_str(&format!(
                            "        if let Some(v) = self.{} {{\n            len += {};\n",
                            g.name,
                            run_bits(parts.statics) / 8
                        ));
                        for n in &parts.nested {
                            out.push_str(&format!(
                                "            if v.{}.is_some() {{\n\
                                 \x20               len += {};\n\
                                 \x20           }}\n",
                                n.name,
                                run_bits(n.items) / 8
                            ));
                        }
                        out.push_str("        }\n");
                    }
                }
            }
            Region::Rep(r) => {
                let eb = run_bits(r.items) / 8;
                if let Some(AllEscape { all, specific, .. }) = &r.all_escape {
                    let count = if eb == 1 {
                        "f.len()".to_string()
                    } else {
                        format!("f.len() * {eb}")
                    };
                    out.push_str(&format!(
                        "        len += match self.{} {{\n\
                         \x20           {all} => 0,\n\
                         \x20           {specific}(ref f) => {count},\n\
                         \x20       }};\n",
                        r.name
                    ));
                } else if eb == 1 {
                    out.push_str(&format!("        len += self.{}.len();\n", r.name));
                } else {
                    out.push_str(&format!("        len += self.{}.len() * {eb};\n", r.name));
                }
            }
            Region::Tail(ts) => {
                out.push_str(&format!("        len += self.{}.len();\n", ts.name));
            }
            Region::CSlice(cs) => match &cs.all_ones {
                AllOnes::Reserved => {
                    out.push_str(&format!("        len += self.{}.len();\n", cs.name));
                }
                AllOnes::All { all, specific, .. } => {
                    out.push_str(&format!(
                        "        len += match self.{} {{\n\
                         \x20           {all} => 0,\n\
                         \x20           {specific}(f) => f.len(),\n\
                         \x20       }};\n",
                        cs.name
                    ));
                }
            },
            Region::Inline(g) => {
                let parts = inline_parts(def, g);
                let sb = run_bits(parts.statics) / 8;
                match parts.switch {
                    None => out.push_str(&format!("        len += {sb};\n")),
                    Some(sw) => {
                        let Wide::Ctx(expr) = &sw.wide else {
                            unreachable!()
                        };
                        out.push_str(&format!(
                            "        len += {sb} + if {} {{ 2 }} else {{ 1 }};\n",
                            cond_expr(def, expr, true)
                        ));
                    }
                }
            }
            Region::Sw(s) => match &s.wide {
                Wide::ValueOverflow => {
                    let nmax = (1u32 << s.narrow_bits) - 1;
                    out.push_str(&format!(
                        "        len += if self.{} > {nmax} {{ 2 }} else {{ 1 }};\n",
                        s.field.name
                    ));
                }
                Wide::Ctx(expr) => out.push_str(&format!(
                    "        len += if {} {{ 2 }} else {{ 1 }};\n",
                    cond_expr(def, expr, true)
                )),
            },
            Region::Var(v) => {
                out.push_str(&format!(
                    "        len += match self.{} {{\n\
                     \x20           {}(_) => {},\n\
                     \x20           {}(_) => {},\n\
                     \x20       }};\n",
                    v.name,
                    v.narrow.path,
                    v.narrow.bits / 8,
                    v.wide.path,
                    v.wide.bits / 8
                ));
            }
        }
    }
    out
}

fn serialize_fn(def: &MessageDef, lay: &Layout<'_>) -> String {
    let mut out = String::new();
    let n_bytes = lay.prefix_bytes;
    let self_access = |f: &Field| {
        if let Some((g, _)) = claiming_group(def, f.name) {
            let i = g.leaves.iter().position(|l| *l == f.name).unwrap();
            return g.accessors[i].to_string();
        }
        format!("self.{}", f.name)
    };
    // heapless::Vec iteration / push is not const fn; everything else
    // the emitter produces is, so repeat-free codecs work at compile
    // time.
    let constness = if lay.regions.iter().any(|r| matches!(r, Region::Rep(_))) {
        ""
    } else {
        "const "
    };

    if lay.regions.is_empty() {
        let plural = if n_bytes == 1 { "" } else { "s" };
        out.push_str(&format!(
            "    /// Serialize the body. Writes exactly {n_bytes} byte{plural}.\n\
             \x20   ///\n\
             \x20   /// # Errors\n\
             \x20   ///\n\
             \x20   /// Returns [`ExcessiveBitsSet`] if `out` is shorter than\n\
             \x20   /// {n_bytes} byte{plural}.\n"
        ));
        out.push_str(
            &format!(
        "    pub {constness}fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {{\n"
    ),
        );
        if n_bytes == 1 {
            out.push_str(
                "        if out.is_empty() {\n            return Err(ExcessiveBitsSet);\n        }\n",
            );
        } else {
            out.push_str(&format!(
                "        if out.len() < {n_bytes} {{\n            return Err(ExcessiveBitsSet);\n        }}\n"
            ));
        }
        for stmt in run_serialize(
            def,
            &lay.prefix,
            &lay.ctrls,
            n_bytes,
            false,
            &self_access,
            "        ",
        ) {
            out.push_str(&stmt);
        }
        out.push_str(&format!("        Ok({n_bytes})\n    }}\n"));
        return out;
    }

    let mut clauses = vec!["if `out` is shorter than [`Self::encoded_len`]".to_string()];
    for region in &lay.regions {
        match region {
            Region::Rep(r) if r.bias > 0 => clauses.push(format!(
                "for an empty `{}` list (the on-wire count is `len - {}`)",
                r.name, r.bias
            )),
            Region::Opt(g) => {
                if let [Item::Switch(s)] = g.items
                    && matches!(s.wide, Wide::Ctx(_))
                {
                    clauses.push(format!(
                        "if `{}` does not fit the selected narrow on-wire form",
                        g.name
                    ));
                }
            }
            Region::CSlice(cs) => {
                let max = (1u32 << count_bits(&lay.ctrls, cs.name)) - 2;
                match &cs.all_ones {
                    AllOnes::Reserved => {
                        clauses.push(format!("if `{}` holds more than {max} entries", cs.name))
                    }
                    AllOnes::All { specific, .. } => clauses.push(format!(
                        "if a `{specific}` value of `{}` holds more than {max} entries",
                        cs.name
                    )),
                }
            }
            _ => {}
        }
    }
    out.push_str(
        "    /// Serialize the body into `out`. Returns the number of bytes written.\n\
         \x20   ///\n\
         \x20   /// # Errors\n\
         \x20   ///\n",
    );
    out.push_str(&format!(
        "    /// Returns [`ExcessiveBitsSet`] {}.\n",
        clauses.join(", or ")
    ));
    out.push_str(
        &format!(
        "    pub {constness}fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {{\n"
    ),
    );
    out.push_str(&ser_dynamic_body(def, lay));
    out.push_str("    }\n");
    out
}

/// Guards, length check, prefix writes and dynamic regions of a
/// serialize body, ending in `Ok(pos)`.
fn ser_dynamic_body(def: &MessageDef, lay: &Layout<'_>) -> String {
    let mut out = String::new();
    let n_bytes = lay.prefix_bytes;
    let self_access = |f: &Field| {
        if let Some((g, _)) = claiming_group(def, f.name) {
            let i = g.leaves.iter().position(|l| *l == f.name).unwrap();
            return g.accessors[i].to_string();
        }
        format!("self.{}", f.name)
    };
    for region in &lay.regions {
        if let Region::Rep(r) = region
            && r.bias > 0
        {
            let guard = if r.bias == 1 {
                format!("self.{}.is_empty()", r.name)
            } else {
                format!("self.{}.len() < {}", r.name, r.bias)
            };
            out.push_str(&format!(
                "        if {guard} {{\n\
                 \x20           return Err(ExcessiveBitsSet);\n\
                 \x20       }}\n"
            ));
        }
        if let Region::CSlice(cs) = region {
            let max = (1u32 << count_bits(&lay.ctrls, cs.name)) - 2;
            match &cs.all_ones {
                AllOnes::Reserved => out.push_str(&format!(
                    "        if self.{}.len() > {max} {{\n\
                     \x20           return Err(ExcessiveBitsSet);\n\
                     \x20       }}\n",
                    cs.name
                )),
                AllOnes::All { specific, .. } => out.push_str(&format!(
                    "        if let {specific}(f) = self.{}\n\
                     \x20           && f.len() > {max}\n\
                     \x20       {{\n\
                     \x20           return Err(ExcessiveBitsSet);\n\
                     \x20       }}\n",
                    cs.name
                )),
            }
        }
    }
    out.push_str(
        "        let len = self.encoded_len();\n\
         \x20       if out.len() < len {\n            return Err(ExcessiveBitsSet);\n        }\n",
    );
    for stmt in run_serialize(
        def,
        &lay.prefix,
        &lay.ctrls,
        n_bytes,
        false,
        &self_access,
        "        ",
    ) {
        out.push_str(&stmt);
    }
    if lay.regions.is_empty() {
        out.push_str(&format!("        Ok({n_bytes})\n"));
        return out;
    }
    out.push_str(&format!("        let mut pos = {n_bytes};\n"));

    for region in &lay.regions {
        match region {
            Region::Fixed { slots, bytes } => {
                for stmt in run_serialize(def, slots, &[], *bytes, true, &self_access, "        ") {
                    out.push_str(&stmt);
                }
                out.push_str(&format!("        pos += {bytes};\n"));
            }
            Region::Opt(g) => {
                out.push_str(&format!("        if let Some(v) = self.{} {{\n", g.name));
                if let [Item::Switch(s)] = g.items {
                    let Wide::Ctx(expr) = &s.wide else {
                        unreachable!()
                    };
                    out.push_str(&format!(
                        "            if {} {{\n",
                        cond_expr(def, expr, true)
                    ));
                    out.push_str(&wide_guard(s, "v", "                "));
                    out.push_str(
                        "                let raw = v.to_be_bytes();\n\
                         \x20               out[pos] = raw[0];\n\
                         \x20               out[pos + 1] = raw[1];\n\
                         \x20               pos += 2;\n\
                         \x20           } else {\n\
                         \x20               if v > 0xFF {\n\
                         \x20                   return Err(ExcessiveBitsSet);\n\
                         \x20               }\n\
                         \x20               out[pos] = v as u8;\n\
                         \x20               pos += 1;\n\
                         \x20           }\n",
                    );
                } else {
                    let parts = group_parts(def, g);
                    let slots = run_slots(parts.statics);
                    let bytes = run_bits(parts.statics) / 8;
                    let access: Box<dyn Fn(&Field) -> String> = match &g.composite {
                        None => Box::new(|_f: &Field| "v".to_string()),
                        Some(c) => {
                            let mut map = std::collections::HashMap::new();
                            for (i, s) in slots.iter().enumerate() {
                                map.insert(s.field.name, c.accessors[i]);
                            }
                            Box::new(move |f: &Field| map[f.name].to_string())
                        }
                    };
                    for stmt in
                        run_serialize(def, &slots, &[], bytes, true, &access, "            ")
                    {
                        out.push_str(&stmt);
                    }
                    out.push_str(&format!("            pos += {bytes};\n"));
                    for n in &parts.nested {
                        let nslots = run_slots(n.items);
                        let nbytes = run_bits(n.items) / 8;
                        out.push_str(&format!("            if let Some(v) = v.{} {{\n", n.name));
                        let naccess = |_f: &Field| "v".to_string();
                        for stmt in run_serialize(
                            def,
                            &nslots,
                            &[],
                            nbytes,
                            true,
                            &naccess,
                            "                ",
                        ) {
                            out.push_str(&stmt);
                        }
                        out.push_str(&format!(
                            "                pos += {nbytes};\n            }}\n"
                        ));
                    }
                }
                out.push_str("        }\n");
            }
            Region::Rep(r) => {
                let slots = run_slots(r.items);
                let bytes = run_bits(r.items) / 8;
                let escaped = r.all_escape.is_some();
                if let Some(AllEscape { specific, .. }) = &r.all_escape {
                    out.push_str(&format!(
                        "        if let {specific}(ref f) = self.{} {{\n        for v in f {{\n",
                        r.name
                    ));
                } else {
                    out.push_str(&format!("        for v in &self.{} {{\n", r.name));
                }
                let access: Box<dyn Fn(&Field) -> String> = match &r.composite {
                    None => Box::new(|_f: &Field| "v".to_string()),
                    Some(c) => {
                        let mut map = std::collections::HashMap::new();
                        for (i, s) in slots.iter().enumerate() {
                            map.insert(s.field.name, c.accessors[i]);
                        }
                        Box::new(move |f: &Field| map[f.name].to_string())
                    }
                };
                for stmt in run_serialize(def, &slots, &[], bytes, true, &access, "            ") {
                    out.push_str(&stmt);
                }
                out.push_str(&format!("            pos += {bytes};\n        }}\n"));
                if escaped {
                    out.push_str("        }\n");
                }
            }
            Region::Sw(s) => {
                let name = s.field.name;
                match &s.wide {
                    Wide::ValueOverflow => {
                        let nmax = (1u32 << s.narrow_bits) - 1;
                        out.push_str(&format!(
                            "        if self.{name} > {nmax} {{\n\
                             \x20           let raw = self.{name}.to_be_bytes();\n\
                             \x20           out[pos] = raw[0];\n\
                             \x20           out[pos + 1] = raw[1];\n\
                             \x20           pos += 2;\n\
                             \x20       }} else {{\n\
                             \x20           out[pos] = self.{name} as u8;\n\
                             \x20           pos += 1;\n\
                             \x20       }}\n"
                        ));
                    }
                    Wide::Ctx(expr) => {
                        out.push_str(&format!(
                            "        if {} {{\n\
                             \x20           let raw = self.{name}.to_be_bytes();\n\
                             \x20           out[pos] = raw[0];\n\
                             \x20           out[pos + 1] = raw[1];\n\
                             \x20           pos += 2;\n\
                             \x20       }} else {{\n\
                             \x20           if self.{name} > 0xFF {{\n\
                             \x20               return Err(ExcessiveBitsSet);\n\
                             \x20           }}\n\
                             \x20           out[pos] = self.{name} as u8;\n\
                             \x20           pos += 1;\n\
                             \x20       }}\n",
                            cond_expr(def, expr, true)
                        ));
                    }
                }
            }
            Region::Tail(ts) => {
                out.push_str(&format!(
                    "        let mut i = 0;\n\
                     \x20       while i < self.{0}.len() {{\n\
                     \x20           out[pos] = self.{0}[i].{1}();\n\
                     \x20           pos += 1;\n\
                     \x20           i += 1;\n\
                     \x20       }}\n",
                    ts.name, ts.getter
                ));
            }
            Region::CSlice(cs) => match &cs.all_ones {
                AllOnes::Reserved => out.push_str(&format!(
                    "        let mut i = 0;\n\
                         \x20       while i < self.{0}.len() {{\n\
                         \x20           out[pos] = self.{0}[i].{1}();\n\
                         \x20           pos += 1;\n\
                         \x20           i += 1;\n\
                         \x20       }}\n",
                    cs.name, cs.getter
                )),
                AllOnes::All { specific, .. } => out.push_str(&format!(
                    "        if let {specific}(f) = self.{} {{\n\
                         \x20           let mut i = 0;\n\
                         \x20           while i < f.len() {{\n\
                         \x20               out[pos] = f[i].{}();\n\
                         \x20               pos += 1;\n\
                         \x20               i += 1;\n\
                         \x20           }}\n\
                         \x20       }}\n",
                    cs.name, cs.getter
                )),
            },
            Region::Inline(g) => {
                let parts = inline_parts(def, g);
                let c = g.composite.as_ref().expect("inline composite");
                let mut acc_idx = 0;
                if let Some(sw) = parts.switch {
                    let acc = c.accessors[0];
                    acc_idx = 1;
                    let Wide::Ctx(expr) = &sw.wide else {
                        unreachable!()
                    };
                    let nmax = (1u32 << sw.narrow_bits) - 1;
                    out.push_str(&format!("        if {} {{\n", cond_expr(def, expr, true)));
                    out.push_str(&wide_guard(sw, acc, "            "));
                    out.push_str(&format!(
                        "            let raw = {acc}.to_be_bytes();\n\
                         \x20           out[pos] = raw[0];\n\
                         \x20           out[pos + 1] = raw[1];\n\
                         \x20           pos += 2;\n\
                         \x20       }} else {{\n\
                         \x20           if {acc} > 0x{nmax:X} {{\n\
                         \x20               return Err(ExcessiveBitsSet);\n\
                         \x20           }}\n\
                         \x20           out[pos] = {acc} as u8;\n\
                         \x20           pos += 1;\n\
                         \x20       }}\n"
                    ));
                }
                let slots = run_slots(parts.statics);
                let bytes = run_bits(parts.statics) / 8;
                let mut map = std::collections::HashMap::new();
                for (i, sl) in slots.iter().enumerate() {
                    map.insert(sl.field.name, c.accessors[acc_idx + i]);
                }
                let access = move |f: &Field| map[f.name].to_string();
                for stmt in run_serialize(def, &slots, &[], bytes, true, &access, "        ") {
                    out.push_str(&stmt);
                }
                out.push_str(&format!("        pos += {bytes};\n"));
            }
            Region::Var(v) => {
                out.push_str(&format!("        match self.{} {{\n", v.name));
                for arm in [&v.narrow, &v.wide] {
                    let n = arm.bits as usize / 8;
                    let raw = raw_expr(&arm.ty, "v");
                    out.push_str(&format!(
                        "            {}(v) => {{\n\
                         \x20               let raw = {raw}.to_be_bytes();\n",
                        arm.path
                    ));
                    for j in 0..n {
                        let i = if j == 0 {
                            "pos".to_string()
                        } else {
                            format!("pos + {j}")
                        };
                        out.push_str(&format!("                out[{i}] = raw[{j}];\n"));
                    }
                    out.push_str(&format!("                pos += {n};\n            }}\n"));
                }
                out.push_str("        }\n");
            }
        }
    }
    out.push_str("        Ok(pos)\n");
    out
}

fn parse_fn(def: &MessageDef, lay: &Layout<'_>) -> String {
    let mut out = String::new();
    let fallible = any_fallible(def);
    let constness = if lay.regions.iter().any(|r| matches!(r, Region::Rep(_))) {
        ""
    } else {
        "const "
    };

    out.push_str("    /// Parse the bytes as `Self`.\n");
    for c in def.ctx {
        out.push_str(&format!("    ///\n    /// `{}`: {}\n", c.name, c.doc));
    }
    out.push_str("    ///\n    /// # Errors\n    ///\n");
    if fallible {
        out.push_str(
            "    /// Returns [`ParsingError::Truncated`] on short input and\n\
             \x20   /// [`ParsingError::ReservedValue`] when a field carries a\n\
             \x20   /// reserved code point.\n",
        );
    } else {
        out.push_str("    /// Returns [`ParsingError::Truncated`] on short input.\n");
    }
    let ctx_params: String = def
        .ctx
        .iter()
        .map(|c| format!(", {}: {}", c.name, c.ty))
        .collect();
    let ret_ty = if lay
        .regions
        .iter()
        .any(|r| matches!(r, Region::Tail(_) | Region::CSlice(_)))
    {
        format!("{}<'_>", def.name)
    } else {
        "Self".to_string()
    };
    out.push_str(&format!(
        "    pub {constness}fn parse(buffer: &[u8]{ctx_params}) -> Result<{ret_ty}, ParsingError> {{\n"
    ));
    out.push_str(&parse_dynamic_body(def, lay));
    let ctor = if ret_ty == "Self" { "Self" } else { def.name };
    out.push_str(&format!("        Ok({ctor} {{\n"));
    for sf in &struct_fields(def) {
        if let SField::FGroup(g) = sf {
            out.push_str(&format!("            {}: {},\n", g.name, g.construct));
            continue;
        }
        let (name, _, _) = sfield_decl(sf);
        out.push_str(&format!("            {name},\n"));
    }
    out.push_str("        })\n    }\n");
    out
}

/// Length check, byte locals, control lets, prefix fields and dynamic
/// regions of a parse body (everything before the final `Ok`).
fn parse_dynamic_body(def: &MessageDef, lay: &Layout<'_>) -> String {
    let mut out = String::new();
    let n_bytes = lay.prefix_bytes;
    if n_bytes == 1 {
        out.push_str(
            "        if buffer.is_empty() {\n            return Err(ParsingError::Truncated);\n        }\n",
        );
    } else {
        out.push_str(&format!(
            "        if buffer.len() < {n_bytes} {{\n            return Err(ParsingError::Truncated);\n        }}\n"
        ));
    }
    let mut local_bytes: Vec<usize> = lay
        .prefix
        .iter()
        .filter(|s| s.field.bits < 8)
        .map(|s| s.bit / 8)
        .chain(
            lay.ctrls
                .iter()
                .filter(|c| !matches!(c, Ctrl::Const { .. }))
                .map(|c| ctrl_bit(c) / 8),
        )
        .collect();
    local_bytes.sort_unstable();
    local_bytes.dedup();
    for k in local_bytes {
        out.push_str(&format!("        let b{k} = buffer[{k}];\n"));
    }
    for c in &lay.ctrls {
        match c {
            Ctrl::Flag { of, bit } => {
                let k = bit / 8;
                let mask = 1u8 << (7 - bit % 8);
                match of.split_once('.') {
                    None => out.push_str(&format!(
                        "        let {of}_present = b{k} & 0x{mask:02X} != 0;\n"
                    )),
                    // A set bit without its parent present is ignored
                    // per the receiver-ignores-reserved convention.
                    Some((parent, child)) => out.push_str(&format!(
                        "        let {child}_present = {parent}_present && b{k} & 0x{mask:02X} != 0;\n"
                    )),
                }
            }
            Ctrl::Count { of, bit, bits } => {
                let k = bit / 8;
                let o = bit % 8;
                let shift = 8 - o - *bits as usize;
                let mask = (1u16 << bits) - 1;
                let read = if shift == 0 {
                    format!("b{k} & 0x{mask:02X}")
                } else if o == 0 {
                    format!("b{k} >> {shift}")
                } else {
                    format!("(b{k} >> {shift}) & 0x{mask:02X}")
                };
                match count_target(def, of) {
                    CountTarget::Rep(r) if r.bias > 0 => {
                        out.push_str(&format!(
                            "        let {of}_count = (({read}) + {}) as usize;\n",
                            r.bias
                        ));
                    }
                    CountTarget::Rep(Repeat {
                        all_escape: Some(_),
                        ..
                    }) => {
                        out.push_str(&format!("        let {of}_count_raw = {read};\n"));
                    }
                    CountTarget::Rep(r) => {
                        out.push_str(&format!("        let {of}_count = ({read}) as usize;\n"));
                        if r.reserved_max {
                            out.push_str(&format!(
                                "        if {of}_count == {mask} {{\n\
                                 \x20           return Err(ParsingError::ReservedValue);\n\
                                 \x20       }}\n"
                            ));
                        }
                    }
                    CountTarget::Slice(cs) => match &cs.all_ones {
                        AllOnes::Reserved => {
                            out.push_str(&format!(
                                "        let {of}_count = ({read}) as usize;\n\
                                 \x20       if {of}_count == {mask} {{\n\
                                 \x20           return Err(ParsingError::ReservedValue);\n\
                                 \x20       }}\n"
                            ));
                        }
                        AllOnes::All { .. } => {
                            out.push_str(&format!("        let {of}_count_raw = {read};\n"));
                        }
                    },
                }
            }
            Ctrl::Sel { of, bit } => {
                let k = bit / 8;
                let mask = 1u8 << (7 - bit % 8);
                out.push_str(&format!(
                    "        let {of}_wide = b{k} & 0x{mask:02X} != 0;\n"
                ));
            }
            Ctrl::Const { .. } => {}
            Ctrl::Mode { tag, bit } => {
                let k = bit / 8;
                let o = bit % 8;
                let shift = 8 - o - tag.bits as usize;
                let mask = (1u16 << tag.bits) - 1;
                let read = if shift == 0 {
                    format!("b{k} & 0x{mask:02X}")
                } else if o == 0 {
                    format!("b{k} >> {shift}")
                } else {
                    format!("(b{k} >> {shift}) & 0x{mask:02X}")
                };
                let of = tag.of;
                out.push_str(&format!(
                    "        let {of}_mode_raw = {read};\n\
                     \x20       let {of}_mode = if {of}_mode_raw == 0 {{\n\
                     \x20           None\n\
                     \x20       }} else {{\n\
                     \x20           let Some(m) = {}::{}({of}_mode_raw) else {{\n\
                     \x20               return Err(ParsingError::ReservedValue);\n\
                     \x20           }};\n\
                     \x20           Some(m)\n\
                     \x20       }};\n",
                    tag.ty, tag.ctor
                ));
            }
        }
    }
    for s in &lay.prefix {
        out.push_str(&parse_stmt(def, s, false, true, "        "));
    }

    let needs_pos = lay
        .regions
        .iter()
        .any(|r| !matches!(r, Region::Tail(_) | Region::CSlice(_)));
    if !lay.regions.is_empty() {
        if needs_pos {
            out.push_str(&format!("        let mut pos = {n_bytes};\n"));
        }
        for region in &lay.regions {
            match region {
                Region::Fixed { slots, bytes } => {
                    out.push_str(&check_rel(*bytes, "        "));
                    for s in slots {
                        out.push_str(&parse_stmt(def, s, true, false, "        "));
                    }
                    out.push_str(&format!("        pos += {bytes};\n"));
                }
                Region::Opt(g) => {
                    match gate_of(def, g.name) {
                        Gate::Flag => out.push_str(&format!(
                            "        let {} = if {}_present {{\n",
                            g.name, g.name
                        )),
                        Gate::Mode => out.push_str(&format!(
                            "        let {} = if let Some(mode) = {}_mode {{\n",
                            g.name, g.name
                        )),
                    }
                    if let [Item::Switch(s)] = g.items {
                        let Wide::Ctx(expr) = &s.wide else {
                            unreachable!()
                        };
                        out.push_str(&format!(
                            "            if {} {{\n",
                            cond_expr(def, expr, false)
                        ));
                        out.push_str(&check_rel(2, "                "));
                        out.push_str(&format!(
                            "                let v = {};\n\
                             \x20               pos += 2;\n\
                             \x20               Some(v)\n\
                             \x20           }} else {{\n",
                            wide_read(s)
                        ));
                        out.push_str(&check_rel(1, "                "));
                        out.push_str(
                            "                let v = buffer[pos] as u16;\n\
                             \x20               pos += 1;\n\
                             \x20               Some(v)\n\
                             \x20           }\n",
                        );
                    } else {
                        let parts = group_parts(def, g);
                        let slots = run_slots(parts.statics);
                        let bytes = run_bits(parts.statics) / 8;
                        out.push_str(&check_rel(bytes, "            "));
                        match &g.composite {
                            None => {
                                let s = &slots[0];
                                out.push_str(&parse_binding(
                                    def,
                                    s.field,
                                    s.bit,
                                    true,
                                    false,
                                    "            ",
                                    "v",
                                ));
                                out.push_str(&format!(
                                    "            pos += {bytes};\n            Some(v)\n"
                                ));
                            }
                            Some(c) => {
                                for s in &slots {
                                    out.push_str(&parse_stmt(def, s, true, false, "            "));
                                }
                                out.push_str(&format!("            pos += {bytes};\n"));
                                for n in &parts.nested {
                                    let nslots = run_slots(n.items);
                                    let nbytes = run_bits(n.items) / 8;
                                    let ns = &nslots[0];
                                    out.push_str(&format!(
                                        "            let {} = if {}_present {{\n",
                                        n.name, n.name
                                    ));
                                    out.push_str(&check_rel(nbytes, "                "));
                                    out.push_str(&parse_binding(
                                        def,
                                        ns.field,
                                        ns.bit,
                                        true,
                                        false,
                                        "                ",
                                        "v",
                                    ));
                                    out.push_str(&format!(
                                        "                pos += {nbytes};\n\
                                         \x20               Some(v)\n\
                                         \x20           }} else {{\n\
                                         \x20               None\n\
                                         \x20           }};\n"
                                    ));
                                }
                                out.push_str(&format!("            Some({})\n", c.construct));
                            }
                        }
                    }
                    out.push_str("        } else {\n            None\n        };\n");
                }
                Region::Rep(r) => {
                    let slots = run_slots(r.items);
                    let bytes = run_bits(r.items) / 8;
                    let (_, bits) = find_count(&lay.ctrls, r.name);
                    let vec_name = if r.all_escape.is_some() { "f" } else { r.name };
                    if let Some(AllEscape { all, .. }) = &r.all_escape {
                        let allones = (1u32 << bits) - 1;
                        out.push_str(&format!(
                            "        let {0} = if {0}_count_raw == {allones} {{\n\
                             \x20           {all}\n\
                             \x20       }} else {{\n\
                             \x20       let {0}_count = {0}_count_raw as usize;\n",
                            r.name
                        ));
                    }
                    let need = if bytes == 1 {
                        format!("{}_count", r.name)
                    } else {
                        format!("{}_count * {bytes}", r.name)
                    };
                    out.push_str(&format!(
                        "        if buffer.len() < pos + {need} {{\n\
                         \x20           return Err(ParsingError::Truncated);\n\
                         \x20       }}\n"
                    ));
                    out.push_str(&format!("        let mut {vec_name} = Vec::new();\n"));
                    out.push_str(&format!("        for _ in 0..{}_count {{\n", r.name));
                    match &r.composite {
                        None => {
                            let s = &slots[0];
                            out.push_str(&parse_binding(
                                def,
                                s.field,
                                s.bit,
                                true,
                                false,
                                "            ",
                                "v",
                            ));
                        }
                        Some(c) => {
                            for s in &slots {
                                out.push_str(&parse_stmt(def, s, true, false, "            "));
                            }
                            out.push_str(&format!("            let v = {};\n", c.construct));
                        }
                    }
                    out.push_str(&format!(
                        "            pos += {bytes};\n\
                         \x20           {vec_name}.push(v)\n\
                         \x20               .expect(\"count bounded by the {bits}-bit count field\");\n\
                         \x20       }}\n"
                    ));
                    if let Some(AllEscape { specific, .. }) = &r.all_escape {
                        out.push_str(&format!("        {specific}({vec_name})\n        }};\n"));
                    }
                }
                Region::Sw(s) => {
                    let name = s.field.name;
                    let cond = match &s.wide {
                        Wide::ValueOverflow => format!("{name}_wide"),
                        Wide::Ctx(expr) => cond_expr(def, expr, false),
                    };
                    out.push_str(&format!("        let {name} = if {cond} {{\n"));
                    out.push_str(&check_rel(2, "            "));
                    out.push_str(&format!(
                        "            let v = {};\n            pos += 2;\n            v\n\
                         \x20       }} else {{\n",
                        wide_read(s)
                    ));
                    out.push_str(&check_rel(1, "            "));
                    out.push_str(
                        "            let v = buffer[pos] as u16;\n\
                         \x20           pos += 1;\n            v\n        };\n",
                    );
                }
                Region::Inline(g) => {
                    let parts = inline_parts(def, g);
                    let c = g.composite.as_ref().expect("inline composite");
                    if let Some(sw) = parts.switch {
                        let Wide::Ctx(expr) = &sw.wide else {
                            unreachable!()
                        };
                        out.push_str(&format!(
                            "        let {} = if {} {{\n",
                            sw.field.name,
                            cond_expr(def, expr, false)
                        ));
                        out.push_str(&check_rel(2, "            "));
                        out.push_str(&format!(
                            "            let v = {};\n            pos += 2;\n            v\n\
                             \x20       }} else {{\n",
                            wide_read(sw)
                        ));
                        out.push_str(&check_rel(1, "            "));
                        out.push_str(
                            "            let v = buffer[pos] as u16;\n\
                             \x20           pos += 1;\n            v\n        };\n",
                        );
                    }
                    let slots = run_slots(parts.statics);
                    let bytes = run_bits(parts.statics) / 8;
                    out.push_str(&check_rel(bytes, "        "));
                    for sl in &slots {
                        out.push_str(&parse_stmt(def, sl, true, false, "        "));
                    }
                    out.push_str(&format!("        pos += {bytes};\n"));
                    out.push_str(&format!("        let {} = {};\n", g.name, c.construct));
                }
                Region::Var(v) => {
                    out.push_str(&format!("        let {} = if {}_wide {{\n", v.name, v.name));
                    for (i, arm) in [(0usize, &v.wide), (1, &v.narrow)] {
                        let n = arm.bits as usize / 8;
                        out.push_str(&check_rel(n, "            "));
                        out.push_str(&format!(
                            "            let raw = {};\n            pos += {n};\n",
                            rel_word_read(arm.bits)
                        ));
                        match &arm.ty {
                            Ty::Raw => out.push_str(&format!("            {}(raw)\n", arm.path)),
                            Ty::Bool => panic!("bool variant arm"),
                            Ty::Fallible { ty, ctor, .. } => {
                                out.push_str(&format!(
                                    "            let Some(v) = {ty}::{ctor}(raw) else {{\n\
                                     \x20               return Err(ParsingError::ReservedValue);\n\
                                     \x20           }};\n"
                                ));
                                out.push_str(&format!("            {}(v)\n", arm.path));
                            }
                            Ty::Wrap { construct, .. } => out.push_str(&format!(
                                "            {}({})\n",
                                arm.path,
                                construct.replace("{}", "raw")
                            )),
                        }
                        if i == 0 {
                            out.push_str("        } else {\n");
                        }
                    }
                    out.push_str("        };\n");
                }
                Region::Tail(ts) => {
                    let at = if needs_pos {
                        "pos".to_string()
                    } else {
                        n_bytes.to_string()
                    };
                    out.push_str(&format!(
                        "        let (_, rest) = buffer.split_at({at});\n\
                         \x20       // SAFETY: {0} is #[repr(transparent)] over u8 and has no\n\
                         \x20       // validity invariant (any byte is representable).\n\
                         \x20       let {1}: &[{0}] = unsafe {{\n\
                         \x20           core::slice::from_raw_parts(rest.as_ptr().cast::<{0}>(), rest.len())\n\
                         \x20       }};\n",
                        ts.ty, ts.name
                    ));
                }
                Region::CSlice(cs) => {
                    let at = if needs_pos {
                        "pos".to_string()
                    } else {
                        n_bytes.to_string()
                    };
                    let allones = (1u32 << count_bits(&lay.ctrls, cs.name)) - 1;
                    let safety = format!(
                        "// SAFETY: {0} is #[repr(transparent)] over u8 and has no\n\
                         {{i}}// validity invariant (any byte is representable).\n",
                        cs.ty
                    );
                    match &cs.all_ones {
                        AllOnes::Reserved => {
                            out.push_str(&format!(
                                "        if buffer.len() < {at} + {0}_count {{\n\
                                 \x20           return Err(ParsingError::Truncated);\n\
                                 \x20       }}\n\
                                 \x20       let (_, rest) = buffer.split_at({at});\n\
                                 \x20       let (raw, _) = rest.split_at({0}_count);\n\
                                 \x20       {1}\
                                 \x20       let {0}: &[{2}] = unsafe {{\n\
                                 \x20           core::slice::from_raw_parts(raw.as_ptr().cast::<{2}>(), raw.len())\n\
                                 \x20       }};\n",
                                cs.name,
                                safety.replace("{i}", "        "),
                                cs.ty
                            ));
                        }
                        AllOnes::All { all, specific, .. } => {
                            out.push_str(&format!(
                                "        let {0} = if {0}_count_raw == {allones} {{\n\
                                 \x20           {all}\n\
                                 \x20       }} else {{\n\
                                 \x20           let count = {0}_count_raw as usize;\n\
                                 \x20           if buffer.len() < {at} + count {{\n\
                                 \x20               return Err(ParsingError::Truncated);\n\
                                 \x20           }}\n\
                                 \x20           let (_, rest) = buffer.split_at({at});\n\
                                 \x20           let (raw, _) = rest.split_at(count);\n\
                                 \x20           {1}\
                                 \x20           let f: &[{2}] = unsafe {{\n\
                                 \x20               core::slice::from_raw_parts(raw.as_ptr().cast::<{2}>(), raw.len())\n\
                                 \x20           }};\n\
                                 \x20           {specific}(f)\n\
                                 \x20       }};\n",
                                cs.name,
                                safety.replace("{i}", "            "),
                                cs.ty
                            ));
                        }
                    }
                }
            }
        }
        if !matches!(
            lay.regions.last(),
            Some(Region::Tail(_) | Region::CSlice(_))
        ) {
            out.push_str("        let _ = pos;\n");
        }
    }
    out
}

/// Emit src/mac/messages/generated.rs (the module index).
pub fn emit_index(defs: &[MessageDef]) -> String {
    let mut out = String::from(
        "// GENERATED by codegen - DO NOT EDIT. Regenerate with `make codegen`.\n\n\
         //! Generated MAC message codecs.\n\
         //!\n\
         //! Each module is emitted by the `codegen/` crate from a\n\
         //! declarative layout definition and carries an ASCII wire-layout\n\
         //! figure in its module documentation.\n\n",
    );
    let mut modules: Vec<&str> = defs.iter().map(|d| d.module).collect();
    modules.sort_unstable();
    for m in modules {
        out.push_str(&format!("pub mod {m};\n"));
    }
    out
}

/// Identifier-boundary replacement of `self.<name>` with an arm
/// binding path.
fn bind_subst(code: &str, binds: &[(&str, &str)]) -> String {
    let mut s = code.to_string();
    for (name, path) in binds {
        let needle = format!("self.{name}");
        let mut outp = String::with_capacity(s.len());
        let mut rest = s.as_str();
        while let Some(i) = rest.find(&needle) {
            let after = &rest[i + needle.len()..];
            let boundary = after
                .chars()
                .next()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
            outp.push_str(&rest[..i]);
            if boundary {
                outp.push_str(path);
            } else {
                outp.push_str(&needle);
            }
            rest = &rest[i + needle.len()..];
        }
        outp.push_str(rest);
        s = outp;
    }
    s
}

/// Per-arm view: the arm's items in the surrounding def's context.
fn arm_def(def: &MessageDef, arm: &VArm) -> MessageDef {
    MessageDef {
        items: arm.items,
        ..*def
    }
}

/// Emit a module whose body is an enum dispatched on leading bits.
fn emit_enum_message(def: &MessageDef, vb: &VariantBody) -> String {
    let arm_lays: Vec<(MessageDef, &VArm)> = vb.arms.iter().map(|a| (arm_def(def, a), a)).collect();
    for (ad, a) in &arm_lays {
        assert!(
            matches!(a.items.first(), Some(Item::Const { bits, value, .. })
                if *bits == vb.bits && *value == a.value),
            "{}: arm {} must start with its discriminant Const",
            def.name,
            a.pattern
        );
        let lay = layout(ad);
        assert!(
            !lay.regions
                .iter()
                .any(|r| matches!(r, Region::Tail(_) | Region::CSlice(_))),
            "{}: borrowed slices are not supported inside enum arms",
            def.name
        );
    }
    let has_repeat = arm_lays.iter().any(|(ad, _)| {
        layout(ad)
            .regions
            .iter()
            .any(|r| matches!(r, Region::Rep(_)))
    });
    let constness = if has_repeat { "" } else { "const " };
    let scrutinee = if def.ctx.is_empty() {
        "self".to_string()
    } else {
        format!("self.{}", vb.name)
    };

    let (fig, legend) = figure::render(def);
    let mut out = String::new();
    out.push_str(&format!(
        "// GENERATED by codegen/src/defs/{}.rs - DO NOT EDIT.\n\
         // Regenerate with `make codegen`.\n\n",
        def.module
    ));
    out.push_str(&format!("//! {} generated codec.\n//!\n", def.name));
    out.push_str(&format!("//! {}.\n//!\n", def.spec));
    out.push_str("//! Wire layout (ETSI bit numbering, column 0 = MSB):\n//!\n//! ```text\n");
    for line in fig.lines() {
        if line.is_empty() {
            out.push_str("//!\n");
        } else {
            out.push_str(&format!("//! {line}\n"));
        }
    }
    out.push_str("//! ```\n");
    if !legend.is_empty() {
        out.push_str("//!\n");
        for l in &legend {
            out.push_str(&format!("//! * {l}\n"));
        }
    }
    out.push('\n');

    if def.ie_type.is_some() {
        out.push_str("use crate::mac::pdu::MessageBody;\n");
    }
    if def.short_ie.is_some() {
        out.push_str("use crate::mac::pdu::ShortMessageBody;\n");
    }
    for imp in def.imports {
        out.push_str(imp);
        out.push('\n');
    }
    out.push_str("use crate::types::*;\nuse crate::{ExcessiveBitsSet, ParsingError};\n");
    if has_repeat {
        out.push_str("use heapless::Vec;\n");
    }
    out.push('\n');

    for (ad, _) in &arm_lays {
        let lay = layout(ad);
        for region in &lay.regions {
            if let Region::Rep(r) = region {
                let (_, bits) = find_count(&lay.ctrls, r.name);
                let max = (1u32 << bits) - 1 + u32::from(r.bias)
                    - u32::from(r.reserved_max || r.all_escape.is_some());
                out.push_str(&format!(
                    "/// {}\npub const {}: usize = {max};\n\n",
                    r.max_doc, r.max_const
                ));
            }
        }
    }

    if !def.ctx.is_empty() {
        out.push_str(&format!("/// {}\n", def.doc));
        if has_repeat {
            out.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
        } else {
            out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
        }
        out.push_str("#[cfg_attr(feature = \"defmt\", derive(defmt::Format))]\n");
        out.push_str(&format!("pub struct {} {{\n", def.name));
        for c in def.ctx {
            out.push_str(&format!(
                "    /// {}\n    pub {}: {},\n",
                c.doc, c.name, c.ty
            ));
        }
        out.push_str(&format!(
            "    /// {}\n    pub {}: {},\n",
            vb.doc, vb.name, vb.ty
        ));
        out.push_str("}\n\n");
    }

    out.push_str(&format!("impl {} {{\n", def.name));

    out.push_str(&format!(
        "    /// Number of bytes [`Self::serialize`] will write.\n\
         \x20   #[must_use]\n\
         \x20   #[inline]\n\
         \x20   pub {constness}fn encoded_len(&self) -> usize {{\n\
         \x20       match {scrutinee} {{\n"
    ));
    for (ad, a) in &arm_lays {
        let lay = layout(ad);
        let base: usize = lay.prefix_bytes
            + lay
                .regions
                .iter()
                .map(|r| match r {
                    Region::Fixed { bytes, .. } => *bytes,
                    _ => 0,
                })
                .sum::<usize>();
        if lay
            .regions
            .iter()
            .all(|r| matches!(r, Region::Fixed { .. }))
        {
            out.push_str(&format!("            {} => {base},\n", a.len_pattern));
        } else {
            let body = bind_subst(&len_regions(ad, &lay), a.binds);
            out.push_str(&format!(
                "            {} => {{\n                let mut len = {base};\n{body}                len\n            }}\n",
                a.len_pattern
            ));
        }
    }
    out.push_str("        }\n    }\n\n");

    out.push_str(
        "    /// Serialize the body into `out`. Returns the number of bytes written.\n\
         \x20   ///\n\
         \x20   /// # Errors\n\
         \x20   ///\n\
         \x20   /// Returns [`ExcessiveBitsSet`] if the buffer is too short or a\n\
         \x20   /// field cannot be encoded in its on-wire form.\n",
    );
    out.push_str(&format!(
        "    pub {constness}fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {{\n\
         \x20       match {scrutinee} {{\n"
    ));
    for (ad, a) in &arm_lays {
        let lay = layout(ad);
        let body = bind_subst(&ser_dynamic_body(ad, &lay), a.binds);
        out.push_str(&format!(
            "            {} => {{\n{body}            }}\n",
            a.pattern
        ));
    }
    out.push_str("        }\n    }\n\n");

    out.push_str("    /// Parse the bytes as `Self`.\n");
    for c in def.ctx {
        out.push_str(&format!("    ///\n    /// `{}`: {}\n", c.name, c.doc));
    }
    out.push_str("    ///\n    /// # Errors\n    ///\n");
    // The variant dispatch itself rejects reserved discriminants, so
    // both error clauses apply regardless of the arms' field types.
    out.push_str(
        "    /// Returns [`ParsingError::Truncated`] on short input and\n\
         \x20   /// [`ParsingError::ReservedValue`] when a field carries a\n\
         \x20   /// reserved code point.\n",
    );
    let ctx_params: String = def
        .ctx
        .iter()
        .map(|c| format!(", {}: {}", c.name, c.ty))
        .collect();
    out.push_str(&format!(
        "    pub {constness}fn parse(buffer: &[u8]{ctx_params}) -> Result<Self, ParsingError> {{\n"
    ));
    out.push_str(
        "        if buffer.is_empty() {\n            return Err(ParsingError::Truncated);\n        }\n",
    );
    out.push_str(&format!("        match buffer[0] >> {} {{\n", 8 - vb.bits));
    for (ad, a) in &arm_lays {
        let lay = layout(ad);
        let body = parse_dynamic_body(ad, &lay);
        let ok = if def.ctx.is_empty() {
            format!("Ok({})", a.construct)
        } else {
            let ctx_names: Vec<&str> = def.ctx.iter().map(|c| c.name).collect();
            format!(
                "Ok(Self {{ {}, {}: {} }})",
                ctx_names.join(", "),
                vb.name,
                a.construct
            )
        };
        out.push_str(&format!(
            "            {} => {{\n{body}                {ok}\n            }}\n",
            a.value
        ));
    }
    out.push_str("            _ => Err(ParsingError::ReservedValue),\n        }\n    }\n}\n");

    if let Some(v) = def.ie_type {
        out.push_str(&format!(
            "\nimpl MessageBody for {} {{\n\
             \x20   const IE_TYPE: IEType6bit = IEType6bit::{v};\n\
             \x20   #[inline]\n\
             \x20   fn encoded_len(&self) -> usize {{\n        Self::encoded_len(self)\n    }}\n\
             \x20   #[inline]\n\
             \x20   fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {{\n\
             \x20       Self::serialize(self, out)\n    }}\n}}\n",
            def.name
        ));
    }
    if let Some(expr) = def.short_ie {
        out.push_str(&format!(
            "\nimpl ShortMessageBody for {} {{\n\
             \x20   const IE_TYPE: ShortIeType = {expr};\n\
             \x20   #[inline]\n\
             \x20   fn encoded_len(&self) -> usize {{\n        Self::encoded_len(self)\n    }}\n\
             \x20   #[inline]\n\
             \x20   fn serialize(&self, out: &mut [u8]) -> Result<usize, ExcessiveBitsSet> {{\n\
             \x20       Self::serialize(self, out)\n    }}\n}}\n",
            def.name
        ));
    }
    out
}
