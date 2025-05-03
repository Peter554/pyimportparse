# pyimportparse

A rust crate to parse python imports (while ignoring the rest of the code).

Motivation:
* For fun/interest.
* Lack of alternative tool:
  * Presently [RustPython/Parser](https://github.com/RustPython/Parser) does not support Python 3.12+.
  * Presently [Ruff Python Parser](https://github.com/astral-sh/ruff/tree/main/crates/ruff_python_parser) is not published publicly as a crate.

Use with care. I've run the parser over the Django codebase and the results suggest that 
all imports were successfully parsed. However, there could still be cases where the parser does
not behave correctly. 

```rust
use pyimportparse::{parse_imports, Import};

let code = r#"
import a
from b import c
from .d import (e, f)
from ..g import *

if TYPE_CHECKING:
    import h

def foo():
    import i
"#;
    
let imports = parse_imports(&code).unwrap();
    
assert_eq!(vec![
    // (imported_object, line_number, typechecking_only)
    Import::new("a".to_owned(), 2, false),
    Import::new("b.c".to_owned(), 3, false),
    Import::new(".d.e".to_owned(), 4, false),
    Import::new(".d.f".to_owned(), 4, false),
    Import::new("..g.*".to_owned(), 5, false),
    Import::new("h".to_owned(), 8, true),
    Import::new("i".to_owned(), 11, false),
], imports);




```