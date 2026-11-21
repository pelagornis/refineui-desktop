// Stub for cross-checking on Linux (bindgen output needs the macOS SDK).
#[repr(C)]
pub struct dispatch_queue_s {
    _p: [u8; 0],
}
pub type dispatch_queue_t = *mut dispatch_queue_s;
#[repr(C)]
pub struct dispatch_source_s {
    _p: [u8; 0],
}
pub type dispatch_source_t = *mut dispatch_source_s;
#[repr(C)]
pub struct dispatch_source_type_s {
    _p: [u8; 0],
}
pub type dispatch_source_type_t = *const dispatch_source_type_s;
#[repr(C)]
pub struct dispatch_queue_global_s {
    _p: [u8; 0],
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union dispatch_object_t {
    pub _ds: dispatch_source_t,
    pub _dq: dispatch_queue_t,
}
pub type dispatch_time_t = u64;
pub type dispatch_function_t = Option<unsafe extern "C" fn(arg1: *mut ::std::os::raw::c_void)>;
pub const DISPATCH_QUEUE_PRIORITY_HIGH: u32 = 2;
pub const DISPATCH_TIME_NOW: u32 = 0;
unsafe extern "C" {
    pub static _dispatch_main_q: dispatch_queue_s;
    pub static _dispatch_source_type_data_add: dispatch_source_type_s;
    pub fn dispatch_get_global_queue(identifier: isize, flags: usize) -> dispatch_queue_t;
    pub fn dispatch_async_f(
        queue: dispatch_queue_t,
        context: *mut ::std::os::raw::c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_after_f(
        when: dispatch_time_t,
        queue: dispatch_queue_t,
        context: *mut ::std::os::raw::c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_time(when: dispatch_time_t, delta: i64) -> dispatch_time_t;
    pub fn dispatch_source_merge_data(source: dispatch_source_t, value: usize);
    pub fn dispatch_source_create(
        type_: dispatch_source_type_t,
        handle: usize,
        mask: usize,
        queue: dispatch_queue_t,
    ) -> dispatch_source_t;
    pub fn dispatch_source_set_event_handler_f(
        source: dispatch_source_t,
        handler: dispatch_function_t,
    );
    pub fn dispatch_resume(object: dispatch_object_t);
    pub fn dispatch_suspend(object: dispatch_object_t);
    pub fn dispatch_source_cancel(source: dispatch_source_t);
    pub fn dispatch_set_context(object: dispatch_object_t, context: *mut ::std::os::raw::c_void);
}
