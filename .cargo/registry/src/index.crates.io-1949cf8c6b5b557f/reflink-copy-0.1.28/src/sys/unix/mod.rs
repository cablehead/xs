use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android"))] {
        mod linux;
        pub use linux::reflink;
        pub(crate) use linux::reflink_block;
    } else if #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos", target_os = "watchos"))] {
        mod macos;
        pub use macos::reflink;
        pub(crate) use super::reflink_block_not_supported as reflink_block;
    } else {
        pub use super::reflink_not_supported as reflink;
        pub(crate) use super::reflink_block_not_supported as reflink_block;
    }
}
