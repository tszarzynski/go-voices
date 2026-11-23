//! Converts GO:Plus's `GO-sounds.md` (a Markdown table: patch, category,
//! MSB, LSB, PC) into `catalogue/data/patches.tsv`.
//!
//! `import-sounds <GO-sounds.md> <patches.tsv>`
//!
//! Rows that aren't a patch the keyboard has are left out and listed on
//! stderr, so they never reach the app. Known mistakes in the table are
//! corrected ([`CORRECTIONS`]).

use std::{env, fmt::Write, fs};

use anyhow::{Context, Result, bail};
use catalogue::Patch;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let [source, target] = args.as_slice() else {
        bail!("usage: import-sounds <GO-sounds.md> <patches.tsv>");
    };
    let markdown = fs::read_to_string(source).with_context(|| format!("reading {source}"))?;
    let (patches, skipped) = convert(&markdown);
    for (line, row, error) in &skipped {
        eprintln!("{source}:{line}: skipped ({error}): {row}");
    }
    fs::write(target, tsv(&patches)).with_context(|| format!("writing {target}"))?;
    eprintln!(
        "{} patches written to {target}, {} rows skipped",
        patches.len(),
        skipped.len()
    );
    Ok(())
}

/// Rows whose program is wrong in `GO-sounds.md`: name, MSB, LSB, and the
/// right program. In GM2, Clavi Bass and Hammer are variations of Synth
/// Bass 1 (program 39); the table gives 40, which is Attack Pulse's
/// address for Clavi Bass and no sound for Hammer.
const CORRECTIONS: [(&str, u8, u8, u8); 2] = [("Clavi Bass", 121, 3, 39), ("Hammer", 121, 4, 39)];

/// `patch`, with its program corrected if it's a known mistake.
fn corrected(mut patch: Patch) -> Patch {
    if let Some(&(.., pc)) = CORRECTIONS
        .iter()
        .find(|&&(name, msb, lsb, _)| patch.name == name && patch.msb == msb && patch.lsb == lsb)
    {
        patch.pc = pc;
    }
    patch
}

/// The table's patches, and the rows that aren't one (line number, the row
/// and why).
fn convert(markdown: &str) -> (Vec<Patch>, Vec<(usize, &str, String)>) {
    let mut patches = Vec::new();
    let mut skipped = Vec::new();
    for (i, row) in table_rows(markdown) {
        let line = row
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\t");
        match line.parse::<Patch>() {
            Ok(patch) => patches.push(corrected(patch)),
            Err(e) => skipped.push((i + 1, row.trim(), e.to_string())),
        }
    }
    (patches, skipped)
}

/// The table's body rows, with their line indices: rows after the header
/// and its `|---|` separator.
fn table_rows(markdown: &str) -> impl Iterator<Item = (usize, &str)> {
    markdown
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with('|'))
        .skip(2)
}

fn tsv(patches: &[Patch]) -> String {
    patches.iter().fold(String::new(), |mut out, p| {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            p.name,
            p.category.code(),
            p.msb,
            p.lsb,
            p.pc
        );
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "\
| Patch | Category | MSB | LSB | PC |
|---|---|---|---|---|
|\t88StageGrand\t|\tPNO\t|\t087\t|\t064\t|\t001\t|
|\tGO Grand\t|\tPNO\t|\t0\t|\t68\t|\t1\t|
";

    #[test]
    fn converts_rows_and_reports_the_bad_ones() {
        let (patches, skipped) = convert(TABLE);
        assert_eq!(tsv(&patches), "88StageGrand\tPNO\t87\t64\t1\n");
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].0, 4);
        assert_eq!(skipped[0].2, "no bank with MSB 0");
    }

    #[test]
    fn corrects_known_mistakes() {
        let (patches, _) = convert(
            "| Patch | Category | MSB | LSB | PC |\n|---|---|---|---|---|\n| Clavi Bass | SBS | 121 | 3 | 40 |\n",
        );
        assert_eq!(tsv(&patches), "Clavi Bass\tSBS\t121\t3\t39\n");
    }
}
