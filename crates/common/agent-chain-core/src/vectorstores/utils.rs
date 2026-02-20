use crate::{Error, Result};

pub fn cosine_similarity(x: &[Vec<f32>], y: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
    if x.is_empty() || y.is_empty() {
        return Ok(vec![vec![]]);
    }

    let m = x[0].len();
    for row in y {
        if row.len() != m {
            return Err(Error::Other(format!(
                "Number of columns in x and y must be the same. x has {} columns and y has {} columns.",
                m,
                row.len()
            )));
        }
    }

    let x_norms: Vec<f32> = x.iter().map(|row| l2_norm(row)).collect();
    let y_norms: Vec<f32> = y.iter().map(|row| l2_norm(row)).collect();

    let mut result = Vec::with_capacity(x.len());
    for (i, x_row) in x.iter().enumerate() {
        let mut row = Vec::with_capacity(y.len());
        for (j, y_row) in y.iter().enumerate() {
            let dot: f32 = x_row.iter().zip(y_row.iter()).map(|(a, b)| a * b).sum();
            let denom = x_norms[i] * y_norms[j];
            let sim = if denom == 0.0 { 0.0 } else { dot / denom };
            let sim = if sim.is_nan() || sim.is_infinite() {
                0.0
            } else {
                sim
            };
            row.push(sim);
        }
        result.push(row);
    }

    Ok(result)
}

fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

pub fn maximal_marginal_relevance(
    query_embedding: &[f32],
    embedding_list: &[Vec<f32>],
    lambda_mult: f32,
    k: usize,
) -> Result<Vec<usize>> {
    let effective_k = k.min(embedding_list.len());
    if effective_k == 0 {
        return Ok(vec![]);
    }

    let query_matrix = vec![query_embedding.to_vec()];
    let similarity_to_query = cosine_similarity(&query_matrix, embedding_list)?[0].clone();

    let most_similar = similarity_to_query
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    let mut idxs = vec![most_similar];
    let mut selected = vec![embedding_list[most_similar].clone()];

    while idxs.len() < effective_k {
        let similarity_to_selected = cosine_similarity(embedding_list, &selected)?;
        let mut best_score = f32::NEG_INFINITY;
        let mut idx_to_add = 0;

        for (i, query_score) in similarity_to_query.iter().enumerate() {
            if idxs.contains(&i) {
                continue;
            }
            let redundant_score = similarity_to_selected[i]
                .iter()
                .cloned()
                .fold(f32::NEG_INFINITY, f32::max);
            let equation_score = lambda_mult * query_score - (1.0 - lambda_mult) * redundant_score;
            if equation_score > best_score {
                best_score = equation_score;
                idx_to_add = i;
            }
        }

        idxs.push(idx_to_add);
        selected.push(embedding_list[idx_to_add].clone());
    }

    Ok(idxs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_basic() {
        let x = vec![vec![1.0, 0.0, 0.0]];
        let y = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
        let result = cosine_similarity(&x, &y).unwrap();
        assert!((result[0][0] - 1.0).abs() < 1e-6);
        assert!(result[0][1].abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let x: Vec<Vec<f32>> = vec![];
        let y = vec![vec![1.0, 0.0]];
        let result = cosine_similarity(&x, &y).unwrap();
        assert_eq!(result, vec![vec![] as Vec<f32>]);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let x = vec![vec![1.0, 0.0]];
        let y = vec![vec![1.0, 0.0, 0.0]];
        assert!(cosine_similarity(&x, &y).is_err());
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let x = vec![vec![0.0, 0.0]];
        let y = vec![vec![1.0, 0.0]];
        let result = cosine_similarity(&x, &y).unwrap();
        assert_eq!(result[0][0], 0.0);
    }

    #[test]
    fn test_mmr_basic() {
        let query = vec![1.0, 0.0];
        let embeddings = vec![vec![1.0, 0.0], vec![0.9, 0.1], vec![0.0, 1.0]];
        let result = maximal_marginal_relevance(&query, &embeddings, 0.5, 2).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0); // most similar first
    }

    #[test]
    fn test_mmr_empty() {
        let query = vec![1.0, 0.0];
        let embeddings: Vec<Vec<f32>> = vec![];
        let result = maximal_marginal_relevance(&query, &embeddings, 0.5, 4).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_mmr_k_larger_than_list() {
        let query = vec![1.0, 0.0];
        let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let result = maximal_marginal_relevance(&query, &embeddings, 0.5, 10).unwrap();
        assert_eq!(result.len(), 2);
    }
}
