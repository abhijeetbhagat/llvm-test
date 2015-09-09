#![feature(rustc_private)]
#![feature(libc)]
extern crate llvm_sys as llvm;
extern crate rustc;
extern crate libc;
use std::ptr;

use rustc::lib::llvm as rustc_llvm;

use std::collections::{HashMap};
use std::mem;

struct Context{
    context : llvm::prelude::LLVMContextRef,
    module : llvm::prelude::LLVMModuleRef,
    builder : llvm::prelude::LLVMBuilderRef,
    named_values : HashMap<String, llvm::prelude::LLVMValueRef>
}

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

type IRBuildingResult = Result<llvm::prelude::LLVMValueRef, String>;

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
            let llvm_context =  llvm::core::LLVMContextCreate();
            let llvm_module = llvm::core::LLVMModuleCreateWithNameInContext(module_name.as_ptr() as *const i8, llvm_context);
            let builder = llvm::core::LLVMCreateBuilderInContext(llvm_context);
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
            llvm::core::LLVMDumpModule(self.module);
        }
    }
}

impl Drop for Context{
    fn drop(&mut self){
        unsafe{
            llvm::core::LLVMDisposeBuilder(self.builder);
            llvm::core::LLVMDisposeModule(self.module);
            llvm::core::LLVMContextDispose(self.context);
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
                    let ty = llvm::core::LLVMDoubleTypeInContext(ctxt.context);
                    Ok(llvm::core::LLVMConstReal(ty, *i as f64))
                },
                &Expr::AddExpr(ref e1, ref e2) => {
                    println!("add called");
                    let ev1 = try!(e1.codegen(ctxt));
                    let ev2 = try!(e2.codegen(ctxt));
                    Ok(llvm::core::LLVMBuildFAdd(ctxt.builder, ev1, ev2, "add_tmp".as_ptr() as *const i8))
                },
                _ => Err("error".to_string())
            }
        }
    }
}

fn main(){
    unsafe{

        let mut ctxt = Context::new("mod1");

        //printf prototype
        // let print_ty = llvm::core::LLVMIntTypeInContext(ctxt.context, 32);
        // let mut pf_type_args_vec = Vec::new(); 
        // pf_type_args_vec.push(llvm::target::LLVMIntPtrTypeInContext(ctxt.context, ptr::null_mut()));
        // let proto = llvm::core::LLVMFunctionType(print_ty, pf_type_args_vec.as_mut_ptr(), 1, 1);
        // let print_function = llvm::core::LLVMAddFunction(ctxt.module, 
        //                                                  "printf".as_ptr() as *const i8, 
        //                                                  proto);

        let print_ty = llvm::core::LLVMIntTypeInContext(ctxt.context, 32);
        let mut pf_type_args_vec = Vec::new(); 
        let p = libc::malloc(mem::size_of::<llvm::target::LLVMTargetDataRef>() as libc::size_t) 
            as *mut llvm::target::LLVMTargetDataRef;
        pf_type_args_vec.push(llvm::target::LLVMIntPtrTypeInContext(ctxt.context, 
                                                                    llvm::target::LLVMCreateTargetData("e".as_ptr() as *const i8)));
        //pf_type_args_vec.push(llvm::core::LLVMIntTypeInContext(ctxt.context, 32));
        let proto = llvm::core::LLVMFunctionType(print_ty, pf_type_args_vec.as_mut_ptr(), 1, 1);
        let print_function = llvm::core::LLVMAddFunction(ctxt.module, 
                                                         "printf".as_ptr() as *const i8, 
                                                         proto);

        //main protype
        let ty = llvm::core::LLVMDoubleTypeInContext(ctxt.context);
        let proto = llvm::core::LLVMFunctionType(ty, ptr::null_mut(), 0, 0);
        let function = llvm::core::LLVMAddFunction(ctxt.module, "foo".as_ptr() as *const i8, proto);

        
        let n1 = Expr::NumExpr(32);
        let n2 = Expr::NumExpr(32);
        let n = Expr::AddExpr(B(n1), B(n2));
        let body = n.codegen(&mut ctxt);
        let unwrapped_body = match body{
            Ok(value) => value,
            _ => panic!("invalid")
        };

        let bb = llvm::core::LLVMAppendBasicBlockInContext(ctxt.context,
                                            function,
                                            "entry".as_ptr() as *const i8);
        llvm::core::LLVMPositionBuilderAtEnd(ctxt.builder, bb);

        //preparing printf call
        //
        // let gstr = llvm::core::LLVMBuildGlobalStringPtr(ctxt.builder, 
        //                                                 "abhi".as_ptr() as *const i8, 
        //                                                 ".str".as_ptr() as *const i8);

        /*
        pub fn LLVMBuildCall(arg1: LLVMBuilderRef, Fn: LLVMValueRef,
                         Args: *mut LLVMValueRef, NumArgs: ::libc::c_uint,
                         Name: *const ::libc::c_char) -> LLVMValueRef;
         */
        // let mut pf_args = Vec::new();
        // pf_args.push(gstr);
        // let print_call = llvm::core::LLVMBuildCall(ctxt.builder,
        //                                            print_function,
        //                                            pf_args.as_mut_ptr(),
        //                                            1,
        //                                            "name".as_ptr() as *const i8);

        llvm::core::LLVMBuildRet(ctxt.builder, unwrapped_body);

        ctxt.dump();
    }
}
