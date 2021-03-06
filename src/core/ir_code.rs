use std::collections::HashMap;

use crate::utils::{
    Error, ErrorType, Instruction, Instructions, Memory, Node, Token, TokenType, Val, ValNumber,
    ValType, Variables, POINTER_SIZE,
};

/// Generates the Intermediate 3-address code from the AST
pub struct CodeGenerator {
    instructions: Instructions,
    ret: Vec<(usize, usize)>,
    statics: HashMap<String, Val>,
    structs: Vec<ValType>,
}

impl CodeGenerator {
    fn make_instruction(
        &mut self,
        node: &Node,
        vars: &mut Variables,
        memory: &mut Memory,
    ) -> Result<Val, Error> {
        match node {
            Node::Number(num) => {
                if let TokenType::Number(num1) = num.token_type {
                    Ok(Val::Num(num1 as ValNumber))
                } else {
                    unreachable!()
                }
            }

            Node::Boolean(b) => {
                if let TokenType::Keyword(ref boolean) = b.token_type {
                    match boolean.as_ref() {
                        "true" => Ok(Val::Bool(true)),
                        "false" => Ok(Val::Bool(false)),
                        _ => unreachable!(),
                    }
                } else {
                    unreachable!()
                }
            }

            Node::BinaryOp(op, left, right, _) => {
                let left = self.make_instruction(left, vars, memory)?;
                let right = self.make_instruction(right, vars, memory)?;
                let left_type = left.r#type();
                let right_type = right.r#type();
                let t = match left_type.get_result_type(&right_type, op) {
                    Some(result_type) => result_type,
                    None => {
                        return Err(Error::new(
                            ErrorType::TypeError,
                            op.position.clone(),
                            format!(
                                "Cannot {} `{}` to `{}`",
                                op.token_type.get_operation_name(),
                                right_type,
                                left_type
                            ),
                        ))
                    }
                };
                let size = t.get_size();
                let mut mem = memory.allocate(size);
                match op.token_type {
                    TokenType::Ge => {
                        self.instructions.push(
                            Instruction::Lt(left, right),
                            (Some((mem, size)), memory.last_memory_index),
                        );
                        let new_mem = memory.allocate(size);
                        self.instructions.push(
                            Instruction::LNot(Val::Index(mem, ValType::Boolean)),
                            (Some((new_mem, size)), memory.last_memory_index),
                        );
                        mem = new_mem;
                    }
                    TokenType::Gt => {
                        self.instructions.push(
                            Instruction::Le(left, right),
                            (Some((mem, size)), memory.last_memory_index),
                        );
                        let new_mem = memory.allocate(size);
                        self.instructions.push(
                            Instruction::LNot(Val::Index(mem, ValType::Boolean)),
                            (Some((new_mem, size)), memory.last_memory_index),
                        );
                        mem = new_mem;
                    }
                    _ => {
                        self.instructions.push(
                            Instruction::from_token_binary(op)(left, right),
                            (Some((mem, size)), memory.last_memory_index),
                        );
                    }
                }
                Ok(Val::Index(mem, t))
            }

            Node::UnaryOp(op, expr, _) => {
                let expr = self.make_instruction(expr, vars, memory)?;
                let expr_type = expr.r#type();
                let t = match expr_type.get_result_type_unary(op) {
                    Some(result_type) => result_type,
                    None => {
                        return Err(Error::new(
                            ErrorType::TypeError,
                            op.position.clone(),
                            format!(
                                "Cannot apply `{}` to `{}`",
                                op.token_type.get_operation_name(),
                                expr_type,
                            ),
                        ))
                    }
                };
                let size = t.get_size();
                let mem = memory.allocate(size);
                self.instructions.push(
                    Instruction::from_token_unary(op)(expr),
                    (Some((mem, size)), memory.last_memory_index),
                );
                Ok(Val::Index(mem, t))
            }

            Node::VarAssign(var1, expr, _) => {
                if let TokenType::Identifier(ref var) = var1.token_type {
                    match self.make_instruction(expr, vars, memory)? {
                        Val::Index(index, type_ @ ValType::Ref(_)) => {
                            vars.insert(var.clone(), Val::Index(index, type_));
                            Ok(Val::None)
                        }
                        Val::Index(index, type_) => {
                            let size = type_.get_size();
                            let mem = memory.allocate(size);
                            self.instructions.push(
                                Instruction::Copy(Val::Index(index, type_.clone())),
                                (Some((mem, size)), memory.last_memory_index),
                            );
                            vars.insert(var.clone(), Val::Index(mem, type_));
                            Ok(Val::None)
                        }
                        Val::Ref(index, type_) => {
                            vars.insert(
                                var.clone(),
                                Val::Index(index, ValType::Ref(Box::new(type_))),
                            );
                            Ok(Val::None)
                        }
                        val => {
                            let v = val.r#type();
                            let size = val.get_size();
                            let mem = memory.allocate(v.get_size());
                            self.instructions.push(
                                Instruction::Copy(val),
                                (Some((mem, size)), memory.last_memory_index),
                            );
                            vars.insert(var.clone(), Val::Index(mem, v));
                            Ok(Val::None)
                        }
                    }
                } else {
                    unreachable!();
                }
            }

            Node::VarAccess(var, _) => {
                if let TokenType::Identifier(ref var) = var.token_type {
                    Ok(vars.get(var).cloned().unwrap())
                } else {
                    unreachable!();
                }
            }

            Node::VarReassign(var1, expr) => {
                if let TokenType::Identifier(ref var2) = var1.token_type {
                    match self.make_instruction(expr, vars, memory)? {
                        Val::Index(index, type_ @ ValType::Ref(_)) => {
                            let var = vars.get_mut(var2).unwrap();
                            if var.r#type() != type_ {
                                return Err(Error::new(
                                    ErrorType::TypeError,
                                    var1.position.clone(),
                                    format!(
                                        "Variable {} is of type {} but is being assigned to type {}",
                                        var1,
                                        var.r#type(),
                                        type_
                                    ),
                                ));
                            }
                            let size = type_.get_size();
                            self.instructions.push(
                                Instruction::Copy(Val::Index(index, type_)),
                                (Some((index, size)), memory.last_memory_index),
                            );
                            Ok(Val::None)
                        }
                        Val::Index(index, type_) => {
                            let var = vars.get_mut(var2).unwrap();
                            if var.r#type() != type_ {
                                return Err(Error::new(
                                    ErrorType::TypeError,
                                    var1.position.clone(),
                                    format!(
                                        "Variable {} is of type {} but is being assigned to type {}",
                                        var1,
                                        var.r#type(),
                                        type_
                                    ),
                                ));
                            }
                            let size = type_.get_size();
                            if let Val::Index(mem, _) = var {
                                self.instructions.push(
                                    Instruction::Copy(Val::Index(index, type_)),
                                    (Some((*mem, size)), memory.last_memory_index),
                                );
                            } else {
                                unreachable!();
                            }
                            Ok(Val::None)
                        }
                        Val::Ref(index, type_) => {
                            let var = vars.get_mut(var2).unwrap();
                            if !matches!(var.r#type(), ValType::Ref(t) if *t == type_) {
                                return Err(Error::new(
                                    ErrorType::TypeError,
                                    var1.position.clone(),
                                    format!(
                                        "Variable {} is of type {} but is being assigned to type {}",
                                        var1,
                                        var.r#type(),
                                        type_
                                    ),
                                ));
                            }
                            let size = type_.get_size();
                            self.instructions.push(
                                Instruction::Copy(Val::Index(index, type_)),
                                (Some((index, size)), memory.last_memory_index),
                            );
                            Ok(Val::None)
                        }
                        val => {
                            let var = vars.get_mut(var2).unwrap();
                            let size = val.get_size();
                            let val_type = val.r#type();
                            if var.r#type() != val_type {
                                return Err(Error::new(
                                    ErrorType::TypeError,
                                    var1.position.clone(),
                                    format!(
                                        "Variable {} is of type {} but is being assigned to type {}",
                                        val,
                                        var.r#type(),
                                        val_type
                                    ),
                                ));
                            }
                            if let Val::Index(mem, _) = var {
                                self.instructions.push(
                                    Instruction::Copy(val),
                                    (Some((*mem, size)), memory.last_memory_index),
                                );
                            } else {
                                unreachable!();
                            }
                            Ok(Val::None)
                        }
                    }
                } else {
                    unreachable!();
                }
            }

            Node::Statements(statements, ..) => {
                let mut new = memory.clone();
                let mut new_vars = Variables::new_from_parent(vars.clone());
                for statement in statements {
                    self.make_instruction(statement, &mut new_vars, &mut new)?;
                }
                *vars = *new_vars.super_vars.unwrap();
                // if new.last_memory_index > memory.last_memory_index {
                //     self.instructions.push(
                //         Instruction::Clear(memory.last_memory_index, new.last_memory_index),
                //         (None, memory.last_memory_index),
                //     );
                // }
                Ok(Val::None)
            }

            Node::Print(exprs, _) => {
                for expr in exprs {
                    let expr = self.make_instruction(expr, vars, memory)?;
                    if expr.r#type() == ValType::Char {
                        self.instructions
                            .push(Instruction::Ascii(expr), (None, memory.last_memory_index));
                    } else {
                        self.instructions
                            .push(Instruction::Print(expr), (None, memory.last_memory_index));
                    }
                }
                Ok(Val::None)
            }

            Node::Ascii(exprs, _) => {
                for expr in exprs {
                    let expr = self.make_instruction(expr, vars, memory)?;
                    self.instructions
                        .push(Instruction::Ascii(expr), (None, memory.last_memory_index));
                }
                Ok(Val::None)
            }

            Node::Input(..) => {
                let t = ValType::Number;
                let size = t.get_size();
                let mem = memory.allocate(size);
                self.instructions.push(
                    Instruction::Input,
                    (Some((mem, size)), memory.last_memory_index),
                );
                Ok(Val::Index(mem, t))
            }

            Node::If(cond1, then1, else1, _) => {
                let cond = self.make_instruction(cond1, vars, memory)?;
                if cond.r#type() != ValType::Boolean {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        cond1.position(),
                        format!(
                            "Condition in an if statement can only be of type Boolean, and not of type {:?}",
                            cond.r#type()
                        ),
                    ));
                }
                let mem = memory.allocate(2); // Is used while compiling
                self.instructions.push(
                    Instruction::If(cond, mem, else1.is_some()),
                    (None, memory.last_memory_index),
                );
                let then = self.make_instruction(then1, vars, memory)?;
                if then.r#type() != ValType::None {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        then1.position(),
                        format!(
                            "If statement can only return None, and not type {:?}",
                            then.r#type()
                        ),
                    ));
                }

                if let Some(else_) = else1 {
                    self.instructions
                        .push(Instruction::Else(mem), (None, memory.last_memory_index));
                    let e = self.make_instruction(else_, vars, memory)?;
                    if e.r#type() != ValType::None {
                        return Err(Error::new(
                            ErrorType::TypeError,
                            then1.position(),
                            format!(
                                "If statement can only return None, and not type {:?}",
                                then.r#type()
                            ),
                        ));
                    }
                }
                self.instructions.push(
                    Instruction::EndIf(mem, else1.is_some()),
                    (None, memory.last_memory_index),
                );
                Ok(Val::None)
            }

            Node::Ternary(cond1, then1, else_1, ..) => {
                let cond = self.make_instruction(cond1, vars, memory)?;
                if cond.r#type() != ValType::Boolean {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        cond1.position(),
                        format!(
                            "Condition in an if statement can only be of type Boolean, and not of type {:?}",
                            cond.r#type()
                        ),
                    ));
                }

                let then = self.make_instruction(then1, vars, memory)?;
                let then_type = then.r#type();

                let else_ = self.make_instruction(else_1, vars, memory)?;
                if then_type != else_.r#type() {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        else_1.position(),
                        format!("Then and else branch have different types, expected type {:?}, found type {:?}", then_type, else_.r#type()),
                    ));
                }

                let mem = memory.allocate(1);
                self.instructions.push(
                    Instruction::TernaryIf(cond, then, else_),
                    (Some((mem, then_type.get_size())), memory.last_memory_index),
                );
                Ok(Val::Index(mem, then_type))
            }

            Node::None(_) => Ok(Val::None),

            Node::Index(arr1, index1, ..) => {
                let arr = if let TokenType::Identifier(ref var) = arr1.token_type {
                    vars.get(var).cloned().unwrap()
                } else {
                    unreachable!();
                };
                let index = self.make_instruction(index1, vars, memory)?;
                if index.r#type() != ValType::Number {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        index1.position(),
                        format!(
                            "Indexing can only be done with numbers, and not of type {:?}",
                            index.r#type()
                        ),
                    ));
                }
                let arr_type = match &arr {
                    Val::Pointer(_, t) => t.clone(),
                    Val::Index(_, ValType::Pointer(t)) => (**t).clone(),
                    _ => {
                        return Err(Error::new(
                            ErrorType::TypeError,
                            arr1.position.clone(),
                            format!("Cannot index type {:?}", arr.r#type()),
                        ))
                    }
                };
                let t = arr.r#type();
                let size = arr.get_size();
                let mem = memory.allocate(POINTER_SIZE + size);
                self.instructions.push(
                    Instruction::Add(arr, index),
                    (Some((mem, POINTER_SIZE)), memory.last_memory_index),
                );
                self.instructions.push(
                    Instruction::Deref(Val::Index(mem, t)),
                    (Some((mem + POINTER_SIZE, size)), memory.last_memory_index),
                );
                Ok(Val::Index(mem + POINTER_SIZE, arr_type))
            }

            Node::IndexAssign(arr1, index1, assign) => {
                let arr = if let TokenType::Identifier(ref var) = arr1.token_type {
                    vars.get(var).cloned().unwrap()
                } else {
                    unreachable!();
                };
                let index = self.make_instruction(index1, vars, memory)?;
                if index.r#type() != ValType::Number {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        index1.position(),
                        format!(
                            "Indexing can only be done with numbers, and not of type {:?}",
                            index.r#type()
                        ),
                    ));
                }
                let t = arr.r#type();
                let mem = memory.allocate(POINTER_SIZE);
                self.instructions.push(
                    Instruction::Add(arr, index),
                    (Some((mem, POINTER_SIZE)), memory.last_memory_index),
                );
                let assign = self.make_instruction(assign, vars, memory)?;
                self.instructions.push(
                    Instruction::DerefAssign(Val::Index(mem, t), assign),
                    (None, memory.last_memory_index),
                );
                Ok(Val::None)
            }

            Node::Array(elements, t, _) => {
                let type_ = ValType::from_parse_type(t);
                let size = type_.get_size();
                let mem = memory.allocate(size * elements.len());
                let mut current = mem;
                for element1 in elements {
                    let element = self.make_instruction(element1, vars, memory)?;
                    self.instructions.push(
                        Instruction::Copy(element),
                        (Some((current, size)), memory.last_memory_index),
                    );
                    current += size;
                }
                Ok(Val::Pointer(mem, type_))
            }

            Node::Return(val, ..) => {
                let val = self.make_instruction(val, vars, memory)?;
                let &(mem, size) = self.ret.last().unwrap();
                self.instructions.push(
                    Instruction::Copy(val),
                    (Some((mem, size)), memory.last_memory_index),
                );
                Ok(Val::None)
            }

            Node::Ref(val1, ..) => {
                let val = self.make_instruction(val1, vars, memory)?;
                if let Val::Index(n, t) | Val::Pointer(n, t) | Val::Ref(n, t) = val {
                    Ok(Val::Ref(n, t))
                } else {
                    Err(Error::new(
                        ErrorType::TypeError,
                        val1.position(),
                        format!("Cannot reference a {}", val.r#type()),
                    ))
                }
            }

            Node::Deref(val1, ..) => {
                let val = self.make_instruction(val1, vars, memory)?;
                if let ValType::Pointer(t) = val.r#type() {
                    let size = t.get_size();
                    let mem = memory.allocate(size);
                    self.instructions.push(
                        Instruction::Deref(val),
                        (Some((mem, size)), memory.last_memory_index),
                    );
                    Ok(Val::Index(mem, *t))
                } else if let ValType::Ref(t) = val.r#type() {
                    let size = t.get_size();
                    let mem = memory.allocate(size);
                    self.instructions.push(
                        Instruction::DerefRef(val),
                        (Some((mem, size)), memory.last_memory_index),
                    );
                    Ok(Val::Index(mem, *t))
                } else {
                    Err(Error::new(
                        ErrorType::TypeError,
                        val1.position(),
                        format!("Cannot dereference a {}", val.r#type()),
                    ))
                }
            }

            Node::Char(c) => {
                if let TokenType::Char(c) = c.token_type {
                    Ok(Val::Char(c))
                } else {
                    unreachable!()
                }
            }

            Node::DerefAssign(deref, assign, _) => {
                let assign = self.make_instruction(assign, vars, memory)?;
                if let Node::Deref(val1, ..) = &**deref {
                    let val = self.make_instruction(val1, vars, memory)?;
                    if let ValType::Pointer(_) = val.r#type() {
                        self.instructions.push(
                            Instruction::DerefAssign(val, assign),
                            (None, memory.last_memory_index),
                        );
                    } else if let ValType::Ref(_) = val.r#type() {
                        self.instructions.push(
                            Instruction::DerefAssignRef(val, assign),
                            (None, memory.last_memory_index),
                        );
                    } else {
                        println!(
                            "{}",
                            Error::new(
                                ErrorType::TypeError,
                                val1.position(),
                                format!("Cannot dereference a {}", val.r#type()),
                            )
                        );
                    }
                } else {
                    unreachable!()
                };
                Ok(Val::None)
            }

            Node::While(cond1, body1, _) => {
                let mut cond = self.make_instruction(cond1, vars, memory)?;
                if cond.r#type() != ValType::Boolean {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        cond1.position(),
                        format!(
                            "Condition in an if statement can only be of type Boolean, and not of type {:?}",
                            cond.r#type()
                        ),
                    ));
                }

                if let Val::Bool(_) = cond {
                    let size = cond.get_size();
                    let mem = memory.allocate(size);
                    self.instructions.push(
                        Instruction::Copy(cond),
                        (Some((mem, size)), memory.last_memory_index),
                    );
                    cond = Val::Index(mem, ValType::Boolean);
                }

                self.instructions.push(
                    Instruction::While(cond.clone()),
                    (None, memory.last_memory_index),
                );
                let body = self.make_instruction(body1, vars, memory)?;
                if body.r#type() != ValType::None {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        body1.position(),
                        format!(
                            "Body of a while loop can only be of type (None, memory.last_memory_index), and not of type {:?}",
                            body.r#type()
                        ),
                    ));
                }
                if let Val::Index(m, t) = &cond {
                    let cond2 = self.make_instruction(cond1, vars, memory)?;
                    if cond2 != cond {
                        self.instructions.push(
                            Instruction::Copy(cond2),
                            (Some((*m, t.get_size())), memory.last_memory_index),
                        );
                    }
                }
                self.instructions.push(
                    Instruction::EndWhile(cond),
                    (None, memory.last_memory_index),
                );
                Ok(Val::None)
            }

            Node::For(init1, cond1, step1, body1, _) => {
                let init = self.make_instruction(init1, vars, memory)?;
                if init.r#type() != ValType::None {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        init1.position(),
                        format!(
                            "Initialization in a for loop can only be of type None, and not of type {:?}",
                            init.r#type()
                        ),
                    ));
                }

                let mut cond = self.make_instruction(cond1, vars, memory)?;
                if cond.r#type() != ValType::Boolean {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        cond1.position(),
                        format!(
                            "Condition in a for loop can only be of type Boolean, and not of type {:?}",
                            cond.r#type()
                        ),
                    ));
                }

                if let Val::Bool(_) = cond {
                    let size = cond.get_size();
                    let mem = memory.allocate(size);
                    self.instructions.push(
                        Instruction::Copy(cond),
                        (Some((mem, size)), memory.last_memory_index),
                    );
                    cond = Val::Index(mem, ValType::Boolean);
                }

                self.instructions.push(
                    Instruction::While(cond.clone()),
                    (None, memory.last_memory_index),
                );

                let body = self.make_instruction(body1, vars, memory)?;
                if body.r#type() != ValType::None {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        body1.position(),
                        format!(
                            "Body of a for loop can only be of type (None, memory.last_memory_index), and not of type {:?}",
                            body.r#type()
                        ),
                    ));
                }

                let step = self.make_instruction(step1, vars, memory)?;
                if step.r#type() != ValType::None {
                    return Err(Error::new(
                        ErrorType::TypeError,
                        step1.position(),
                        format!(
                            "Step in a for loop can only be of type (None, memory.last_memory_index), and not of type {:?}",
                            step.r#type()
                        ),
                    ));
                }

                if let Val::Index(m, t) = &cond {
                    let cond2 = self.make_instruction(cond1, vars, memory)?;
                    if cond2 != cond {
                        self.instructions.push(
                            Instruction::Copy(cond2),
                            (Some((*m, t.get_size())), memory.last_memory_index),
                        );
                    }
                }

                self.instructions.push(
                    Instruction::EndWhile(cond),
                    (None, memory.last_memory_index),
                );

                Ok(Val::None)
            }

            Node::Expanded(statements, t) => {
                let t = ValType::from_parse_type(t);
                let size = t.get_size();
                let mem = memory.allocate(size);
                self.ret.push((mem, size));

                let mut new_vars = vars.clone();
                let mut new = memory.clone();

                for statement in statements {
                    self.make_instruction(statement, &mut new_vars, &mut new)?;
                }
                self.ret.pop().unwrap();
                // if new.last_memory_index > memory.last_memory_index {
                //     self.instructions.push(
                //         Instruction::Clear(memory.last_memory_index, new.last_memory_index),
                //         (None, memory.last_memory_index),
                //     );
                // }
                Ok(Val::Index(mem, t))
            }

            Node::String(t) => {
                let s = if let TokenType::String(ref s) = t.token_type {
                    s
                } else {
                    unreachable!()
                };
                let mem = memory.allocate(s.len() + 1);
                let mut current_mem = mem;
                for p in s.chars() {
                    self.instructions.push(
                        Instruction::Copy(Val::Char(p as u8)),
                        (Some((current_mem, POINTER_SIZE)), memory.last_memory_index),
                    );
                    current_mem += 1;
                }
                self.instructions.push(
                    Instruction::Copy(Val::Char(0)),
                    (Some((current_mem, POINTER_SIZE)), memory.last_memory_index),
                );
                Ok(Val::Pointer(mem, ValType::Char))
            }

            Node::Converted(n, t) => {
                let val = self.make_instruction(n, vars, memory)?;
                let t = ValType::from_parse_type(t);
                Ok(match val {
                    Val::Num(n) => match t {
                        ValType::Boolean => Val::Bool(n != 0),
                        ValType::Char => Val::Char((n as i16 + 128) as u8),
                        ValType::Number => val,
                        _ => unreachable!(),
                    },
                    Val::Bool(n) => match t {
                        ValType::Boolean => val,
                        ValType::Char => Val::Char(n as u8),
                        ValType::Number => Val::Num(n as i8),
                        _ => unreachable!(),
                    },
                    Val::Char(n) => match t {
                        ValType::Boolean => Val::Bool(n as u8 != 0),
                        ValType::Char => val,
                        ValType::Number => Val::Num((n as i16 - 128) as i8),
                        _ => unreachable!(),
                    },
                    Val::Ref(n, t) => Val::Pointer(n, t),
                    Val::Index(n, _) => Val::Index(n, t),
                    _ => unreachable!("{val} {t}"),
                })
            }

            Node::StaticVar(
                Token {
                    token_type: TokenType::Identifier(ident),
                    ..
                },
                _,
            ) => {
                vars.insert(ident.clone(), self.statics.get(ident).cloned().unwrap());
                Ok(Val::None)
            }

            Node::AttrAccess(_, _, _) => todo!(),

            Node::Struct(_, _, _) => todo!(),

            Node::StructConstructor(_, _, _) => todo!(),

            Node::Pointer(expr, _) => {
                let val = self.make_instruction(expr, vars, memory)?;
                if let Val::Index(n, t) | Val::Pointer(n, t) | Val::Ref(n, t) = val {
                    Ok(Val::Pointer(n, t))
                } else {
                    Err(Error::new(
                        ErrorType::TypeError,
                        expr.position(),
                        format!("Cannot take the address of {}", val.r#type()),
                    ))
                }
            }

            _ => unreachable!(),
        }
    }

    fn make_static(
        &mut self,
        node: Node,
        vars: &mut Variables,
        memory: &mut Memory,
    ) -> Result<Val, Error> {
        match node {
            Node::StaticVar(
                Token {
                    token_type: TokenType::Identifier(ident),
                    ..
                },
                expr,
            ) => {
                match self.make_instruction(&expr, vars, memory)? {
                    Val::Index(_, ValType::Ref(_)) | Val::Ref(..) => (),
                    Val::Index(index, type_) => {
                        let size = type_.get_size();
                        let mem = memory.allocate(size);
                        self.instructions.push(
                            Instruction::Copy(Val::Index(index, type_.clone())),
                            (Some((mem, size)), memory.last_memory_index),
                        );
                        self.statics.insert(ident, Val::Index(mem, type_));
                    }
                    val => {
                        let v = val.r#type();
                        let size = val.get_size();
                        let mem = memory.allocate(v.get_size());
                        self.instructions.push(
                            Instruction::Copy(val),
                            (Some((mem, size)), memory.last_memory_index),
                        );
                        self.statics.insert(ident, Val::Index(mem, v));
                    }
                }
                Ok(Val::None)
            }
            _ => unreachable!(),
        }
    }
}

/// Generates and returns the Intermediate Representation of the AST
pub fn generate_code(
    ast: Node,
    statics: Vec<Node>,
    structs: Vec<Node>,
) -> Result<Instructions, Error> {
    let mut structs_valtype = vec![];
    for struct_ in structs {
        structs_valtype.push(ValType::from_parse_type(
            &struct_.struct_from_def().unwrap(),
        ));
    }

    let mut obj = CodeGenerator {
        instructions: Instructions::new(),
        ret: vec![],
        statics: HashMap::new(),
        structs: structs_valtype,
    };
    let mut vars = Variables::new();
    let mut memory = Memory::new();
    for node in statics {
        obj.make_static(node, &mut vars, &mut memory)?;
    }

    obj.make_instruction(&ast, &mut vars, &mut memory)?;
    Ok(obj.instructions)
}
