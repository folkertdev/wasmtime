//! Implementation of the interpreter loop for Pulley with a simple `match`
//! statement.
//!
//! This module is notably in contrast to the `tail_loop.rs` which implements
//! the interpreter loop with tail calls. It's predicted that tail calls are a
//! more performant solution but that's also not available on stable Rust today,
//! so this module instead compiles on stable Rust.
//!
//! This interpreter loop is a simple `loop` with a "moral `match`" despite not
//! actually having one here. The `Decoder` API is used to dispatch to the
//! `OpVisitor` trait implementation on `Interpreter<'_>`. The literal `match`
//! is embedded within the `Decoder::decode_one` function.
//!
//! Note that as of the time of this writing there hasn't been much performance
//! analysis of this loop just yet. It's probably too simple to compile well and
//! will probably need tweaks to make it more performant.

use super::*;
use crate::Opcode;

impl Interpreter<'_> {
    #[unsafe(export_name = "my_very_unique_function")]
    pub fn run(self) -> Done {
        let mut visitor = debug::Debug(self);
        let visitor = &mut visitor;

        let Ok(byte) = u8::decode(visitor.bytecode());
        let Ok(mut opcode) =
            Opcode::new(byte).ok_or_else(|| visitor.bytecode().invalid_opcode(byte));
        loop {
            macro_rules! dispatch {
                (
                    $(
                        $( #[$attr:meta] )*
                            $snake_name:ident = $name:ident $( {
                            $(
                                $( #[$field_attr:meta] )*
                                $field:ident : $field_ty:ty
                            ),*
                        } )? ;
                    )*
                ) => {
                    #[cfg_attr(pulley_indirectbr, indirect_branch)]
                    match opcode {
                        $(
                            Opcode::$name => {
                                $(
                                    $(
                                        let Ok($field) = <$field_ty>::decode(
                                            visitor.bytecode(),
                                        );
                                    )*
                                )?

                                let ret = visitor.$snake_name($( $( $field ),* )?);
                                visitor.after_visit();
                                match ret {
                                    ControlFlow::Continue(()) => {
                                        let Ok(byte) = u8::decode(visitor.bytecode());
                                        let Ok(o2) =
                                            Opcode::new(byte).ok_or_else(|| visitor.bytecode().invalid_opcode(byte));
                                        opcode = o2;
                                    }
                                    ControlFlow::Break(done) => break done,
                                }
                            },
                        )*
                        Opcode::ExtendedOp => {
                            let Ok(r) = crate::decode::decode_one_extended(visitor);
                            match r {
                                ControlFlow::Continue(()) => {}
                                ControlFlow::Break(done) => break done,
                            }
                            let Ok(byte) = u8::decode(visitor.bytecode());
                            let Ok(o2) =
                                Opcode::new(byte).ok_or_else(|| visitor.bytecode().invalid_opcode(byte));
                            opcode = o2;
                        }
                    }
                };
            }
            for_each_op!(dispatch);
        }
    }
}
