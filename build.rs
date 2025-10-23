use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        mutex: { all(target_os = "linux", feature = "mutex") },
        random: {all(target_os = "linux", feature = "random")},
        semaphore: {all(target_os = "linux", feature = "semaphore")},
        not_linux: { not(target_os="linux")},
    }
}
