//! T03 part A: borrow-checker puzzles on a toy graph, in their fixed form.
//!
//! Run with `pixi run -e dev cargo run --example borrow_puzzles`.

/// A toy graph: `adjacency[i]` lists the neighbors of node `i`.
#[derive(Debug, Clone)]
struct Graph {
    adjacency: Vec<Vec<usize>>,
}

// Puzzle 1: taking `&Graph` instead of `Graph` lends the graph instead of moving it
fn degree_sum(graph: &Graph) -> usize {
    graph.adjacency.iter().map(Vec::len).sum()
}

// A `usize` is `Copy`, so passing it by value leaves the caller's copy usable
fn double(n: usize) -> usize {
    2 * n
}

// Puzzle 2: `&mut` borrows the graph exclusively for the duration of the call
fn add_edge(graph: &mut Graph, i: usize, j: usize) {
    graph.adjacency[i].push(j);
    graph.adjacency[j].push(i);
}

// Puzzle 3, fix 1: copy the neighbor list, then mutate freely
fn symmetrize_by_cloning(graph: &mut Graph) {
    for i in 0..graph.adjacency.len() {
        let neighbors = graph.adjacency[i].clone();
        for j in neighbors {
            if !graph.adjacency[j].contains(&i) {
                graph.adjacency[j].push(i);
            }
        }
    }
}

// Puzzle 3, fix 2: index instead of holding a borrow across the loop body
fn symmetrize_by_index(graph: &mut Graph) {
    for i in 0..graph.adjacency.len() {
        for k in 0..graph.adjacency[i].len() {
            let j = graph.adjacency[i][k];
            if !graph.adjacency[j].contains(&i) {
                graph.adjacency[j].push(i);
            }
        }
    }
}

// Puzzle 3, fix 3: take the list out (leaving an empty `Vec`), then put it back.
// No copy, but node `i` looks neighborless while its list is out: a self-loop
// `i -> i` would see an empty list and be added twice.
fn symmetrize_by_taking(graph: &mut Graph) {
    for i in 0..graph.adjacency.len() {
        let neighbors = std::mem::take(&mut graph.adjacency[i]);
        for &j in &neighbors {
            if !graph.adjacency[j].contains(&i) {
                graph.adjacency[j].push(i);
            }
        }
        graph.adjacency[i] = neighbors;
    }
}

// Puzzle 4: a `String` built inside the function must be returned by value
fn hub_name(graph: &Graph) -> String {
    let hub = (0..graph.adjacency.len())
        .max_by_key(|&i| graph.adjacency[i].len())
        .unwrap_or(0);
    format!("atom {hub}")
}

// ...while a string literal lives for the whole program, so it can be borrowed
fn graph_kind(graph: &Graph) -> &'static str {
    if graph.adjacency.is_empty() {
        "empty"
    } else {
        "molecular"
    }
}

// Puzzle 5: `&str` accepts literals, `&String` (by deref coercion) and slices
fn label(name: &str) -> String {
    format!("<{name}>")
}

// Puzzle 6: the result borrows from `a` or `b`, so all three share one lifetime
fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    // Puzzle 1
    let graph = Graph {
        adjacency: vec![vec![1], vec![0, 2], vec![1]],
    };
    let total = degree_sum(&graph);
    println!("degree sum {total} over {} nodes", graph.adjacency.len());
    let n = 3;
    println!("double {n} is {}", double(n));

    // Puzzle 2: end the shared borrow before the mutable one starts (or copy out
    // what you need: `let first = graph.adjacency[0].clone();`)
    let mut graph = graph;
    let first_degree = graph.adjacency[0].len();
    add_edge(&mut graph, 0, 2);
    println!(
        "node 0 had {first_degree} neighbors, now {:?}",
        graph.adjacency[0]
    );

    // Puzzle 3
    let directed = Graph {
        adjacency: vec![vec![1, 2], vec![], vec![1]],
    };
    for symmetrize in [
        symmetrize_by_cloning,
        symmetrize_by_index,
        symmetrize_by_taking,
    ] {
        let mut graph = directed.clone();
        symmetrize(&mut graph);
        println!("symmetrized: {:?}", graph.adjacency);
    }

    // Puzzle 4
    println!(
        "{} is the hub of a {} graph",
        hub_name(&graph),
        graph_kind(&graph)
    );

    // Puzzle 5
    let owned = String::from("Cl");
    println!("{} {} {}", label("C"), label(&owned), label(&owned[..1]));

    // Puzzle 6: the result can't outlive the shorter-lived argument, so `bromine`
    // has to live as long as `result` is used
    let bromine = String::from("Br");
    let result = longer(&bromine, "C");
    println!("longer: {result}");
}
