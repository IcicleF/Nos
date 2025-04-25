#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(clippy::all)]

pub const _NUMA_H: u32 = 1;
pub const LIBNUMA_API_VERSION: u32 = 2;
pub type __pid_t = ::std::os::raw::c_int;
pub type pid_t = __pid_t;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct nodemask_t {
    pub n: [::std::os::raw::c_ulong; 2usize],
}
#[test]
fn bindgen_test_layout_nodemask_t() {
    const UNINIT: ::std::mem::MaybeUninit<nodemask_t> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<nodemask_t>(),
        16usize,
        concat!("Size of: ", stringify!(nodemask_t))
    );
    assert_eq!(
        ::std::mem::align_of::<nodemask_t>(),
        8usize,
        concat!("Alignment of ", stringify!(nodemask_t))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).n) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(nodemask_t),
            "::",
            stringify!(n)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bitmask {
    pub size: ::std::os::raw::c_ulong,
    pub maskp: *mut ::std::os::raw::c_ulong,
}
#[test]
fn bindgen_test_layout_bitmask() {
    const UNINIT: ::std::mem::MaybeUninit<bitmask> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<bitmask>(),
        16usize,
        concat!("Size of: ", stringify!(bitmask))
    );
    assert_eq!(
        ::std::mem::align_of::<bitmask>(),
        8usize,
        concat!("Alignment of ", stringify!(bitmask))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).size) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(bitmask),
            "::",
            stringify!(size)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).maskp) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(bitmask),
            "::",
            stringify!(maskp)
        )
    );
}
extern "C" {
    pub fn numa_bitmask_isbitset(
        arg1: *const bitmask,
        arg2: ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_bitmask_setall(arg1: *mut bitmask) -> *mut bitmask;
}
extern "C" {
    pub fn numa_bitmask_clearall(arg1: *mut bitmask) -> *mut bitmask;
}
extern "C" {
    pub fn numa_bitmask_setbit(arg1: *mut bitmask, arg2: ::std::os::raw::c_uint) -> *mut bitmask;
}
extern "C" {
    pub fn numa_bitmask_clearbit(arg1: *mut bitmask, arg2: ::std::os::raw::c_uint) -> *mut bitmask;
}
extern "C" {
    pub fn numa_bitmask_nbytes(arg1: *mut bitmask) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn numa_bitmask_weight(arg1: *const bitmask) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn numa_bitmask_alloc(arg1: ::std::os::raw::c_uint) -> *mut bitmask;
}
extern "C" {
    pub fn numa_bitmask_free(arg1: *mut bitmask);
}
extern "C" {
    pub fn numa_bitmask_equal(arg1: *const bitmask, arg2: *const bitmask) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn copy_nodemask_to_bitmask(arg1: *mut nodemask_t, arg2: *mut bitmask);
}
extern "C" {
    pub fn copy_bitmask_to_nodemask(arg1: *mut bitmask, arg2: *mut nodemask_t);
}
extern "C" {
    pub fn copy_bitmask_to_bitmask(arg1: *mut bitmask, arg2: *mut bitmask);
}
extern "C" {
    pub fn numa_available() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_max_node() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_max_possible_node() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_preferred() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_node_size64(
        node: ::std::os::raw::c_int,
        freep: *mut ::std::os::raw::c_longlong,
    ) -> ::std::os::raw::c_longlong;
}
extern "C" {
    pub fn numa_node_size(
        node: ::std::os::raw::c_int,
        freep: *mut ::std::os::raw::c_long,
    ) -> ::std::os::raw::c_long;
}
extern "C" {
    pub fn numa_pagesize() -> ::std::os::raw::c_int;
}
extern "C" {
    pub static mut numa_all_nodes_ptr: *mut bitmask;
}
extern "C" {
    pub static mut numa_nodes_ptr: *mut bitmask;
}
extern "C" {
    pub static mut numa_all_nodes: nodemask_t;
}
extern "C" {
    pub static mut numa_all_cpus_ptr: *mut bitmask;
}
extern "C" {
    pub static mut numa_no_nodes_ptr: *mut bitmask;
}
extern "C" {
    pub static mut numa_no_nodes: nodemask_t;
}
extern "C" {
    pub fn numa_bind(nodes: *mut bitmask);
}
extern "C" {
    pub fn numa_set_interleave_mask(nodemask: *mut bitmask);
}
extern "C" {
    pub fn numa_get_interleave_mask() -> *mut bitmask;
}
extern "C" {
    pub fn numa_allocate_nodemask() -> *mut bitmask;
}
extern "C" {
    pub fn numa_set_preferred(node: ::std::os::raw::c_int);
}
extern "C" {
    pub fn numa_set_localalloc();
}
extern "C" {
    pub fn numa_set_membind(nodemask: *mut bitmask);
}
extern "C" {
    pub fn numa_get_membind() -> *mut bitmask;
}
extern "C" {
    pub fn numa_get_mems_allowed() -> *mut bitmask;
}
extern "C" {
    pub fn numa_get_interleave_node() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_alloc_interleaved_subset(
        size: usize,
        nodemask: *mut bitmask,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn numa_alloc_interleaved(size: usize) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn numa_alloc_onnode(
        size: usize,
        node: ::std::os::raw::c_int,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn numa_alloc_local(size: usize) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn numa_alloc(size: usize) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn numa_realloc(
        old_addr: *mut ::std::os::raw::c_void,
        old_size: usize,
        new_size: usize,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn numa_free(mem: *mut ::std::os::raw::c_void, size: usize);
}
extern "C" {
    pub fn numa_interleave_memory(
        mem: *mut ::std::os::raw::c_void,
        size: usize,
        mask: *mut bitmask,
    );
}
extern "C" {
    pub fn numa_tonode_memory(
        start: *mut ::std::os::raw::c_void,
        size: usize,
        node: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn numa_tonodemask_memory(
        mem: *mut ::std::os::raw::c_void,
        size: usize,
        mask: *mut bitmask,
    );
}
extern "C" {
    pub fn numa_setlocal_memory(start: *mut ::std::os::raw::c_void, size: usize);
}
extern "C" {
    pub fn numa_police_memory(start: *mut ::std::os::raw::c_void, size: usize);
}
extern "C" {
    pub fn numa_run_on_node_mask(mask: *mut bitmask) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_run_on_node_mask_all(mask: *mut bitmask) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_run_on_node(node: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_get_run_node_mask() -> *mut bitmask;
}
extern "C" {
    pub fn numa_set_bind_policy(strict: ::std::os::raw::c_int);
}
extern "C" {
    pub fn numa_set_strict(flag: ::std::os::raw::c_int);
}
extern "C" {
    pub fn numa_num_possible_nodes() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_num_possible_cpus() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_num_configured_nodes() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_num_configured_cpus() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_num_task_cpus() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_num_thread_cpus() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_num_task_nodes() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_num_thread_nodes() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_allocate_cpumask() -> *mut bitmask;
}
extern "C" {
    pub fn numa_node_to_cpus(
        arg1: ::std::os::raw::c_int,
        arg2: *mut bitmask,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_node_of_cpu(cpu: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_distance(
        node1: ::std::os::raw::c_int,
        node2: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_error(where_: *mut ::std::os::raw::c_char);
}
extern "C" {
    pub static mut numa_exit_on_error: ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_warn(num: ::std::os::raw::c_int, fmt: *mut ::std::os::raw::c_char, ...);
}
extern "C" {
    pub static mut numa_exit_on_warn: ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_migrate_pages(
        pid: ::std::os::raw::c_int,
        from: *mut bitmask,
        to: *mut bitmask,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_move_pages(
        pid: ::std::os::raw::c_int,
        count: ::std::os::raw::c_ulong,
        pages: *mut *mut ::std::os::raw::c_void,
        nodes: *const ::std::os::raw::c_int,
        status: *mut ::std::os::raw::c_int,
        flags: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_sched_getaffinity(arg1: pid_t, arg2: *mut bitmask) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_sched_setaffinity(arg1: pid_t, arg2: *mut bitmask) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn numa_parse_nodestring(arg1: *const ::std::os::raw::c_char) -> *mut bitmask;
}
extern "C" {
    pub fn numa_parse_nodestring_all(arg1: *const ::std::os::raw::c_char) -> *mut bitmask;
}
extern "C" {
    pub fn numa_parse_cpustring(arg1: *const ::std::os::raw::c_char) -> *mut bitmask;
}
extern "C" {
    pub fn numa_parse_cpustring_all(arg1: *const ::std::os::raw::c_char) -> *mut bitmask;
}
