# Payments Engine

This is a simple payments engine capable of determining account balances from transactions described
in a `.csv` file. To run the payments engine, use: 
```
cargo run -- transactions.csv > accounts.csv
```

If testing performance, please instead run with
```
cargo run --release -- transactions.csv > accounts.csv
```

## Correctness
This payments engine uses unit tests run on sample data to test for correctness. To run these
tests, use:
```
cargo test
```
In addition to unit testing, this payments engine makes heavy use of Rust's type system to minimize
room for programmer error. Amounts are represented as fixed-precision values, while transactions and
accounts are (de)serialized to/from their respective `structs`. 

## Safety and Robustness
This payments engine uses no unsafe code, and most errors are recoverable. When a recoverable error
occurs, the transaction in question is ignored and an error message is printed to stderr. However,
there are still a few failure modes. If an invalid argument is provided, for example, the process
will exit and an error will be printed to stderr. Similarly, if headers cannot be read from the
provided csv, the process will exit and an error will be printed to stderr. 

## Efficiency
The dataset is read line-by-line, reducing memory usage. This necessitates re-reading old
transactions from disk when processing disputes, but this is only a minor inconvenience given the
(theoretical) infrequency of disputes.

If this code were to be bundled into a server, some improvements could be made. It may make sense to
process datasets on a threadpool using a crate like rayon. Tasks might then be divided
between threads by client id to avoid locking on shared resources.

## Maintainability
Maintainability was a priority during the development of this payments engine. The code is well
commented, errors never pass by silently, and the unit test coverage is respectable. The code has
also been divided into small functions and modules, enabling future developers to focus only on the
components they are interested in. Finally, clippy and rustfmt were used extensively to ensure the
code follows the commonly accepted best-practices for formatting and style. 

## Future Improvements
Time permitting, there are many improvements that could be made to this project. The code could be
refactored into a more object-oriented structure with an `engine` object containing member functions
for `deposit()`, `withdrawal()`, and the other three transaction types. It may also be possible to
reduce the impact of storage latency by reading lines of the csv in small batches, rather than one
at a time.

*This project is my solution to the programming test included in an unnamed company's hiring process*
