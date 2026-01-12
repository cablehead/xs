//! ## Precedence climbing
//!
//! `pest_consume` makes it easy to leverage the precedence climbing facilities provided by `pest`.
//! The special `prec_climb` attribute can be placed on a rule method, and that method will automatically
//! use precedence climbing.
//!
//! The attribute needs two arguments: the first is the name of the rule of the non-operator children of this rule.
//! For example, if the rule definition is `expr = { term ~ (operator ~ term)* }`, the non-operator children of the rule
//! `expr` have the rule `term`.
//! The second argument is an expression that references a `pest` [`PrecClimber`].
//! The usual way is to use [`lazy_static`] to define the [`PrecClimber`], to ensure we only construct it once.
//!
//! Finally, the method itself must take three arguments: a [`Node`] containing the matched operator, and a left and a right value.
//! The type of the values, the return type of the rule, and the return type of the children's rule must all be the same.
//!
//! ```ignore
//! lazy_static::lazy_static! {
//!     static ref PRECCLIMBER: PrecClimber<Rule> = PrecClimber::new(
//!         vec![
//!             pcl::Operator::new(Rule::plus, pcl::Assoc::Left),
//!             pcl::Operator::new(Rule::minus, pcl::Assoc::Left),
//!             pcl::Operator::new(Rule::times, pcl::Assoc::Left),
//!         ]
//!     );
//! }
//! ...
//! #[pest_consume::parser]
//! impl MathParser {
//!     ...
//!     #[prec_climb(term, PRECCLIMBER)]
//!     fn expr(l: i64, op: Node, r: i64) -> Result<i64> {
//!         use Rule::*;
//!         match op.as_rule() {
//!             plus => Ok(l + r),
//!             minus => Ok(l - r),
//!             times => Ok(l * r),
//!             r => Err(op.error(format!("Rule {:?} isn't an operator", r)))?,
//!         }
//!     }
//!     ...
//! }
//! ```
//!
//! For a full example using precedence climbing, see [here][prec_climbing-example].
//!
//! [`Node`]: struct.Node.html
//! [`PrecClimber`]: https://docs.rs/pest/2.1.2/pest/prec_climber/struct.PrecClimber.html
//! [`lazy_static`]: https://crates.io/crates/lazy_static
//! [prec_climbing-example]: https://github.com/Nadrieril/pest_consume/tree/master/pest_consume/examples/prec_climbing
