// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use prelude::v1::*;

use alloc::boxed::FnBox;
use io;
use mem;
use libc::c_void;
use ptr;
use sys::c;
use sys::handle::Handle;
use sys::stack_overflow;
use time::Duration;

pub struct Thread {
    handle: Handle
}

impl Thread {
    pub unsafe fn new<F: FnOnce()>(stack: usize, p: F)
                          -> io::Result<Thread> {
        extern "system" fn thread_start<F: FnOnce()>(main: *mut c_void)
            -> c::DWORD {
            unsafe {
                let main = Box::from_raw(main as *mut F);

                // Next, set up our stack overflow handler which may get triggered if we run
                // out of stack.
                let _handler = stack_overflow::Handler::new();

                // Finally, let's run some code.
                main();
            }
            0
        }

        let p = box p;

        match Thread::new_inner(stack, &*p as *const _ as *const _, thread_start::<F>) {
            Ok(thread) => {
                mem::forget(p); // ownership passed to CreateThread
                Ok(thread)
            }
            Err(e) => Err(e),
        }
    }

    unsafe fn new_inner(stack: usize, p: *const c_void,
                 f: extern "system" fn(*mut c_void) -> *mut c::DWORD)
                 -> io::Result<Thread> {
        // FIXME On UNIX, we guard against stack sizes that are too small but
        // that's because pthreads enforces that stacks are at least
        // PTHREAD_STACK_MIN bytes big.  Windows has no such lower limit, it's
        // just that below a certain threshold you can't do anything useful.
        // That threshold is application and architecture-specific, however.
        // Round up to the next 64 kB because that's what the NT kernel does,
        // might as well make it explicit.
        let stack_size = (stack + 0xfffe) & (!0xfffe);
        let ret = c::CreateThread(ptr::null_mut(), stack,
                                  f, p as *mut _,
                                  0, ptr::null_mut());

        return if ret as usize == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(Thread { handle: Handle::new(ret) })
        };
    }

    pub fn set_name(_name: &str) {
        // Windows threads are nameless
        // The names in MSVC debugger are obtained using a "magic" exception,
        // which requires a use of MS C++ extensions.
        // See https://msdn.microsoft.com/en-us/library/xcb2z8hs.aspx
    }

    pub fn join(self) {
        unsafe { c::WaitForSingleObject(self.handle.raw(), c::INFINITE); }
    }

    pub fn yield_now() {
        // This function will return 0 if there are no other threads to execute,
        // but this also means that the yield was useless so this isn't really a
        // case that needs to be worried about.
        unsafe { c::SwitchToThread(); }
    }

    pub fn sleep(dur: Duration) {
        unsafe {
            c::Sleep(super::dur2timeout(dur))
        }
    }

    pub fn handle(&self) -> &Handle { &self.handle }

    pub fn into_handle(self) -> Handle { self.handle }
}

#[cfg_attr(test, allow(dead_code))]
pub mod guard {
    pub unsafe fn current() -> Option<usize> { None }
    pub unsafe fn init() -> Option<usize> { None }
}
