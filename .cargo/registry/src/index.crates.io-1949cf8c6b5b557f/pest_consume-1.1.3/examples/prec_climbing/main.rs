use pest::prec_climber as pcl;
use pest::prec_climber::PrecClimber;
use pest_consume::{match_nodes, Error, Parser};

type Result<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

#[derive(Parser)]
#[grammar = "../examples/prec_climbing/grammar.pest"]
struct MathParser;

lazy_static::lazy_static! {
    static ref PRECCLIMBER: PrecClimber<Rule> = PrecClimber::new(
        vec![
            pcl::Operator::new(Rule::plus, pcl::Assoc::Left),
            pcl::Operator::new(Rule::minus, pcl::Assoc::Left),
            pcl::Operator::new(Rule::times, pcl::Assoc::Left),
        ]
    );
}

#[pest_consume::parser]
impl MathParser {
    fn EOI(_input: Node) -> Result<()> {
        Ok(())
    }

    fn number(input: Node) -> Result<i64> {
        input.as_str().trim().parse().map_err(|e| input.error(e))
    }

    #[prec_climb(term, PRECCLIMBER)]
    fn expr(l: i64, op: Node, r: i64) -> Result<i64> {
        use Rule::*;
        match op.as_rule() {
            plus => Ok(l + r),
            minus => Ok(l - r),
            times => Ok(l * r),
            r => Err(op.error(format!("Rule {:?} isn't an operator", r)))?,
        }
    }

    fn term(input: Node) -> Result<i64> {
        Ok(match_nodes!(input.into_children();
            [number(n)] => n,
            [expr(n)] => n,
        ))
    }

    fn calculation(input: Node) -> Result<i64> {
        Ok(match_nodes!(input.into_children();
            [expr(e), EOI(_)] => e,
        ))
    }
}

fn parse_math(input_str: &str) -> Result<i64> {
    // Parse the input into `Nodes`
    let inputs = MathParser::parse(Rule::calculation, input_str)?;
    // There should be a single root node in the parsed tree
    let input = inputs.single()?;
    // Consume the `Node` recursively into the final value
    MathParser::calculation(input)
}

fn main() -> Result<()> {
    assert_eq!(parse_math("1 + 1")?, 2);
    assert_eq!(parse_math("1 + 2 * 3")?, 7);
    assert_eq!(parse_math("1 - 2 + 3")?, 2);
    assert_eq!(parse_math("1 - 2 * 3")?, -5);
    assert_eq!(parse_math("1 - 2 - 3")?, -4);

    Ok(())
}
