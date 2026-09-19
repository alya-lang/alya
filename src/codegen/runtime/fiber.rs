// ==============================================================================
// Alya M:N Cooperative Fiber Runtime Architecture ("Colorless Concurrency")
// ==============================================================================
// Specification Chapter 17 & ROADMAP Phase 4.8
//
// Provides lightweight user-space green fibers with cooperative scheduling,
// stack recycling/pooling, and sub-microsecond context switching over native
// OS worker thread pools without function coloring (rejecting async/await).

pub const FIBER_ID_OFFSET: usize = 0;
pub const FIBER_STATE_OFFSET: usize = 8;
pub const FIBER_STACK_BASE_OFFSET: usize = 16;
pub const FIBER_STACK_SIZE_OFFSET: usize = 24;
pub const FIBER_SAVED_SP_OFFSET: usize = 32;
pub const FIBER_FN_PTR_OFFSET: usize = 40;
pub const FIBER_ARG_OFFSET: usize = 48;
pub const FIBER_RESULT_OFFSET: usize = 56;
pub const FIBER_PARENT_OFFSET: usize = 64;
pub const FIBER_NEXT_OFFSET: usize = 72;
pub const FIBER_WAIT_DATA_OFFSET: usize = 80;

pub const FIBER_STRUCT_SIZE: usize = 128;
pub const DEFAULT_FIBER_STACK_SIZE: usize = 16384; // 16 KB initial stack

pub const FIBER_STATE_READY: u64 = 0;
pub const FIBER_STATE_RUNNING: u64 = 1;
pub const FIBER_STATE_WAITING: u64 = 2;
pub const FIBER_STATE_COMPLETED: u64 = 3;
