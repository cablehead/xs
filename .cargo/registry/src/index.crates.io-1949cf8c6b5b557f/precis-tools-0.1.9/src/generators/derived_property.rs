use crate::error::Error;
use crate::generators::CodeGen;
use std::fs::File;
use std::io::Write;

/// Generates the derived property `enum` with the
/// values described in the PRECIS
/// [Code Point Properties](https://datatracker.ietf.org/doc/html/rfc8264#section-8)
/// section.
/// # Example:
/// ```rust
/// pub enum DerivedPropertyValue {
///    PValid,
///    SpecClassPval,
///    SpecClassDis,
///    ContextJ,
///    ContextO,
///    Disallowed,
///    Unassigned,
/// }
/// ```
pub struct DerivedPropertyValueGen {}

impl DerivedPropertyValueGen {
    /// Creates a new generator for the `DerivedPropertyValue` `enum`.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for DerivedPropertyValueGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGen for DerivedPropertyValueGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        writeln!(file, "/// Derived property value")?;
        writeln!(file, "/// # Notes")?;
        writeln!(
            file,
            "/// * **SpecClassPVal** maps to those code points that are allowed"
        )?;
        writeln!(
            file,
            "///   to be used in specific string classes such as [`IdentifierClass`]"
        )?;
        writeln!(
            file,
            "///   and [`FreeformClass`]. PRECIS framework defines two allowed"
        )?;
        writeln!(
            file,
            "///   values for above classes (ID_PVAL adn FREE_PVAL). In practice,"
        )?;
        writeln!(
            file,
            "///   the derived property ID_PVAL is not used in this specification,"
        )?;
        writeln!(
            file,
            "///   because every ID_PVAL code point is PVALID, so only FREE_PVAL"
        )?;
        writeln!(file, "///   is actually mapped to SpecClassPVal.")?;
        writeln!(
            file,
            "/// * **SpecClassDis** maps to those code points that are not to be"
        )?;
        writeln!(
            file,
            "///   included in one of the string classes but that might be permitted"
        )?;
        writeln!(
            file,
            "///   in others. PRECIS framework defines \"FREE_DIS\" for the"
        )?;
        writeln!(
            file,
            "///   [`FreeformClass`] and \"ID_DIS\" for the [`IdentifierClass`]."
        )?;
        writeln!(
            file,
            "///   In practice, the derived property FREE_DIS is not used in this"
        )?;
        writeln!(
            file,
            "///   specification, because every FREE_DIS code point is DISALLOWED,"
        )?;
        writeln!(file, "///   so only ID_DIS is mapped to SpecClassDis.")?;
        writeln!(
            file,
            "///   Both SpecClassPVal and SpecClassDis values are used to ease"
        )?;
        writeln!(
            file,
            "///   extension if more classes are added beyond [`IdentifierClass`]"
        )?;
        writeln!(file, "///   and [`FreeformClass`] in the future.")?;
        writeln!(file, "#[derive(Clone, Copy, Debug, PartialEq, Eq)]")?;
        writeln!(file, "pub enum DerivedPropertyValue {{")?;
        writeln!(file, "\t/// Value assigned to all those code points that are allowed to be used in any PRECIS string class.")?;
        writeln!(file, "\tPValid,")?;
        writeln!(file, "\t/// Value assigned to all those code points that are allowed to be used in an specific PRECIS string class.")?;
        writeln!(file, "\tSpecClassPval,")?;
        writeln!(file, "\t/// Value assigned to all those code points that are disallowed by a specific PRECIS string class.")?;
        writeln!(file, "\tSpecClassDis,")?;
        writeln!(
            file,
            "\t/// Contextual rule required for Join_controls Unicode codepoints."
        )?;
        writeln!(file, "\tContextJ,")?;
        writeln!(
            file,
            "\t/// Contextual rule required for Others Unicode codepoints."
        )?;
        writeln!(file, "\tContextO,")?;
        writeln!(
            file,
            "\t/// Those code points that are not permitted in any PRECIS string class."
        )?;
        writeln!(file, "\tDisallowed,")?;
        writeln!(
            file,
            "\t/// Those code points that are not designated in the Unicode Standard."
        )?;
        writeln!(file, "\tUnassigned,")?;
        writeln!(file, "}}")?;

        writeln!(file)?;
        writeln!(file, "impl std::fmt::Display for DerivedPropertyValue {{")?;
        writeln!(
            file,
            "\tfn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{"
        )?;
        writeln!(file, "\t\tmatch self {{")?;
        writeln!(
            file,
            "\t\t\tDerivedPropertyValue::PValid => writeln!(f, \"PValid\"),"
        )?;
        writeln!(
            file,
            "\t\t\tDerivedPropertyValue::SpecClassPval => writeln!(f, \"SpecClassPval\"),"
        )?;
        writeln!(
            file,
            "\t\t\tDerivedPropertyValue::SpecClassDis => writeln!(f, \"SpecClassDis\"),"
        )?;
        writeln!(
            file,
            "\t\t\tDerivedPropertyValue::ContextJ => writeln!(f, \"ContextJ\"),"
        )?;
        writeln!(
            file,
            "\t\t\tDerivedPropertyValue::ContextO => writeln!(f, \"ContextO\"),"
        )?;
        writeln!(
            file,
            "\t\t\tDerivedPropertyValue::Disallowed => writeln!(f, \"Disallowed\"),"
        )?;
        writeln!(
            file,
            "\t\t\tDerivedPropertyValue::Unassigned => writeln!(f, \"Unassigned\"),"
        )?;
        writeln!(file, "\t\t}}")?;

        writeln!(file, "\t}}")?;
        writeln!(file, "}}")?;

        Ok(writeln!(file)?)
    }
}
