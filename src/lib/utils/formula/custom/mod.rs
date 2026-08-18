
#[cfg(feature = "meval_formula")]
mod meval;
#[cfg(feature = "meval_formula")]
pub use meval::*;

#[cfg(feature = "tabulon_formula")]
mod tabulon;
#[cfg(feature = "tabulon_formula")]
pub use tabulon::*;


// Other options
// !mathexpr
// !mexe
// xprs 
// mexprp
// evalexpr
// evalexpr_jit
// dslcompile