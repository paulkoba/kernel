use crate::LogLevel;
use core::mem;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::{
    DivergingHandlerFuncWithErrCode, InterruptDescriptorTable, PageFaultErrorCode,
};

use crate::interrupt_idx::InterruptIndex;
use crate::interrupts::PICS;
use crate::time;
use crate::{klog, logging};
use core::fmt::Write;
use x86_64::registers::control::Cr2;

static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init_idt() {
    #[allow(static_mut_refs)]
    unsafe {
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.page_fault.set_handler_fn(page_fault);
        IDT.invalid_tss.set_handler_fn(invalid_tss);
        IDT.cp_protection_exception
            .set_handler_fn(cp_protection_exception);
        IDT.general_protection_fault
            .set_handler_fn(general_protection_fault);
        IDT.invalid_opcode.set_handler_fn(invalid_opcode);
        IDT.segment_not_present.set_handler_fn(segment_not_present);
        IDT.divide_error.set_handler_fn(divide_error);
        IDT.stack_segment_fault.set_handler_fn(stack_segment_fault);

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

extern "x86-interrupt" fn cp_protection_exception(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    klog!(
        Info,
        "Coprocessor protection exception. Error code: {}",
        error_code
    );
}

extern "x86-interrupt" fn invalid_opcode(_stack_frame: InterruptStackFrame) {
    klog!(Error, "Invalid opcode.");
}

extern "x86-interrupt" fn general_protection_fault(
    _stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    klog!(
        Error,
        "General protection fault. Error code: {:?}",
        error_code
    );
}

extern "x86-interrupt" fn segment_not_present(_stack_frame: InterruptStackFrame, error_code: u64) {
    klog!(Error, "Segment not present. Error code: {}", error_code);
}

extern "x86-interrupt" fn stack_segment_fault(_stack_frame: InterruptStackFrame, error_code: u64) {
    klog!(Error, "Stack segment fault. Error code: {}", error_code);
}

extern "x86-interrupt" fn divide_error(_stack_frame: InterruptStackFrame) {
    klog!(Error, "Divide error.");
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    klog!(Info, "Breakpoint.");
}

extern "x86-interrupt" fn page_fault(
    _stack_frame: InterruptStackFrame,
    page_fault_error_code: PageFaultErrorCode,
) {
    let fault_addr = Cr2::read();

    if fault_addr.is_err() {
        klog!(Error, "Page fault. Error code: {:?}", page_fault_error_code);
    } else {
        let fault_addr: *const u8 = fault_addr.unwrap().as_ptr();
        klog!(
            Error,
            "Page fault at address: {:#?}, error code: {:?}",
            fault_addr,
            page_fault_error_code
        );
    }
}

extern "x86-interrupt" fn invalid_tss(_stack_frame: InterruptStackFrame, error_code: u64) {
    klog!(Error, "Invalid TSS. Error code: {}", error_code);
}

extern "x86-interrupt" fn double_fault_handler(_stack_frame: InterruptStackFrame, error_code: u64) {
    klog!(Error, "Double fault. Error code: {}", error_code);
    loop {}
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        time::PIT_TICK_COUNT += 1;
    }

    #[allow(static_mut_refs)]
    unsafe {
        PICS.notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    klog!(Debug, "Keyboard interrupt: Scancode: {:#04x}", scancode);

    #[allow(static_mut_refs)]
    unsafe {
        PICS.notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
