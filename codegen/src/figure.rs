//! ASCII bit-layout figure renderer (hand-rolled by design: the
//! grid is fixed-geometry and general table crates cannot match the
//! ETSI figure style). ETSI column convention: bits 0..7 left to
//! right per byte row, column 0 = MSB. Each bit column is 4
//! characters wide.
//!
//! The fixed prefix renders as the main figure; each dynamic region
//! (optional / repeated / width-switched / variant) renders as a
//! captioned sub-figure below it.

use crate::ir::{Item, MessageDef, Wide};

/// One rendered segment: a label spanning `bits` columns.
struct Seg {
    label: String,
    bits: u8,
}

/// Render the layout figure plus a legend mapping abbreviated or
/// truncated labels to field names. Returns (figure, legend) where
/// legend entries are "label = field_name" strings.
pub fn render(def: &MessageDef) -> (String, Vec<String>) {
    let mut legend: Vec<String> = Vec::new();
    let header: String = (0..8).map(|i| format!("{i:>4}")).collect::<String>()[1..].to_string();
    let mut out = String::new();
    out.push_str(&header);
    out.push('\n');

    if let [Item::VariantBody(vb)] = def.items {
        for (i, arm) in vb.arms.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&format!(
                "{} = {:0w$b} ({}):\n",
                vb.fig,
                arm.value,
                arm.pattern,
                w = vb.bits as usize
            ));
            render_items(def, arm.items, &mut out, &mut legend);
        }
        legend.dedup();
        return (out, legend);
    }

    render_items(def, def.items, &mut out, &mut legend);
    legend.dedup();
    (out, legend)
}

/// Render one item run (prefix box plus captioned dynamic regions).
fn render_items(def: &MessageDef, items: &[Item], out: &mut String, legend: &mut Vec<String>) {
    let split = items
        .iter()
        .position(|i| {
            matches!(
                i,
                Item::Optional(_)
                    | Item::Repeat(_)
                    | Item::Switch(_)
                    | Item::Variant(_)
                    | Item::TailSlice(_)
                    | Item::CountSlice(_)
                    | Item::Inline(_)
            )
        })
        .unwrap_or(items.len());

    out.push_str(&boxed(&flatten(&items[..split], legend)));

    // Pending run of mandatory fields between dynamic regions.
    let mut fixed_run: Vec<Seg> = Vec::new();
    for item in &items[split..] {
        if matches!(item, Item::Field(_) | Item::Reserved { .. }) {
            fixed_run.extend(flatten(std::slice::from_ref(item), legend));
            continue;
        }
        if !fixed_run.is_empty() {
            out.push_str("\nAlways present:\n");
            out.push_str(&boxed(&std::mem::take(&mut fixed_run)));
        }
        match item {
            Item::Optional(g) => {
                let flag = ctrl_fig(items, g.name);
                let mode_gated = items
                    .iter()
                    .any(|i| matches!(i, Item::ModeTag(t) if t.of == g.name));
                let set = if mode_gated { "nonzero" } else { "set" };
                if let [Item::Switch(s)] = g.items {
                    let cond = cond_display(def, &s.wide);
                    out.push_str(&format!(
                        "\nPresent iff {flag} is {set}, when not ({cond}):\n"
                    ));
                    out.push_str(&boxed(&[seg(s.field.fig_label(), s.narrow_bits)]));
                    out.push_str(&format!("\nPresent iff {flag} is {set}, when {cond}:\n"));
                    out.push_str(&boxed(&[seg(s.field.fig_label(), s.field.bits)]));
                    field_legend(legend, s.field.fig_label(), s.field.name);
                } else {
                    let split = g
                        .items
                        .iter()
                        .position(|i| matches!(i, Item::Optional(_)))
                        .unwrap_or(g.items.len());
                    out.push_str(&format!("\nPresent iff {flag} is {set}:\n"));
                    out.push_str(&boxed(&flatten(&g.items[..split], legend)));
                    for item in &g.items[split..] {
                        let Item::Optional(n) = item else {
                            unreachable!()
                        };
                        let nflag = ctrl_fig(items, n.name);
                        out.push_str(&format!("\nPresent iff {flag} and {nflag} are set:\n"));
                        out.push_str(&boxed(&flatten(n.items, legend)));
                    }
                }
            }
            Item::Repeat(r) => {
                let (bits, fig) = count_fig(items, r.name);
                let min = r.bias;
                let max = (1u8 << bits) - 1 + r.bias;
                let caption = if r.all_escape.is_some() {
                    format!(
                        "\nRepeated {fig} times (0..={}; {fig} = {max} means all, no entries):\n",
                        max - 1
                    )
                } else if r.reserved_max {
                    format!(
                        "\nRepeated {fig} times (0..={}; {max} reserved):\n",
                        max - 1
                    )
                } else if r.bias > 0 {
                    format!("\nRepeated {min}..={max} times ({fig} + {}):\n", r.bias)
                } else {
                    format!("\nRepeated {min}..={max} times ({fig}):\n")
                };
                out.push_str(&caption);
                out.push_str(&boxed(&flatten(r.items, legend)));
            }
            Item::Switch(s) => {
                let sel = ctrl_fig(items, s.field.name);
                let label = s.field.fig_label();
                out.push_str(&format!("\n{label} when {sel} is clear:\n"));
                out.push_str(&boxed(&[seg(label, s.narrow_bits)]));
                out.push_str(&format!("\n{label} when {sel} is set:\n"));
                out.push_str(&boxed(&[seg(label, s.field.bits)]));
                field_legend(legend, label, s.field.name);
            }
            Item::Variant(v) => {
                let sel = ctrl_fig(items, v.name);
                out.push_str(&format!(
                    "\n{} when {sel} is clear ({}):\n",
                    v.fig, v.narrow.path
                ));
                out.push_str(&boxed(&[seg(v.fig, v.narrow.bits)]));
                out.push_str(&format!(
                    "\n{} when {sel} is set ({}):\n",
                    v.fig, v.wide.path
                ));
                out.push_str(&boxed(&[seg(v.fig, v.wide.bits)]));
                field_legend(legend, v.fig, v.name);
            }
            Item::Inline(g) => {
                let split = usize::from(matches!(g.items.first(), Some(Item::Switch(_))));
                if let Some(Item::Switch(sw)) = g.items.first() {
                    let cond = cond_display(def, &sw.wide);
                    let label = sw.field.fig_label();
                    out.push_str(&format!("\n{label} when not ({cond}):\n"));
                    out.push_str(&boxed(&[seg(label, sw.narrow_bits)]));
                    out.push_str(&format!("\n{label} when {cond}:\n"));
                    if sw.wide_bits == 16 {
                        out.push_str(&boxed(&[seg(label, 16)]));
                    } else {
                        out.push_str(&boxed(&[
                            seg("Reserved", 16 - sw.wide_bits),
                            seg(label, sw.wide_bits),
                        ]));
                    }
                    field_legend(legend, label, sw.field.name);
                }
                // The static remainder joins the surrounding mandatory
                // run so consecutive always-present bytes share one box.
                fixed_run.extend(flatten(&g.items[split..], legend));
            }
            Item::TailSlice(ts) => {
                out.push_str("\nRepeated for the rest of the body:\n");
                out.push_str(&boxed(&[seg(ts.fig, 8)]));
                field_legend(legend, ts.fig, ts.name);
            }
            Item::CountSlice(cs) => {
                let (bits, fig) = count_fig(items, cs.name);
                let max = (1u8 << bits) - 1;
                let caption = match &cs.all_ones {
                    crate::ir::AllOnes::Reserved => {
                        format!(
                            "\nRepeated {fig} times (0..={}; {max} reserved):\n",
                            max - 1
                        )
                    }
                    crate::ir::AllOnes::All { .. } => format!(
                        "\nRepeated {fig} times (0..={}; {fig} = {max} means all, no entries):\n",
                        max - 1
                    ),
                };
                out.push_str(&caption);
                out.push_str(&boxed(&[seg(cs.fig, 8)]));
                field_legend(legend, cs.fig, cs.name);
            }
            _ => unreachable!(),
        }
    }
    if !fixed_run.is_empty() {
        out.push_str("\nAlways present:\n");
        out.push_str(&boxed(&fixed_run));
    }
}

fn seg(label: &str, bits: u8) -> Seg {
    Seg {
        label: label.to_string(),
        bits,
    }
}

fn field_legend(legend: &mut Vec<String>, label: &str, name: &str) {
    if label != name {
        legend.push(format!("{label} = {name}"));
    }
}

/// Figure label of the PresenceFlag / Selector controlling `of`.
fn ctrl_fig<'a>(items: &'a [Item], of: &str) -> &'a str {
    items
        .iter()
        .find_map(|i| match i {
            Item::PresenceFlag { of: o, fig } | Item::Selector { of: o, fig }
                if *o == of || o.split_once('.').is_some_and(|(_, c)| c == of) =>
            {
                Some(*fig)
            }
            Item::ModeTag(tag) if tag.of == of => Some(tag.fig),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no flag/selector for {of}"))
}

/// (bits, figure label) of the Count controlling `of`.
fn count_fig<'a>(items: &'a [Item], of: &str) -> (u8, &'a str) {
    items
        .iter()
        .find_map(|i| match i {
            Item::Count { of: o, bits, fig } if *o == of => Some((*bits, *fig)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no count for {of}"))
}

/// Human form of a width condition ("{mu}.as_u8() > 4" -> ctx access).
fn cond_display(def: &MessageDef, wide: &Wide) -> String {
    match wide {
        Wide::ValueOverflow => "the value exceeds the narrow form".into(),
        Wide::Ctx(expr) => {
            let mut s = (*expr).to_string();
            for c in def.ctx {
                s = s.replace(&format!("{{{}}}", c.name), c.name);
            }
            s
        }
    }
}

/// Flatten static items (fields, reserved runs, bitmap controls)
/// into figure segments, collecting legend entries.
fn flatten(items: &[Item], legend: &mut Vec<String>) -> Vec<Seg> {
    let mut segs = Vec::new();
    for item in items {
        match item {
            Item::Field(f) => {
                field_legend(legend, f.fig_label(), f.name);
                segs.push(seg(f.fig_label(), f.bits));
            }
            Item::Reserved { bits } => {
                let label = if *bits <= 2 { "R" } else { "Reserved" };
                segs.push(seg(label, *bits));
            }
            Item::PresenceFlag { of, fig } => {
                legend.push(format!("{fig} = {of} present"));
                segs.push(seg(fig, 1));
            }
            Item::Count { of, bits, fig } => {
                legend.push(format!("{fig} = {of} count"));
                segs.push(seg(fig, *bits));
            }
            Item::Selector { of, fig } => {
                legend.push(format!("{fig} = {of} selector"));
                segs.push(seg(fig, 1));
            }
            Item::ModeTag(tag) => {
                legend.push(format!("{} = {} mode (0 = absent)", tag.fig, tag.of));
                segs.push(seg(tag.fig, tag.bits));
            }
            Item::Const { bits, fig, .. } => {
                segs.push(seg(fig, *bits));
            }
            _ => panic!("dynamic item inside a static run"),
        }
    }
    segs
}

/// Render segments as a bordered box, splitting at byte rows.
fn boxed(segs: &[Seg]) -> String {
    // Split segments into 8-bit rows.
    let mut rows: Vec<Vec<Seg>> = vec![Vec::new()];
    let mut bit = 0usize;
    for s in segs {
        let mut bits = s.bits;
        while bits > 0 {
            let row_used = (bit % 8) as u8;
            let in_row = (8 - row_used).min(bits);
            if rows.last().unwrap().iter().map(|s| s.bits).sum::<u8>() == 8 {
                rows.push(Vec::new());
            }
            rows.last_mut().unwrap().push(Seg {
                label: s.label.clone(),
                bits: in_row,
            });
            bit += in_row as usize;
            bits -= in_row;
        }
    }
    assert!(bit.is_multiple_of(8), "figure run is not byte aligned");

    // Border between two rows: '+' where either adjacent row has a
    // segment boundary, '-' elsewhere.
    let mut out = String::new();
    for (i, row) in rows.iter().enumerate() {
        let above: &[Seg] = if i == 0 { &[] } else { &rows[i - 1] };
        out.push_str(&border(above, row));
        out.push('\n');
        out.push_str(&row_line(row));
        out.push('\n');
    }
    out.push_str(&border(rows.last().unwrap(), &[]));
    out.push('\n');
    out
}

/// Bit positions (0..=8) where a segment edge sits in `row`.
fn edges(row: &[Seg]) -> [bool; 9] {
    let mut e = [false; 9];
    if !row.is_empty() {
        e[0] = true;
        e[8] = true;
    }
    let mut at = 0usize;
    for s in row {
        at += s.bits as usize;
        e[at] = true;
    }
    e
}

fn border(above: &[Seg], below: &[Seg]) -> String {
    let a = edges(above);
    let b = edges(below);
    let mut line = String::new();
    for pos in 0..=8 {
        line.push(if a[pos] || b[pos] { '+' } else { '-' });
        if pos < 8 {
            line.push_str("---");
        }
    }
    line
}

fn row_line(row: &[Seg]) -> String {
    let mut line = String::from("|");
    for s in row {
        let width = s.bits as usize * 4 - 1;
        let mut label = s.label.clone();
        if label.len() > width {
            label.truncate(width);
        }
        line.push_str(&format!("{label:^width$}"));
        line.push('|');
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defs;

    /// Pinned snapshot: MAC Security Info (figure regressions must
    /// show up in `--check` like code regressions do).
    #[test]
    fn mac_security_info_snapshot() {
        let def = defs::all()
            .into_iter()
            .find(|d| d.module == "mac_security_info")
            .unwrap();
        let (fig, legend) = render(&def);
        let expected = "  0   1   2   3   4   5   6   7
+-------+-------+---------------+
|Version|Key Idx|    IV Type    |
+-------+-------+---------------+
|              HPC              |
+-------------------------------+
|              HPC              |
+-------------------------------+
|              HPC              |
+-------------------------------+
|              HPC              |
+-------------------------------+
";
        assert_eq!(fig, expected, "\nactual:\n{fig}");
        assert_eq!(
            legend,
            [
                "Version = version",
                "Key Idx = key_index",
                "IV Type = iv_type",
                "HPC = hpc"
            ]
        );
    }
}
