//copy the IR in a sample.ll file
//run llc sample.ll -mtriple=x86_64-w64-mingw32 -o sample.s
//open mingw shell and type run as sample.s
//link: ld a.out -L "C:\MinGW\x86_64-w64-mingw32\lib" -lmsvcrt -o ldtest.exe

#![feature(rustc_private)]

extern crate rustc;

use rustc::lib::llvm;
use std::collections::{HashMap};

use std::ops::Deref;

#[derive(Debug, PartialEq, Clone)] //this is necessary so that TType can be used in assert, compared, cloned
pub struct B<T>{
    ptr : Box<T>
}

//acts like a constructor
pub fn B<T>(value : T)->B<T>{
    B {ptr : Box::new(value)}
}

impl<T> Deref for B<T>{ //allows & to be used for B<T>
    type Target = T;

    fn deref<'a>(&'a self) -> &'a T{
        &self.ptr
    }
}


type IRBuildingResult = Result<llvm::ValueRef, String>;

struct Context{
    context : llvm::ContextRef,
    module : llvm::ModuleRef,
    builder : llvm::BuilderRef,
    named_values : HashMap<String, llvm::ValueRef>
}

pub enum Expr{
   //id
   IdExpr(String),
   //nil
   NilExpr,
   //FIXME is this needed?
   LitExpr,
   //stringLit
   StringExpr(String),
   //break
   BreakExpr,
   //id ( exp*, )
   CallExpr(String, Option<B<Expr>>),
   //intLit
   NumExpr(i32),
   AddExpr(B<Expr>, B<Expr>),
}

impl Context{
    fn new(module_name : &str) -> Self{
        unsafe{
            let llvm_context =  llvm::LLVMContextCreate();
            let llvm_module = llvm::LLVMModuleCreateWithNameInContext(module_name.as_ptr() as *const i8, llvm_context);
            let builder = llvm::LLVMCreateBuilderInContext(llvm_context);
            let named_values = HashMap::new();

            Context {
                context : llvm_context,
                module : llvm_module,
                builder : builder,
                named_values : named_values
            }
        }
    }

    fn dump(&self){
        unsafe{
            llvm::LLVMDumpModule(self.module);
        }
    }
}

impl Drop for Context{
    fn drop(&mut self){
        unsafe{
            llvm::LLVMDisposeBuilder(self.builder);
            llvm::LLVMDisposeModule(self.module);
            llvm::LLVMContextDispose(self.context);
        }
    }
}


trait IRBuilder{
    fn codegen(&self, ctxt : &mut Context) -> IRBuildingResult;
}

impl IRBuilder for Expr{
    fn codegen(&self, ctxt : &mut Context) -> IRBuildingResult{
        unsafe{
            match self{
                &Expr::NumExpr(ref i) => {
                    let ty = llvm::LLVMDoubleTypeInContext(ctxt.context);
                    Ok(llvm::LLVMConstReal(ty, *i as f64))
                },
                &Expr::AddExpr(ref e1, ref e2) => {
                    println!("add called");
                    let ev1 = try!(e1.codegen(ctxt));
                    let ev2 = try!(e2.codegen(ctxt));
                    Ok(llvm::LLVMBuildFAdd(ctxt.builder, ev1, ev2, "add_tmp".as_ptr() as *const i8))
                },
                _ => Err("error".to_string())
            }
        }
    }
}

fn main(){
    unsafe{

        let mut ctxt = Context::new("mod1");

        let ty = llvm::LLVMDoubleTypeInContext(ctxt.context);
        let proto = llvm::LLVMFunctionType(ty, Vec::new().as_ptr(), 0, false as u32);
        let function = llvm::LLVMAddFunction(ctxt.module, "foo".as_ptr() as *const i8, proto);

        let print_ty = llvm::LLVMIntTypeInContext(ctxt.context);
        let pf_type_args_vec = Vec::new();
        pf_type_args_vec.push(llvm::LLVMIntPtrTypeInContext(ctxt.context));
        let proto = llvm::LLVMFunctionType(print_ty, pf_type_args_vec.as_ptr(), 1, false as u32);


        let n1 = Expr::NumExpr(32);
        let n2 = Expr::NumExpr(32);
        let n = Expr::AddExpr(B(n1), B(n2));
        let body = n.codegen(&mut ctxt);
        let unwrapped_body = match body{
            Ok(value) => value,
            _ => panic!("invalid")
        };

        let bb = llvm::LLVMAppendBasicBlockInContext(ctxt.context,
                                            function,
                                            "entry".as_ptr() as *const i8);
        llvm::LLVMPositionBuilderAtEnd(ctxt.builder, bb);
        llvm::LLVMBuildRet(ctxt.builder, unwrapped_body);

        ctxt.dump();
    }
}
