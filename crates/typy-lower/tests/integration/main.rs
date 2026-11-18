//! The integration tests of `typy-lower`, one module per area, built as one test program so the
//! crate links its tests once.

mod foreign;
mod frames;
mod lowering;
mod moves;
mod unions;
