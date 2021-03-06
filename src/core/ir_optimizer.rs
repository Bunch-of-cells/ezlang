use std::collections::HashMap;

use crate::utils::{Instruction, Instructions, Val};

pub fn optimize(code: &Instructions) -> Instructions {
    use crate::check;
    let mut optimized = Instructions::new();
    let mut vars = HashMap::new();
    for (assign, instruction) in &code.0 {
        let optimize = match instruction {
            Instruction::Add(a, Val::Num(0))
            | Instruction::Sub(a, Val::Num(0))
            | Instruction::Mul(a, Val::Num(1))
            | Instruction::Div(a, Val::Num(1)) => {
                let a = if let Val::Index(index, _) = a {
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
            Instruction::Mul(_, Val::Num(0)) => Val::Num(0),
            Instruction::Mul(left, right) if left == right => {
                let left = if let Val::Index(index, _) = left {
                    match dbg!(vars.get(index)) {
                        None => {
                            optimized.push(instruction.clone(), *assign);
                            continue;
                        }
                        Some(val) => val,
                    }
                } else {
                    left
                };
                optimized.push(Instruction::Pow(left.clone(), Val::Num(2)), *assign);
                continue;
            }
            Instruction::Mul(left, Val::Num(-1)) => {
                let left = if let Val::Index(index, _) = left {
                    match dbg!(vars.get(index)) {
                        None => {
                            optimized.push(instruction.clone(), *assign);
                            continue;
                        }
                        Some(val) => val,
                    }
                } else {
                    left
                };
                optimized.push(Instruction::Neg(left.clone()), *assign);
                continue;
            }
            _ => match instruction {
                Instruction::Input => {
                    optimized.push(Instruction::Input, *assign);
                    continue;
                }
                Instruction::Add(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Sub(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Mul(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Div(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Mod(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Eq(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::LAnd(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::LOr(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Lt(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Le(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::LNot(a) => {
                    check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Neg(a) => check!(2 a, optimized, vars, assign, instruction),
                Instruction::Inc(a) => check!(2 a, optimized, vars, assign, instruction),
                Instruction::Dec(a) => check!(2 a, optimized, vars, assign, instruction),
                Instruction::Print(a) => {
                    check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Ascii(a) => {
                    check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Neq(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Pow(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Shl(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Shr(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BAnd(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BOr(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BXor(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::BNot(a) => {
                    check!(2 a, optimized, vars, assign, instruction)
                }
                Instruction::Copy(val) => {
                    check!(2 val, optimized, vars, assign, instruction)
                }
                Instruction::TernaryIf(cond1, then1, else1) => {
                    let cond = if let Val::Index(index, _) = cond1 {
                        match vars.get(index) {
                            Some(Val::Index(..)) | None => None,
                            Some(val) => Some(val.clone()),
                        }
                    } else {
                        Some(cond1.clone())
                    };
                    let then = if let Val::Index(index, _) = then1 {
                        match vars.get(index) {
                            Some(Val::Index(..)) | None => None,
                            Some(val) => Some(val.clone()),
                        }
                    } else {
                        Some(then1.clone())
                    };
                    let else_ = if let Val::Index(index, _) = else1 {
                        match vars.get(index) {
                            Some(Val::Index(..)) | None => None,
                            Some(val) => Some(val.clone()),
                        }
                    } else {
                        Some(else1.clone())
                    };

                    let new_ins = match (cond, then, else_) {
                        (Some(cond), Some(then), None) => {
                            Instruction::TernaryIf(cond, then, else1.clone())
                        }
                        (None, None, Some(else_)) => {
                            Instruction::TernaryIf(cond1.clone(), then1.clone(), else_)
                        }
                        (None, Some(then), None) => {
                            Instruction::TernaryIf(cond1.clone(), then, else1.clone())
                        }
                        (None, Some(then), Some(else_)) => {
                            Instruction::TernaryIf(cond1.clone(), then, else_)
                        }
                        (Some(cond), None, None) => {
                            Instruction::TernaryIf(cond, then1.clone(), else1.clone())
                        }
                        (Some(cond), None, Some(else_)) => {
                            Instruction::TernaryIf(cond, then1.clone(), else_)
                        }
                        (None, None, None) | (Some(_), Some(_), Some(_)) => instruction.clone(),
                    };
                    optimized.push(new_ins.clone(), *assign);
                    continue;
                }
                Instruction::LXor(a, b) => {
                    check!(BINARY2 a, b, optimized, vars, assign, instruction)
                }
                Instruction::Call(f, args) => {
                    let mut new = vec![];
                    for arg in args {
                        if let Val::Index(index, _) = arg {
                            match vars.get(index) {
                                Some(Val::Index(..)) | None => unreachable!(),
                                Some(a) => {
                                    new.push(a.clone());
                                }
                            }
                        }
                    }
                    optimized.push(Instruction::Call(*f, new), *assign);
                    continue;
                }
                _ => todo!(),
            },
        };

        vars.insert(assign.0.unwrap().0, optimize);
    }
    optimized
}
