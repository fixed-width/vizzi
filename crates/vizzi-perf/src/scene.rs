//! Deterministic synthetic geometry for renderer calibration, without code semantics.

/// Rectangles and straight edge endpoints in normalized device coordinates.
#[derive(Debug)]
pub struct Scene {
    /// Center x/y and half width/height for each node.
    pub nodes: Vec<[f32; 4]>,
    /// Two successive endpoints for each edge.
    pub edges: Vec<[f32; 2]>,
}

impl Scene {
    /// Construct a bounded grid with deterministic local and longer-range edges.
    pub fn grid(count: u32, edges_per_node: u32) -> Result<Self, &'static str> {
        if count == 0
            || count > 1_000_000
            || count
                .checked_mul(edges_per_node)
                .is_none_or(|edges| edges > 4_000_000)
        {
            return Err("scene requires 1..=1000000 nodes and at most 4000000 edges");
        }
        let columns = (count as f64).sqrt().ceil() as u32;
        let step = 1.9 / columns as f32;
        let nodes: Vec<_> = (0..count)
            .map(|index| {
                [
                    -0.95 + (index % columns) as f32 * step + step / 2.0,
                    -0.95 + (index / columns) as f32 * step + step / 2.0,
                    step * 0.25,
                    step * 0.25,
                ]
            })
            .collect();
        let mut edges = Vec::with_capacity((count * edges_per_node * 2) as usize);
        for index in 0..count {
            for offset in 0..edges_per_node {
                let target =
                    ((u64::from(index) + 1 + u64::from(offset) * 31) % u64::from(count)) as usize;
                let from = nodes[index as usize];
                let to = nodes[target];
                edges.extend_from_slice(&[[from[0], from[1]], [to[0], to[1]]]);
            }
        }
        Ok(Self { nodes, edges })
    }
}

/// Encode vertex attributes without unsafe memory casts.
pub fn vertex_bytes<const N: usize>(vertices: &[[f32; N]]) -> Vec<u8> {
    vertices
        .iter()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_has_requested_counts_and_edges_join_node_centers() {
        let scene = Scene::grid(10, 3).unwrap();
        assert_eq!((scene.nodes.len(), scene.edges.len()), (10, 60));
        assert!(
            scene
                .edges
                .iter()
                .all(|point| scene.nodes.iter().any(|node| point == &node[..2]))
        );
    }

    #[test]
    fn rejects_empty_or_excessive_scenes() {
        assert!(Scene::grid(0, 4).is_err());
        assert!(Scene::grid(1_000_001, 1).is_err());
        assert!(Scene::grid(1_000_000, 5).is_err());
    }

    #[test]
    fn encodes_all_vertex_attributes_in_little_endian_order() {
        assert_eq!(
            vertex_bytes(&[[0.0, 1.0], [-1.0, 2.0]]),
            vec![0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 128, 191, 0, 0, 0, 64,]
        );
    }

    #[test]
    fn four_nodes_fill_a_centered_two_by_two_grid() {
        let scene = Scene::grid(4, 0).unwrap();
        let expected = [
            [-0.475, -0.475, 0.2375, 0.2375],
            [0.475, -0.475, 0.2375, 0.2375],
            [-0.475, 0.475, 0.2375, 0.2375],
            [0.475, 0.475, 0.2375, 0.2375],
        ];
        assert!(
            scene
                .nodes
                .iter()
                .flatten()
                .zip(expected.iter().flatten())
                .all(|(actual, expected)| (actual - expected).abs() < 0.000001)
        );
    }

    #[test]
    fn edges_follow_the_requested_local_and_stride_links_with_wraparound() {
        let scene = Scene::grid(7, 2).unwrap();
        let pairs = [
            (0, 1),
            (0, 4),
            (1, 2),
            (1, 5),
            (2, 3),
            (2, 6),
            (3, 4),
            (3, 0),
            (4, 5),
            (4, 1),
            (5, 6),
            (5, 2),
            (6, 0),
            (6, 3),
        ];
        let expected: Vec<_> = pairs
            .into_iter()
            .flat_map(|(a, b)| {
                let a = scene.nodes[a];
                let b = scene.nodes[b];
                [[a[0], a[1]], [b[0], b[1]]]
            })
            .collect();
        assert_eq!(scene.edges, expected);
    }

    #[test]
    fn inclusive_node_limit_is_accepted() {
        assert_eq!(Scene::grid(1_000_000, 0).unwrap().nodes.len(), 1_000_000);
    }

    #[test]
    fn inclusive_edge_limit_is_accepted() {
        assert_eq!(Scene::grid(1, 4_000_000).unwrap().edges.len(), 8_000_000);
    }
}
