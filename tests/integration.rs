mod common;

use common::assert_identical;

#[test]
fn min_cost_flow_small() {
    assert_identical("13502460 1 512 2 2 1000 10 100 200 0 0 20 100 10 1000\n");
}

#[test]
fn assignment_problem() {
    assert_identical("12345 1 100 50 50 500 1 100 50 0 0 0 0 1 100\n");
}

#[test]
fn max_flow_problem() {
    assert_identical("99999 1 200 5 5 1000 1 1 500 2 2 20 50 10 100\n");
}

#[test]
fn min_cost_flow_large() {
    assert_identical("7654321 1 1024 10 10 5000 5 500 1000 3 3 30 80 50 2000\n");
}

#[test]
fn stress_8k_nodes() {
    assert_identical("13502460 1 8192 50 50 50000 1 1000 10000 10 10 25 75 100 5000\n");
}

#[test]
fn multi_problem_sequence() {
    assert_identical(
        "13502460 1 512 2 2 1000 10 100 200 0 0 20 100 10 1000\n\
         12345 2 100 50 50 500 1 100 50 0 0 0 0 1 100\n",
    );
}

#[test]
fn assignment_large() {
    assert_identical("42 1 200 100 100 2000 10 500 100 0 0 0 0 1 1\n");
}

#[test]
fn various_seeds() {
    for seed in [1, 42, 1000, 999999, 2147483646] {
        let input = format!("{seed} 1 256 4 4 2000 1 100 500 1 1 10 50 5 200\n");
        assert_identical(&input);
    }
}
