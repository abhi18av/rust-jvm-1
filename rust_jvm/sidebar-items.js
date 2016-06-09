initSidebarItems({"macro":[["c!","Converts a backtracking-controllable parser to a nom parser that will have the correct backtracking behavior."],["catching!",""],["catching_at!",""],["custom_error!","Convenience for creating a custom nom error `nom::Err::Code(nom::ErrorKind::Custom($err))`."],["cut!","`cut!(nom::ErrorKind<E>, I -> nom::IResult<I, O, E>) => Err<_, nom::Err<I, E>> OR nom::IResult::Done<I, O, E> OR IResult::Incomplete<_>` Prevents backtracking out of the specified nom parser."],["done!","`done!(O) => nom::IResult::Done<I, O>` wraps the specified expression in `nom::IResult::Done`."],["eprintln!","Prints to standard error."],["n!","Declares a parser (with a body of type `nom::IResult`) that can be controlled for backtracking by using `c!` and `cut!`. The return type is `std::result::Result<nom::IResult, nom::Err<I, E>>`. If the parser returns `std::result::Result::Ok`, then backtracking occurs. If the parser returns `std::result::Result::Err`, then backtracking does not occur."],["p_add_error!","Adds a custom error if the child nom parser fails."],["p_cut!","`p_cut!(E, I -> nom::IResult<I, O, E>) => Err<_, nom::Err<I, E>> OR IResult::Done<I, O> OR IResult::Incomplete<_>` Like `cut!`, but with a custom error type."],["p_fail!","Returns a custom error for a backtracking-controllable parser."],["p_named!","Declares a parser (with a body of type `std::result::Result<nom::IResult, nom::Err<I, E>>`) that can be controlled for backtracking by using `c!` and `cut!`. The return type is `std::result::Result<nom::IResult, nom::Err<I, E>>`. If the parser returns `std::result::Result::Ok`, then backtracking occurs. If the parser returns `std::result::Result::Err`, then backtracking does not occur."],["p_nom_error!","Returns a custom error for a nom parser."],["p_try!","Binds monadically without backtracking a backtracking-controllable parser."],["p_unwrap!","Binds monadically without backtracking the result of a backtracking-controllable parser."],["p_wrap_nom!","Wraps a nom parser (returning `nom::IResult`) to produce a parser that does not backtrack on error."],["with_warn!",""],["wrap_nom!","Wraps the result of a nom parser (`nom::IResult`) to be non-backtracking."]],"mod":[["logging",""],["model","Structures for the Java SE 8 JVM class file format."],["parser","Contains a parser for a Java class file."],["util",""],["vm","The public interface for the Java virtual machine."]]});