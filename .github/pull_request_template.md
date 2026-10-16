## What changes

## How it was checked

- [ ] `cargo clippy --all-targets` prints no warning
- [ ] `cargo test` passes, with tests for the change
- [ ] Tried as git's pager (`git -c core.pager="$PWD/target/debug/glance" log -p`) when the interface changed
