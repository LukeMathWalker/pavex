use cargo_like_utils::shell::Shell;
use std::{
    fmt::Display,
    sync::{Arc, Mutex, OnceLock},
};

/// A global shell instance used by Pavex CLIs to print messages to the user.
pub static SHELL: OnceLock<Arc<Mutex<Shell>>> = OnceLock::new();

/// Customize how messages are shown in the shell.
pub mod style {
    pub use cargo_like_utils::shell::style::*;
}

/// Initialize the global shell instance, if it hasn't been initialized yet.
pub fn try_init_shell(shell: Shell) {
    SHELL.get_or_init(|| Arc::new(Mutex::new(shell)));
}

pub trait ShellExt {
    fn status<T, U>(&self, status: T, message: U)
    where
        T: Display,
        U: Display;
    fn status_with_color<T, U>(&self, status: T, message: U, color: &style::Style)
    where
        T: Display,
        U: Display;
    fn note<T: Display>(&self, message: T);
    fn warn<T: Display>(&self, message: T);
}

impl ShellExt for OnceLock<Arc<Mutex<Shell>>> {
    fn status<T, U>(&self, status: T, message: U)
    where
        T: Display,
        U: Display,
    {
        self.get().map(|s| {
            let _ = s.lock().unwrap().status(status, message);
        });
    }

    fn status_with_color<T, U>(&self, status: T, message: U, color: &style::Style)
    where
        T: Display,
        U: Display,
    {
        self.get().map(|s| {
            let _ = s.lock().unwrap().status_with_color(status, message, color);
        });
    }

    fn note<T: Display>(&self, message: T) {
        self.get().map(|s| {
            let _ = s.lock().unwrap().note(message);
        });
    }

    fn warn<T: Display>(&self, message: T) {
        self.get().map(|s| {
            let _ = s.lock().unwrap().warn(message);
        });
    }
}
