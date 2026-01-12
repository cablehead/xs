/*!
Types for parsing files in the `extracted` subdirectory of the Unicode
Character Database download.

These are placed here, rather than at the top level, to help keep the number of
types in any given module managable.
*/

pub use self::{
    derived_bidi_class::DerivedBidiClass,
    derived_binary_properties::DerivedBinaryProperties,
    derived_combining_class::DerivedCombiningClass,
    derived_decomposition_type::DerivedDecompositionType,
    derived_east_asian_width::DerivedEastAsianWidth,
    derived_general_category::DerivedGeneralCategory,
    derived_joining_group::DerivedJoiningGroup,
    derived_joining_type::DerivedJoiningType,
    derived_line_break::DerivedLineBreak, derived_name::DerivedName,
    derived_numeric_type::DerivedNumericType,
    derived_numeric_values::DerivedNumericValues,
};

mod derived_bidi_class;
mod derived_binary_properties;
mod derived_combining_class;
mod derived_decomposition_type;
mod derived_east_asian_width;
mod derived_general_category;
mod derived_joining_group;
mod derived_joining_type;
mod derived_line_break;
mod derived_name;
mod derived_numeric_type;
mod derived_numeric_values;
