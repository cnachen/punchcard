use crate::deck::{CardRecord, CardType, ColumnRange, EncodingKind};
use anyhow::{Result, anyhow};

/// Describes a language or workload-specific punch card layout.
#[derive(Debug, Clone)]
pub struct Template {
    pub name: &'static str,
    pub description: &'static str,
    pub columns: &'static [TemplateColumn],
    pub default_type: CardType,
}

/// Column constraint metadata for a [`Template`].
#[derive(Debug, Clone)]
pub struct TemplateColumn {
    pub range: ColumnRange,
    pub label: &'static str,
}

impl Template {
    /// Apply the template to raw text, returning a [`CardRecord`] with column padding and defaults.
    pub fn apply(&self, text: &str) -> Result<CardRecord> {
        CardRecord::from_text(text, EncodingKind::Hollerith, self.default_type.clone())
    }
}

/// Registry of built-in templates recognised by the CLI.
pub struct TemplateRegistry;

impl TemplateRegistry {
    /// Return the set of available templates.
    pub fn list() -> Vec<&'static Template> {
        vec![&FORTRAN_IV, &COBOL, &JCL_JOB, &ASSEMBLER_H]
    }

    /// Resolve a template by name (case-insensitive).
    pub fn get(name: &str) -> Result<&'static Template> {
        let lname = name.to_ascii_lowercase();
        for tpl in Self::list() {
            if tpl.name.eq_ignore_ascii_case(&lname) {
                return Ok(tpl);
            }
        }
        Err(anyhow!("unknown template '{}'", name))
    }
}

macro_rules! tpl_col {
    ($start:expr, $end:expr, $label:expr) => {
        TemplateColumn {
            range: ColumnRange {
                start: $start,
                end: $end,
            },
            label: $label,
        }
    };
}

static FORTRAN_COLUMNS: &[TemplateColumn] = &[
    tpl_col!(1, 5, "Statement label / comment (C in col 1)"),
    tpl_col!(6, 6, "Continuation (non-blank for continuation)"),
    tpl_col!(7, 72, "Source statement"),
    tpl_col!(73, 80, "Sequence number"),
];

static COBOL_COLUMNS: &[TemplateColumn] = &[
    tpl_col!(1, 6, "Sequence number / identification"),
    tpl_col!(7, 7, "Indicator (e.g., * comment)"),
    tpl_col!(8, 11, "Area A"),
    tpl_col!(12, 72, "Area B"),
    tpl_col!(73, 80, "Identification / sequence"),
];

static JCL_COLUMNS: &[TemplateColumn] = &[
    tpl_col!(1, 2, "Job card '//'"),
    tpl_col!(3, 10, "Job/step name"),
    tpl_col!(11, 15, "Operation (JOB/EXEC/DD)"),
    tpl_col!(16, 71, "Parameters"),
    tpl_col!(72, 72, "Continuation indicator"),
    tpl_col!(73, 80, "Sequence number"),
];

static ASSEMBLER_COLUMNS: &[TemplateColumn] = &[
    tpl_col!(1, 8, "Label"),
    tpl_col!(9, 9, "Continuation"),
    tpl_col!(10, 15, "Operation"),
    tpl_col!(16, 71, "Operands / comments"),
    tpl_col!(72, 72, "Continuation"),
    tpl_col!(73, 80, "Sequence number"),
];

static FORTRAN_IV: Template = Template {
    name: "fortran",
    description: "FORTRAN IV layout with fixed-format areas.",
    columns: FORTRAN_COLUMNS,
    default_type: CardType::Code,
};

static COBOL: Template = Template {
    name: "cobol",
    description: "COBOL columnar layout (sequence, area A/B, comments).",
    columns: COBOL_COLUMNS,
    default_type: CardType::Code,
};

static JCL_JOB: Template = Template {
    name: "jcl",
    description: "IBM JCL job card layout.",
    columns: JCL_COLUMNS,
    default_type: CardType::Jcl,
};

static ASSEMBLER_H: Template = Template {
    name: "assembler",
    description: "IBM System/360 assembler (H) columns.",
    columns: ASSEMBLER_COLUMNS,
    default_type: CardType::Code,
};
