use crate::LogLevel;
use core::mem;
use x86_64::structures::idt::{DivergingHandlerFuncWithErrCode, InterruptDescriptorTable, PageFaultErrorCode};
use x86_64::structures::idt::InterruptStackFrame;

use core::fmt::Write;
use crate::{klog, logging};

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init_idt() {
    #[allow(static_mut_refs)]
    unsafe {
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.page_fault.set_handler_fn(page_fault);
        IDT.invalid_tss.set_handler_fn(invalid_tss);
        let handler = {
            let ptr = double_fault_handler as *const ();
            mem::transmute::<*const (), DivergingHandlerFuncWithErrCode>(ptr)
        };
        IDT.double_fault.set_handler_fn(handler).set_stack_index(0);
        IDT.load();
    }
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame)
{
    klog!(Info, "Breakpoint.");
}

extern "x86-interrupt" fn page_fault(_stack_frame: InterruptStackFrame, page_fault_error_code: PageFaultErrorCode)
{
    klog!(Error, "Page fault. Error code: {}", page_fault_error_code.bits());
}

extern "x86-interrupt" fn invalid_tss(_stack_frame: InterruptStackFrame, error_code: u64)
{
    klog!(Error, "Invalid TSS. Error code: {}", error_code);
}

extern "x86-interrupt" fn double_fault_handler(_stack_frame: InterruptStackFrame, error_code: u64)
{
    klog!(Error, "Double fault. Error code: {}", error_code);
    loop {}
}
