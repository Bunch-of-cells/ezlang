use std::collections::HashMap;

use crate::utils::{Instruction, Instructions, Val};

pub fn optimize(code: &Instructions) -> Instructions {
    let mut optimized = Instructions::new();
    let mut vars = HashMap::new();
    for (assign, instruction) in &code.0 {
        let optimize = match instruction {
            Instruction::Add(a, Val::Num(0) | Val::Bool(false))
            | Instruction::Sub(a, Val::Num(0) | Val::Bool(false))
            | Instruction::Mul(a, Val::Num(1) | Val::Bool(true))
            | Instruction::Div(a, Val::Num(1) | Val::Bool(true)) => {
                println!("{:?}", a);
                let a = if let Val::Index(index) = a {
                    match dbg!(vars.get(index)) {
                        None => {
                            optimized.push(instruction.clone(), *assign);
                            continue;
                        }
                        Some(val) => val,
                    }
                } else {
                    a
                };
                a.clone()
            }
            Instruction::Mul(_, Val::Num(0) | Val::Bool(false)) => Val::Num(0),
            _ => match instruction {
                Instruction::Input => {
                    optimized.push(Instruction::Input, *assign);
                    continue;
                }
                Instruction::Add(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Sub(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Mul(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Div(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Mod(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Eq(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::LAnd(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::LOr(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Gt(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Lt(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Ge(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Le(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::LNot(a) => {
                    super::check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Neg(a) => super::check!(2 a, optimized, vars, assign, instruction),
                Instruction::Inc(a) => super::check!(2 a, optimized, vars, assign, instruction),
                Instruction::Dec(a) => super::check!(2 a, optimized, vars, assign, instruction),
                Instruction::Print(a) => {
                    super::check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Ascii(a) => {
                    super::check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Neq(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Pow(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Shl(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Shr(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BAnd(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BOr(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BXor(a, b) => {
                    super::check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BNot(a) => {
                    super::check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Copy(val) => {
                    super::check!(2 val, optimized, vars, assign, instruction)
                }
                Instruction::If(_cond, _then, _else_) => {
                    // let cond = if let Val::Index(index) = cond {
                    //     match vars.get(index) {
                    //         Some(Val::Index(_)) | None => None,
                    //         Some(val) => Some(val.clone()),
                    //     }
                    // } else {
                    //     Some(cond.clone())
                    // };
                    // let then = if let Val::Index(index) = then {
                    //     match vars.get(index) {
                    //         Some(Val::Index(_)) | None => None,
                    //         Some(val) => Some(val.clone()),
                    //     }
                    // } else {
                    //     Some(then.clone())
                    // };
                    // let else_ = if let Some(val) = else_ {
                    //     if let Val::Index(index) = val {
                    //         match vars.get(index) {
                    //             Some(Val::Index(_)) | None => None,
                    //             Some(val) => Some(val.clone()),
                    //         }
                    //     } else {
                    //         Some(val.clone())
                    //     }
                    // } else {
                    //     None
                    // };

                    // let new_ins = match (cond, then, else_) {
                    //     (Some(cond), Some(then), None) => Instruction::If(cond, then, None),
                    //     (None, None, Some(else_)) => Instruction::If(cond, then, None),
                    //     (None, Some(_), None) => Instruction::If(cond, then, None),
                    //     (None, Some(_), Some(_)) => Instruction::If(cond, then, else_),
                    //     (Some(_), None, None) => Instruction::If(cond, then, else_),
                    //     (Some(_), None, Some(_)) => Instruction::If(cond, then, else_),
                    //     (None, None, None) | (Some(_), Some(_), Some(_)) => instruction.clone(),
                    // };
                    optimized.push(instruction.clone(), *assign);
                    continue;
                }
            },
        };

        vars.insert(assign.unwrap(), optimize);
    }
    optimized
}