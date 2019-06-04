// (C) Copyright 2019 Hewlett Packard Enterprise Development LP

pub mod types;
pub mod stdin;
pub mod stdin_hack;
pub mod kubernetes;
pub mod null;

pub use types::Reader;
pub use stdin::read_stdin;
pub use stdin_hack::read_stdin_hack;
pub use kubernetes::read_kubernetes_selector;
pub use null::read_null;
