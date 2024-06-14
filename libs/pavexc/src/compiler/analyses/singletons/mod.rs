mod cloning;
mod thread_safety;

pub(crate) use cloning::runtime_singletons_can_be_cloned_if_needed;
pub(crate) use thread_safety::runtime_singletons_are_thread_safe;
