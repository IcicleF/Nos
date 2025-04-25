mod bindings;

use std::ops::Range;

/// Return the first CPU ID range for the given NUMA node.
pub fn numa_node_to_cpus(numa: usize) -> Range<usize> {
    unsafe {
        let mask = bindings::numa_allocate_cpumask();
        bindings::numa_node_to_cpus(numa as i32, mask);

        let mut start = 0;
        while bindings::numa_bitmask_isbitset(mask, start) == 0 {
            start += 1;
        }

        let mut end = start + 1;
        while bindings::numa_bitmask_isbitset(mask, end) != 0 {
            end += 1;
        }
        bindings::numa_bitmask_free(mask);

        (start as usize)..(end as usize)
    }
}

/// Set allocation policy to local.
pub fn set_localalloc() {
    unsafe {
        bindings::numa_set_localalloc();
    }
}
