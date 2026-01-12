use crate::Parser;
use pest::error::{Error, ErrorVariant};
use pest::iterators::{Pair, Pairs};
use pest::prec_climber::PrecClimber;
use pest::Parser as PestParser;
use pest::{RuleType, Span};

/// A node of the parse tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node<'input, Rule: RuleType, Data> {
    pair: Pair<'input, Rule>,
    user_data: Data,
}

/// Iterator over [`Node`]s. It is created by [`Node::children`] or [`Parser::parse`].
///
/// [`Node`]: struct.Node.html
/// [`Node::children`]: struct.Node.html#method.children
/// [`Parser::parse`]: trait.Parser.html#method.parse
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nodes<'input, Rule: RuleType, Data> {
    pairs: Pairs<'input, Rule>,
    span: Span<'input>,
    user_data: Data,
}

impl<'i, R: RuleType> Node<'i, R, ()> {
    #[doc(hidden)]
    pub fn new(pair: Pair<'i, R>) -> Self {
        Node {
            pair,
            user_data: (),
        }
    }
}
impl<'i, R: RuleType, D> Node<'i, R, D> {
    #[doc(hidden)]
    pub fn new_with_user_data(pair: Pair<'i, R>, user_data: D) -> Self {
        Node { pair, user_data }
    }
    pub fn as_str(&self) -> &'i str {
        self.pair.as_str()
    }
    pub fn as_span(&self) -> Span<'i> {
        self.pair.as_span()
    }
    pub fn as_rule(&self) -> R {
        self.pair.as_rule()
    }
    #[doc(hidden)]
    pub fn as_aliased_rule<C>(&self) -> C::AliasedRule
    where
        C: Parser<Rule = R>,
        <C as Parser>::Parser: PestParser<R>,
    {
        C::rule_alias(self.as_rule())
    }

    /// Returns an iterator over the children of this node
    pub fn into_children(self) -> Nodes<'i, R, D> {
        let span = self.as_span();
        Nodes {
            pairs: self.pair.into_inner(),
            span,
            user_data: self.user_data,
        }
    }
    /// Returns an iterator over the children of this node
    pub fn children(&self) -> Nodes<'i, R, D>
    where
        D: Clone,
    {
        self.clone().into_children()
    }

    /// Create an error that points to the span of the node.
    pub fn error<S: ToString>(&self, message: S) -> Error<R> {
        Error::new_from_span(
            ErrorVariant::CustomError {
                message: message.to_string(),
            },
            self.as_span(),
        )
    }

    pub fn user_data(&self) -> &D {
        &self.user_data
    }
    pub fn into_user_data(self) -> D {
        self.user_data
    }
    pub fn as_pair(&self) -> &Pair<'i, R> {
        &self.pair
    }
    pub fn into_pair(self) -> Pair<'i, R> {
        self.pair
    }
}

impl<'i, R: RuleType, D> Nodes<'i, R, D> {
    /// `input` must be the _original_ input that `pairs` is pointing to.
    #[doc(hidden)]
    pub(crate) fn new(
        input: &'i str,
        pairs: Pairs<'i, R>,
        user_data: D,
    ) -> Self {
        let span = Span::new(input, 0, input.len()).unwrap();
        Nodes {
            pairs,
            span,
            user_data,
        }
    }
    /// Create an error that points to the initial span of the nodes.
    /// Note that this span does not change as the iterator is consumed.
    pub fn error<S: ToString>(&self, message: S) -> Error<R> {
        Error::new_from_span(
            ErrorVariant::CustomError {
                message: message.to_string(),
            },
            self.span.clone(),
        )
    }
    /// Returns the only element if there is only one element.
    pub fn single(mut self) -> Result<Node<'i, R, D>, Error<R>> {
        match (self.pairs.next(), self.pairs.next()) {
            (Some(pair), None) => {
                Ok(Node::new_with_user_data(pair, self.user_data))
            }
            (first, second) => {
                let node_rules: Vec<_> = first
                    .into_iter()
                    .chain(second)
                    .chain(self.pairs)
                    .map(|p| p.as_rule())
                    .collect();

                Err(Error::new_from_span(
                    ErrorVariant::CustomError {
                        message: format!(
                            "Expected a single node, instead got: {:?}",
                            node_rules
                        ),
                    },
                    self.span,
                ))
            }
        }
    }
    #[doc(hidden)]
    pub fn aliased_rules<C>(
        &self,
    ) -> std::iter::Map<
        Pairs<'i, R>,
        impl FnMut(Pair<'i, R>) -> <C as Parser>::AliasedRule,
    >
    where
        C: Parser<Rule = R>,
        <C as Parser>::Parser: PestParser<R>,
    {
        self.pairs.clone().map(|p| C::rule_alias(p.as_rule()))
    }
    /// Construct a node with the provided pair, passing the user data along.
    fn with_pair(&self, pair: Pair<'i, R>) -> Node<'i, R, D>
    where
        D: Clone,
    {
        Node::new_with_user_data(pair, self.user_data.clone())
    }
    /// Performs the precedence climbing algorithm on the nodes.
    pub fn prec_climb<T, E, F1, F2>(
        self,
        climber: &PrecClimber<R>,
        mut primary: F1,
        mut infix: F2,
    ) -> Result<T, E>
    where
        D: Clone,
        F1: FnMut(Node<'i, R, D>) -> Result<T, E>,
        F2: FnMut(T, Node<'i, R, D>, T) -> Result<T, E>,
    {
        let user_data = self.user_data;
        let with_pair = |p| Node::new_with_user_data(p, user_data.clone());
        climber.climb(
            self.pairs,
            |p| primary(with_pair(p)),
            |l, p, r| infix(l?, with_pair(p), r?),
        )
    }

    pub fn user_data(&self) -> &D {
        &self.user_data
    }
    pub fn into_user_data(self) -> D {
        self.user_data
    }
    pub fn as_pairs(&self) -> &Pairs<'i, R> {
        &self.pairs
    }
    pub fn into_pairs(self) -> Pairs<'i, R> {
        self.pairs
    }
}

impl<'i, R, D> Iterator for Nodes<'i, R, D>
where
    R: RuleType,
    D: Clone,
{
    type Item = Node<'i, R, D>;

    fn next(&mut self) -> Option<Self::Item> {
        let child_pair = self.pairs.next()?;
        let child = self.with_pair(child_pair);
        Some(child)
    }
}

impl<'i, R, D> DoubleEndedIterator for Nodes<'i, R, D>
where
    R: RuleType,
    D: Clone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let child_pair = self.pairs.next_back()?;
        let child = self.with_pair(child_pair);
        Some(child)
    }
}

impl<'i, R: RuleType, D> std::fmt::Display for Node<'i, R, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.pair.fmt(f)
    }
}

impl<'i, R: RuleType, D> std::fmt::Display for Nodes<'i, R, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.pairs.fmt(f)
    }
}
