# Road to v1.0.0
This library is currently in  a bit of a messy state. I have written a lot of it whilst I was still relatively inexperienced with Rust as a result the code is subpar. Names or variables and functions are unclear, error handling could be better (i am not sure how many of the unwraps used are actually 'safe') and many sections of code are very convoluted and downright unreadable.

Since I people actually use my library, I want to improve it and make it more maintainable and ensure that it works as expected.

- [x] Develop a proper AST
	* Right now `pest` only provides a way to turn a expression 		into a walk-able tree of pairs. This is makes reading the code pretty confusing as every piece deals with both parsing data as well as actually generating the correct output. Building a AST from the given expression and *only then* actually operating on it would help a lot. Separation of concerns and all that.
	* `pest-ast`, `pest_typed_derive` and `pest-consume` might be useful for that.
- [ ] Improve documentation
- [ ] Give good usage examples
   * Include these examples in the documentation and as standalone example files.
- [ ] Logging
   * Debugging the code is not all too easy. Making logging possible at certain parts of it would help.
- [x] Implement change log
   * Follow guidelines from [keepachangelog](https://keepachangelog.com/en/1.1.0/)
- [ ] Error Handling:
   * Error handling can be more detailed and specific, especially in cases where multiple potential causes exist for a failure.
- [x] Get rid of time zones
   * Dealing with time zones is complicated. Handle all dates and times in their naive form and let the caller manage time zones to prevent issues.
- [ ] Performance
   * Improve performance by getting rid of unnecessary clones and redundant parsing.
   * Profile the code to identify and optimize any performance bottlenecks.
- [ ] Refactor for Readability
   * Simplify complex functions and break them into smaller, more manageable pieces.
   * Ensure the code is easy to read and understand.