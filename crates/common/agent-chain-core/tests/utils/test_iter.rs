use agent_chain_core::utils::iter::batch_iterate;

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
