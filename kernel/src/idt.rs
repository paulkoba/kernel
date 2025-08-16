use crate::LogLevel;
use core::mem;
use x86_64::structures::idt::{DivergingHandlerFuncWithErrCode, InterruptDescriptorTable, PageFaultErrorCode};
use x86_64::structures::idt::InterruptStackFrame;

use core::fmt::Write;
use crate::{klog, logging};
use crate::interrupt_idx::InterruptIndex;
use crate::interrupts::PICS;
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

        IDT[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        IDT[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
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

extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    klog!(Debug, ".");
    #[allow(static_mut_refs)]
    unsafe {
        PICS.notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    klog!(Debug, "Keyboard interrupt: Scancode: {:#04x}", scancode);

    #[allow(static_mut_refs)]
    unsafe {
        PICS.notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
