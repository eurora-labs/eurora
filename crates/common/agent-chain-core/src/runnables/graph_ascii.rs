use std::collections::HashMap;

use rust_sugiyama::configure::Config;

use super::graph::Edge;

const VERTEX_HEIGHT: usize = 3;

fn vertex_width(name: &str) -> usize {
    name.len() + 2
}

struct AsciiCanvas {
    cols: usize,
    lines: usize,
    canvas: Vec<Vec<char>>,
}

impl AsciiCanvas {
    fn new(cols: usize, lines: usize) -> Result<Self, String> {
        if cols <= 1 || lines <= 1 {
            return Err("Canvas dimensions should be > 1".to_string());
        }
        Ok(Self {
            cols,
            lines,
            canvas: vec![vec![' '; cols]; lines],
        })
    }

    fn draw(&self) -> String {
        self.canvas
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn point(&mut self, x: usize, y: usize, ch: char) {
        if x < self.cols && y < self.lines {
            self.canvas[y][x] = ch;
        }
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, ch: char) {
        let (x0, y0, x1, y1) = if x0 > x1 {
            (x1, y1, x0, y0)
        } else {
            (x0, y0, x1, y1)
        };

        let dx = x1 - x0;
        let dy = y1 - y0;

        if dx == 0 && dy == 0 {
            self.point(x0 as usize, y0 as usize, ch);
        } else if dx.abs() >= dy.abs() {
            for x in x0..=x1 {
                let y = if dx == 0 {
                    y0
                } else {
                    y0 + ((x - x0) as f64 * dy as f64 / dx as f64).round() as i32
                };
                self.point(x as usize, y as usize, ch);
            }
        } else if y0 < y1 {
            for y in y0..=y1 {
                let x = if dy == 0 {
                    x0
                } else {
                    x0 + ((y - y0) as f64 * dx as f64 / dy as f64).round() as i32
                };
                self.point(x as usize, y as usize, ch);
            }
        } else {
            for y in y1..=y0 {
                let x = if dy == 0 {
                    x1
                } else {
                    x1 + ((y - y1) as f64 * dx as f64 / dy as f64).round() as i32
                };
                self.point(x as usize, y as usize, ch);
            }
        }
    }

    fn text(&mut self, x: usize, y: usize, text: &str) {
        for (i, ch) in text.chars().enumerate() {
            self.point(x + i, y, ch);
        }
    }

    fn draw_box(&mut self, x0: usize, y0: usize, width: usize, height: usize) {
        if width <= 1 || height <= 1 {
            return;
        }

        let width = width - 1;
        let height = height - 1;

        for x in x0..x0 + width {
            self.point(x, y0, '-');
            self.point(x, y0 + height, '-');
        }

        for y in y0..y0 + height {
            self.point(x0, y, '|');
            self.point(x0 + width, y, '|');
        }

        self.point(x0, y0, '+');
        self.point(x0 + width, y0, '+');
        self.point(x0, y0 + height, '+');
        self.point(x0 + width, y0 + height, '+');
    }
}

pub fn draw_ascii(vertices: &HashMap<String, String>, edges: &[Edge]) -> Result<String, String> {
    if vertices.is_empty() {
        return Ok(String::new());
    }

    let id_list: Vec<&String> = vertices.keys().collect();
    let id_to_idx: HashMap<&String, u32> = id_list
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i as u32))
        .collect();
    let idx_to_id: HashMap<u32, &String> = id_list
        .iter()
        .enumerate()
        .map(|(i, id)| (i as u32, *id))
        .collect();

    let sugiyama_vertices: Vec<(u32, (f64, f64))> = vertices
        .iter()
        .map(|(id, label)| {
            let idx = id_to_idx[id];
            let label_with_spaces = format!(" {} ", label);
            let width = vertex_width(&label_with_spaces) as f64;
            (idx, (width, VERTEX_HEIGHT as f64))
        })
        .collect();

    let sugiyama_edges: Vec<(u32, u32)> = edges
        .iter()
        .filter_map(|edge| {
            let source_idx = id_to_idx.get(&edge.source)?;
            let target_idx = id_to_idx.get(&edge.target)?;
            Some((*source_idx, *target_idx))
        })
        .collect();

    let edge_conditional: HashMap<(u32, u32), bool> = edges
        .iter()
        .filter_map(|edge| {
            let source_idx = id_to_idx.get(&edge.source)?;
            let target_idx = id_to_idx.get(&edge.target)?;
            Some(((*source_idx, *target_idx), edge.conditional))
        })
        .collect();

    let config = Config::default();
    let layouts =
        rust_sugiyama::from_vertices_and_edges(&sugiyama_vertices, &sugiyama_edges, &config);

    if layouts.is_empty() {
        return Ok(String::new());
    }

    let mut all_positions: Vec<(usize, (f64, f64))> = Vec::new();
    for (layout, _width, _height) in &layouts {
        all_positions.extend_from_slice(layout);
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for &(idx, (x, y)) in &all_positions {
        let id = idx_to_id[&(idx as u32)];
        let label = &vertices[id];
        let label_with_spaces = format!(" {} ", label);
        let w = vertex_width(&label_with_spaces) as f64;
        let h = VERTEX_HEIGHT as f64;

        let left = x - w / 2.0;
        let right = x + w / 2.0;
        min_x = min_x.min(left);
        max_x = max_x.max(right);
        min_y = min_y.min(y);
        max_y = max_y.max(y + h);
    }

    let canvas_cols = (max_x - min_x).ceil() as usize + 1;
    let canvas_lines = (max_y - min_y).ceil() as usize + 1;

    let canvas_cols = canvas_cols.max(2);
    let canvas_lines = canvas_lines.max(2);

    let mut canvas = AsciiCanvas::new(canvas_cols, canvas_lines)?;

    let mut vertex_positions: HashMap<usize, (f64, f64, f64, f64)> = HashMap::new();
    for &(idx, (x, y)) in &all_positions {
        let id = idx_to_id[&(idx as u32)];
        let label = &vertices[id];
        let label_with_spaces = format!(" {} ", label);
        let w = vertex_width(&label_with_spaces) as f64;
        let h = VERTEX_HEIGHT as f64;
        vertex_positions.insert(idx, (x, y, w, h));
    }

    for edge in &sugiyama_edges {
        let source_idx = edge.0 as usize;
        let target_idx = edge.1 as usize;

        if let (Some(&(sx, sy, _sw, sh)), Some(&(tx, ty, _tw, _th))) = (
            vertex_positions.get(&source_idx),
            vertex_positions.get(&target_idx),
        ) {
            let start_x = (sx - min_x).round() as i32;
            let start_y = (sy + sh - min_y).round() as i32;
            let end_x = (tx - min_x).round() as i32;
            let end_y = (ty - min_y).round() as i32;

            let ch = if *edge_conditional.get(edge).unwrap_or(&false) {
                '.'
            } else {
                '*'
            };

            canvas.line(start_x, start_y, end_x, end_y, ch);
        }
    }

    for &(idx, (x, y)) in &all_positions {
        let id = idx_to_id[&(idx as u32)];
        let label = &vertices[id];
        let label_with_spaces = format!(" {} ", label);
        let w = vertex_width(&label_with_spaces);
        let h = VERTEX_HEIGHT;

        let box_x = (x - w as f64 / 2.0 - min_x).round() as usize;
        let box_y = (y - min_y).round() as usize;

        canvas.draw_box(box_x, box_y, w, h);
        canvas.text(box_x + 1, box_y + 1, &label_with_spaces);
    }

    Ok(canvas.draw())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_canvas_basic() {
        let mut canvas = AsciiCanvas::new(10, 5).unwrap();
        canvas.point(0, 0, 'X');
        canvas.point(9, 4, 'Y');
        let result = canvas.draw();
        assert!(result.starts_with('X'));
        assert!(result.ends_with('Y'));
    }

    #[test]
    fn test_ascii_canvas_invalid_dimensions() {
        assert!(AsciiCanvas::new(0, 0).is_err());
        assert!(AsciiCanvas::new(1, 1).is_err());
    }

    #[test]
    fn test_ascii_canvas_box() {
        let mut canvas = AsciiCanvas::new(6, 4).unwrap();
        canvas.draw_box(0, 0, 5, 3);
        let result = canvas.draw();
        assert!(result.contains("+---+"));
        assert!(result.contains("|   |"));
    }

    #[test]
    fn test_draw_ascii_simple() {
        let mut vertices = HashMap::new();
        vertices.insert("1".to_string(), "1".to_string());
        vertices.insert("2".to_string(), "2".to_string());

        let edges = vec![Edge {
            source: "1".to_string(),
            target: "2".to_string(),
            data: None,
            conditional: false,
        }];

        let result = draw_ascii(&vertices, &edges).unwrap();
        assert!(
            result.contains("1"),
            "Should contain vertex label '1': {}",
            result
        );
        assert!(
            result.contains("2"),
            "Should contain vertex label '2': {}",
            result
        );
        assert!(
            result.contains('+'),
            "Should contain box corners: {}",
            result
        );
    }

    #[test]
    fn test_draw_ascii_empty() {
        let vertices = HashMap::new();
        let edges: Vec<Edge> = vec![];
        let result = draw_ascii(&vertices, &edges).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_draw_ascii_diamond() {
        let mut vertices = HashMap::new();
        vertices.insert("1".to_string(), "1".to_string());
        vertices.insert("2".to_string(), "2".to_string());
        vertices.insert("3".to_string(), "3".to_string());
        vertices.insert("4".to_string(), "4".to_string());

        let edges = vec![
            Edge {
                source: "1".to_string(),
                target: "2".to_string(),
                data: None,
                conditional: false,
            },
            Edge {
                source: "1".to_string(),
                target: "4".to_string(),
                data: None,
                conditional: false,
            },
            Edge {
                source: "2".to_string(),
                target: "3".to_string(),
                data: None,
                conditional: false,
            },
            Edge {
                source: "2".to_string(),
                target: "4".to_string(),
                data: None,
                conditional: false,
            },
        ];

        let result = draw_ascii(&vertices, &edges).unwrap();
        for label in &["1", "2", "3", "4"] {
            assert!(
                result.contains(label),
                "Should contain vertex label '{}': {}",
                label,
                result
            );
        }
    }

    #[test]
    fn test_draw_ascii_conditional_edges() {
        let mut vertices = HashMap::new();
        vertices.insert("a".to_string(), "A".to_string());
        vertices.insert("b".to_string(), "B".to_string());

        let edges = vec![Edge {
            source: "a".to_string(),
            target: "b".to_string(),
            data: Some("condition".to_string()),
            conditional: true,
        }];

        let result = draw_ascii(&vertices, &edges).unwrap();
        assert!(result.contains('A'), "Should contain vertex A: {}", result);
        assert!(result.contains('B'), "Should contain vertex B: {}", result);
        assert!(
            result.contains('.'),
            "Should contain conditional edge dots: {}",
            result
        );
    }
}
