//! Tests for iter utilities.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/utils/test_iter.py`

use agent_chain_core::utils::iter::batch_iterate;

/// Test batching function.
///
/// Equivalent to Python test:
/// ```python
/// @pytest.mark.parametrize(
///     ("input_size", "input_iterable", "expected_output"),
///     [
///         (2, [1, 2, 3, 4, 5], [[1, 2], [3, 4], [5]]),
///         (3, [10, 20, 30, 40, 50], [[10, 20, 30], [40, 50]]),
///         (1, [100, 200, 300], [[100], [200], [300]]),
///         (4, [], []),
///     ],
/// )
/// def test_batch_iterate(
///     input_size: int, input_iterable: list[str], expected_output: list[list[str]]
/// ) -> None:
///     """Test batching function."""
///     assert list(batch_iterate(input_size, input_iterable)) == expected_output
/// ```
#[test]
fn test_batch_iterate() {
    let test_cases: Vec<(usize, Vec<i32>, Vec<Vec<i32>>)> = vec![
        (
            2,
            vec![1, 2, 3, 4, 5],
            vec![vec![1, 2], vec![3, 4], vec![5]],
        ),
        (
            3,
            vec![10, 20, 30, 40, 50],
            vec![vec![10, 20, 30], vec![40, 50]],
        ),
        (
            1,
            vec![100, 200, 300],
            vec![vec![100], vec![200], vec![300]],
        ),
        (4, vec![], vec![]),
    ];

    for (input_size, input_iterable, expected_output) in test_cases {
        let result: Vec<Vec<i32>> = batch_iterate(Some(input_size), input_iterable).collect();
        assert_eq!(result, expected_output);
    }
}
