#![feature(rustc_private)]
#![feature(libc)]
extern crate llvm_sys as llvm;
//extern crate rustc;
extern crate libc;
use std::ptr;
use std::ffi;
//use rustc::lib::llvm as rustc_llvm;

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
            let llvm_module = llvm::core::LLVMModuleCreateWithNameInContext(ffi::CString::new("mod1").unwrap().as_ptr(), llvm_context);
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
                    let ty = llvm::core::LLVMIntTypeInContext(ctxt.context, 32);
                    Ok(llvm::core::LLVMConstInt(ty, *i as u64, 0))
                },
                &Expr::AddExpr(ref e1, ref e2) => {
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
        let r = llvm::target::LLVM_InitializeNativeTarget();
        assert_eq!(r, 0);
        llvm::target::LLVM_InitializeNativeAsmPrinter();
        let mut ctxt = Context::new("mod1");

        let print_ty = llvm::core::LLVMIntTypeInContext(ctxt.context, 32);
        let mut pf_type_args_vec = Vec::new(); 

        //"e" is little endian because of x86
        //llvm::core::LLVMSetDataLayout(ctxt.module, "e".as_ptr() as *const i8); //x86_64-linux-gnu
        // pf_type_args_vec.push(llvm::target::LLVMIntPtrTypeInContext(ctxt.context, 
        //                                                             llvm::target::LLVMCreateTargetData("e".as_ptr() as *const i8)));
        
        pf_type_args_vec.push(llvm::core::LLVMPointerType(llvm::core::LLVMIntTypeInContext(ctxt.context, 8),
                                                          0));

        
        let proto = llvm::core::LLVMFunctionType(print_ty, pf_type_args_vec.as_mut_ptr(), 1, 1);
        let print_function = llvm::core::LLVMAddFunction(ctxt.module, 
                                                         ffi::CString::new("printf").unwrap().as_ptr(), 
                                                         proto);
        let exit_ty = llvm::core::LLVMVoidTypeInContext(ctxt.context);
        let mut exit_type_args_vec = Vec::new();
        exit_type_args_vec.push(llvm::core::LLVMIntTypeInContext(ctxt.context, 32));
        let exit_proto = llvm::core::LLVMFunctionType(exit_ty, exit_type_args_vec.as_mut_ptr(), 1, 0);
        let exit_function = llvm::core::LLVMAddFunction(ctxt.module,
                                                      ffi::CString::new("exit").unwrap().as_ptr(),
                                                      exit_proto);
        //main protype
        let ty = llvm::core::LLVMIntTypeInContext(ctxt.context, 32);
        let proto = llvm::core::LLVMFunctionType(ty, ptr::null_mut(), 0, 0);
        let function = llvm::core::LLVMAddFunction(ctxt.module, ffi::CString::new("main").unwrap().as_ptr(), proto);

        
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
                                            ffi::CString::new("entry").unwrap().as_ptr());
        llvm::core::LLVMPositionBuilderAtEnd(ctxt.builder, bb);

        // pub unsafe extern fn LLVMBuildCall(arg1: LLVMBuilderRef, 
        // Fn: LLVMValueRef, 
        // Args: *mut LLVMValueRef, 
        // NumArgs: c_uint, 
        // Name: *const c_char) -> LLVMValueRef
        let gstr = llvm::core::LLVMBuildGlobalStringPtr(ctxt.builder, 
                                                        ffi::CString::new("abhi\n").unwrap().as_ptr(), 
                                                        ffi::CString::new(".str").unwrap().as_ptr());
        let mut pf_args = Vec::new();
        pf_args.push(gstr);
        llvm::core::LLVMBuildCall(ctxt.builder, 
                                  print_function, 
                                  pf_args.as_mut_ptr(), 
                                  1, 
                                  ffi::CString::new("call").unwrap().as_ptr());
        //build return expression
        let mut exit_args = Vec::new();
        exit_args.push(llvm::core::LLVMConstInt(llvm::core::LLVMIntTypeInContext(ctxt.context, 32), 0 as u64, 0));
        llvm::core::LLVMBuildCall(ctxt.builder, 
                                  exit_function, 
                                  exit_args.as_mut_ptr(), 
                                  1, 
                                  ffi::CString::new("call").unwrap().as_ptr());
        
        llvm::core::LLVMBuildRet(ctxt.builder, 
                                 llvm::core::LLVMConstInt(llvm::core::LLVMIntTypeInContext(ctxt.context, 32), 0 as u64, 0));
        

        ctxt.dump();
        let target_ref = llvm::target_machine::LLVMGetFirstTarget();
//Triple: *const c_char, CPU: *const c_char, Features: *const c_char, Level: LLVMCodeGenOptLevel, Reloc: LLVMRelocMode, CodeModel: LLVMCodeModel)
        let target_mc = llvm::target_machine::LLVMCreateTargetMachine(target_ref, 
                                                      llvm::target_machine::LLVMGetDefaultTargetTriple(),
                                                      ffi::CString::new("i386").unwrap().as_ptr(),
                                                      ffi::CString::new("").unwrap().as_ptr(),
                                                      llvm::target_machine::LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
                                                      llvm::target_machine::LLVMRelocMode::LLVMRelocDefault,
                                                      llvm::target_machine::LLVMCodeModel::LLVMCodeModelDefault );
        assert!(target_mc != ptr::null_mut());
        llvm::target_machine::LLVMTargetMachineEmitToFile(target_mc, 
                                                          ctxt.module,
                                                          ffi::CString::new("a.o").unwrap().into_raw(),
                                                          llvm::target_machine::LLVMCodeGenFileType::LLVMObjectFile,
                                                          ffi::CString::new("").unwrap().into_raw() as *mut *mut libc::c_char);
    }
}
