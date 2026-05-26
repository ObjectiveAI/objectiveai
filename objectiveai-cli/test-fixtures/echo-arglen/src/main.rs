//! Test-fixture tool. Prints one line containing
//! `args.len() + sum(s.len() for s in args)` (counting only the
//! forwarded args — `argv[0]` is skipped). Used by the
//! vector-completion snapshot tests so the 20-agent swarm's mock
//! tool calls return a deterministic-per-args integer instead of a
//! `plugin not found` error.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let total: usize = args.len() + args.iter().map(|s| s.len()).sum::<usize>();
    println!("{total}");
}
