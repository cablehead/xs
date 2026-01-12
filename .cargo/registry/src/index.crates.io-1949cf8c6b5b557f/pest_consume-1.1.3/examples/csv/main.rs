use pest_consume::{match_nodes, Error, Parser};

#[derive(Debug)]
enum CSVField<'a> {
    Number(f64),
    String(&'a str),
}
type CSVRecord<'a> = Vec<CSVField<'a>>;
type CSVFile<'a> = Vec<CSVRecord<'a>>;

type Result<T> = std::result::Result<T, Error<Rule>>;
type Node<'i> = pest_consume::Node<'i, Rule, ()>;

#[derive(Parser)]
#[grammar = "../examples/csv/csv.pest"]
pub struct CSVParser;

#[pest_consume::parser]
impl CSVParser {
    fn EOI(_input: Node) -> Result<()> {
        Ok(())
    }

    fn number(input: Node) -> Result<f64> {
        input
            .as_str()
            .parse::<f64>()
            // `input.error` links the error to the location in the input file where it occurred.
            .map_err(|e| input.error(e))
    }

    fn string(input: Node) -> Result<&str> {
        Ok(input.as_str())
    }

    fn field(input: Node) -> Result<CSVField> {
        Ok(match_nodes!(input.into_children();
            [number(n)] => CSVField::Number(n),
            [string(s)] => CSVField::String(s),
        ))
    }

    fn record(input: Node) -> Result<CSVRecord> {
        Ok(match_nodes!(input.into_children();
            [field(fields)..] => fields.collect(),
        ))
    }

    fn file(input: Node) -> Result<CSVFile> {
        Ok(match_nodes!(input.into_children();
            [record(records).., _] => records.collect(),
        ))
    }
}

fn parse_csv(input_str: &str) -> Result<CSVFile> {
    // Parse the input into `Nodes`
    let inputs = CSVParser::parse(Rule::file, input_str)?;
    // There should be a single root node in the parsed tree
    let input = inputs.single()?;
    // Consume the `Node` recursively into the final value
    CSVParser::file(input)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_csv("-20, 12.5\n42, 0")?;
    let mut sum = 0.;
    for record in parsed {
        for field in record {
            if let CSVField::Number(x) = field {
                sum += x;
            }
        }
    }
    assert_eq!(sum, 34.5);

    let unsuccessful_parse = parse_csv("4, 0.1.1");
    println!("failure: {}", unsuccessful_parse.unwrap_err());

    let successful_parse = parse_csv("-273.15 , ' a string '\n\n42, 0")?;
    println!("success: {:?}", successful_parse);

    Ok(())
}
