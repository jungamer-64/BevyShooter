#[cfg(all(feature = "dev-logging", debug_assertions))]
#[macro_export]
macro_rules! dlog {
    ($($t:tt)*) => {
        ::bevy::log::info!($($t)*)
    };
}

#[cfg(not(all(feature = "dev-logging", debug_assertions)))]
#[macro_export]
macro_rules! dlog {
    ($($t:tt)*) => {};
}

mod game;

fn main() {
    game::run();
}
