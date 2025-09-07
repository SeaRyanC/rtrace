use nalgebra::{Point3, Vector3};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// 3D point type alias
pub type Point = Point3<f64>;

/// 3D vector type alias  
pub type Vec3 = Vector3<f64>;

/// Triangle defined by three vertices and a normal
#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Point; 3],
    pub normal: Vec3,
}

impl Triangle {
    /// Get the center point of the triangle
    pub fn center(&self) -> Point {
        (self.vertices[0] + self.vertices[1].coords + self.vertices[2].coords) / 3.0
    }

    /// Get the bounding box of the triangle
    pub fn bounds(&self) -> (Point, Point) {
        let mut min = self.vertices[0];
        let mut max = self.vertices[0];

        for vertex in &self.vertices[1..] {
            min.coords = min.coords.inf(&vertex.coords);
            max.coords = max.coords.sup(&vertex.coords);
        }

        (min, max)
    }
}

/// K-d tree node for accelerating ray-triangle intersections
#[derive(Debug, Clone)]
enum KdNode {
    /// Internal node with splitting plane
    Internal {
        axis: usize,            // 0=x, 1=y, 2=z
        split_pos: f64,         // position along axis
        left: Box<KdNode>,      // left child (values <= split_pos)
        right: Box<KdNode>,     // right child (values > split_pos)
        bounds: (Point, Point), // bounding box of this node
    },
    /// Leaf node containing triangles
    Leaf {
        triangles: Vec<usize>,  // indices into mesh triangle array
        bounds: (Point, Point), // bounding box of this node
    },
}

/// K-d tree for accelerating ray-triangle intersections
///
/// A k-dimensional tree that recursively subdivides 3D space to enable
/// fast ray-triangle intersection queries. Instead of testing every triangle
/// in a mesh (O(n) complexity), the k-d tree allows logarithmic search time
/// O(log n) by only testing triangles in leaf nodes that the ray intersects.
///
/// For the 35,628 triangle Espresso Tray STL file, this provides significant
/// performance improvement over brute force intersection testing.
#[derive(Debug, Clone)]
pub struct KdTree {
    root: Option<KdNode>,
    max_depth: usize,
    max_triangles_per_leaf: usize,
}

impl KdTree {
    /// Create a new k-d tree for the given triangles
    pub fn new(triangles: &[Triangle], max_depth: usize, max_triangles_per_leaf: usize) -> Self {
        let mut tree = Self {
            root: None,
            max_depth,
            max_triangles_per_leaf,
        };

        if !triangles.is_empty() {
            // Create list of all triangle indices
            let triangle_indices: Vec<usize> = (0..triangles.len()).collect();

            // Compute overall bounds
            let bounds = Self::compute_bounds(triangles, &triangle_indices);

            // Build the tree recursively
            tree.root = Some(tree.build_recursive(triangles, triangle_indices, bounds, 0));

            // Debug: count leaf nodes
            let (leaf_count, max_leaf_triangles) = tree.count_leaf_nodes();
            println!(
                "K-d tree built: {} leaf nodes, max triangles per leaf: {}",
                leaf_count, max_leaf_triangles
            );
        }

        tree
    }

    /// Count leaf nodes and maximum triangles per leaf (for debugging)
    fn count_leaf_nodes(&self) -> (usize, usize) {
        if let Some(ref root) = self.root {
            self.count_leaf_nodes_recursive(root)
        } else {
            (0, 0)
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn count_leaf_nodes_recursive(&self, node: &KdNode) -> (usize, usize) {
        match node {
            KdNode::Leaf { triangles, .. } => (1, triangles.len()),
            KdNode::Internal { left, right, .. } => {
                let (left_count, left_max) = self.count_leaf_nodes_recursive(left.as_ref());
                let (right_count, right_max) = self.count_leaf_nodes_recursive(right.as_ref());
                (left_count + right_count, left_max.max(right_max))
            }
        }
    }

    /// Recursively build the k-d tree with surface area heuristic (SAH) optimization
    fn build_recursive(
        &self,
        triangles: &[Triangle],
        triangle_indices: Vec<usize>,
        bounds: (Point, Point),
        depth: usize,
    ) -> KdNode {
        // Create leaf if we've reached maximum depth or have few enough triangles
        if depth >= self.max_depth || triangle_indices.len() <= self.max_triangles_per_leaf {
            return KdNode::Leaf {
                triangles: triangle_indices,
                bounds,
            };
        }

        // Choose best splitting axis using Surface Area Heuristic (SAH)
        let best_axis = self.find_best_split_axis(triangles, &triangle_indices, &bounds);
        let axis = best_axis.unwrap_or(depth % 3);

        // Find optimal split position using SAH
        let mut positions: Vec<(f64, usize)> = triangle_indices
            .iter()
            .map(|&idx| {
                let triangle_center = triangles[idx].center();
                (triangle_center[axis], idx)
            })
            .collect();

        positions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // For small sets, use median split
        let split_pos = if positions.len() < 32 {
            let median_idx = positions.len() / 2;
            positions[median_idx].0
        } else {
            // For larger sets, use SAH to find optimal split
            self.find_optimal_split_position(triangles, &positions, &bounds, axis)
        };

        // Split triangles into left and right based on their bounding boxes
        let mut left_triangles = Vec::new();
        let mut right_triangles = Vec::new();

        for (_, triangle_idx) in positions {
            let triangle = &triangles[triangle_idx];
            let (tri_min, tri_max) = triangle.bounds();

            // Check if triangle overlaps with left region (values <= split_pos)
            if tri_min[axis] <= split_pos {
                left_triangles.push(triangle_idx);
            }

            // Check if triangle overlaps with right region (values > split_pos)  
            if tri_max[axis] > split_pos {
                right_triangles.push(triangle_idx);
            }
        }

        // Ensure we don't create empty splits
        if left_triangles.is_empty() {
            left_triangles.push(right_triangles.pop().unwrap());
        } else if right_triangles.is_empty() {
            right_triangles.push(left_triangles.pop().unwrap());
        }

        // Compute bounds for left and right children
        let left_bounds = Self::compute_bounds(triangles, &left_triangles);
        let right_bounds = Self::compute_bounds(triangles, &right_triangles);

        // Recursively build left and right subtrees
        let left =
            Box::new(self.build_recursive(triangles, left_triangles, left_bounds, depth + 1));
        let right =
            Box::new(self.build_recursive(triangles, right_triangles, right_bounds, depth + 1));

        KdNode::Internal {
            axis,
            split_pos,
            left,
            right,
            bounds,
        }
    }

    /// Find the best splitting axis using variance heuristic
    fn find_best_split_axis(
        &self,
        triangles: &[Triangle],
        triangle_indices: &[usize],
        bounds: &(Point, Point),
    ) -> Option<usize> {
        if triangle_indices.len() < 8 {
            return None; // Too few triangles for sophisticated analysis
        }

        let mut best_axis = 0;
        let mut max_variance = 0.0;

        for axis in 0..3 {
            // Skip very thin dimensions  
            let bounds_size = bounds.1[axis] - bounds.0[axis];
            if bounds_size < 1e-6 {
                continue;
            }

            // Calculate variance of triangle centers along this axis
            let mut sum = 0.0;
            let mut sum_sq = 0.0;
            let count = triangle_indices.len() as f64;

            for &idx in triangle_indices {
                let center_pos = triangles[idx].center()[axis];
                sum += center_pos;
                sum_sq += center_pos * center_pos;
            }

            let mean = sum / count;
            let variance = (sum_sq / count) - (mean * mean);

            if variance > max_variance {
                max_variance = variance;
                best_axis = axis;
            }
        }

        Some(best_axis)
    }

    /// Find optimal split position using simplified SAH
    fn find_optimal_split_position(
        &self,
        triangles: &[Triangle],
        positions: &[(f64, usize)],
        bounds: &(Point, Point),
        axis: usize,
    ) -> f64 {
        if positions.len() < 16 {
            // For small sets, just use median
            let median_idx = positions.len() / 2;
            return positions[median_idx].0;
        }

        // Sample a few candidate positions
        let candidates = [
            positions.len() / 4,
            positions.len() / 3,
            positions.len() / 2,
            (2 * positions.len()) / 3,
            (3 * positions.len()) / 4,
        ];

        let mut best_cost = f64::INFINITY;
        let mut best_pos = positions[positions.len() / 2].0;

        for &candidate_idx in &candidates {
            if candidate_idx >= positions.len() {
                continue;
            }

            let candidate_pos = positions[candidate_idx].0;
            let cost = self.evaluate_split_cost(triangles, positions, bounds, axis, candidate_pos);
            
            if cost < best_cost {
                best_cost = cost;
                best_pos = candidate_pos;
            }
        }

        best_pos
    }

    /// Evaluate split cost using simplified SAH
    fn evaluate_split_cost(
        &self,
        triangles: &[Triangle],
        positions: &[(f64, usize)],
        bounds: &(Point, Point),
        axis: usize,
        split_pos: f64,
    ) -> f64 {
        let mut left_count = 0;
        let mut right_count = 0;

        // Count triangles on each side
        for &(_, triangle_idx) in positions {
            let triangle = &triangles[triangle_idx];
            let (tri_min, tri_max) = triangle.bounds();

            if tri_min[axis] <= split_pos {
                left_count += 1;
            }
            if tri_max[axis] > split_pos {
                right_count += 1;
            }
        }

        // Calculate surface area ratio (simplified)
        let total_size = bounds.1[axis] - bounds.0[axis];
        let left_size = split_pos - bounds.0[axis];
        let right_size = bounds.1[axis] - split_pos;

        let left_area_ratio = if total_size > 1e-10 { left_size / total_size } else { 0.5 };
        let right_area_ratio = if total_size > 1e-10 { right_size / total_size } else { 0.5 };

        // SAH cost = cost_traverse + cost_left + cost_right
        // Simplified: just use triangle count weighted by area
        1.0 + left_area_ratio * left_count as f64 + right_area_ratio * right_count as f64
    }

    /// Compute bounding box for a set of triangles
    fn compute_bounds(triangles: &[Triangle], triangle_indices: &[usize]) -> (Point, Point) {
        if triangle_indices.is_empty() {
            return (Point::origin(), Point::origin());
        }

        let mut min = Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for &idx in triangle_indices {
            let (tri_min, tri_max) = triangles[idx].bounds();
            min.coords = min.coords.inf(&tri_min.coords);
            max.coords = max.coords.sup(&tri_max.coords);
        }

        (min, max)
    }

    /// Optimized ray-box intersection test with early termination
    fn ray_intersects_bounds(
        ray_origin: &Point,
        ray_direction: &Vec3,
        bounds: &(Point, Point),
    ) -> bool {
        let (min, max) = bounds;

        let mut t_min = f64::NEG_INFINITY;
        let mut t_max = f64::INFINITY;

        // Unrolled loop for better performance
        for axis in 0..3 {
            let dir_component = ray_direction[axis];
            let origin_component = ray_origin[axis];
            let min_bound = min[axis];
            let max_bound = max[axis];
            
            if dir_component.abs() < 1e-10 {
                // Ray is parallel to the slab - early exit if outside bounds
                if origin_component < min_bound || origin_component > max_bound {
                    return false;
                }
            } else {
                let inv_dir = 1.0 / dir_component;
                let mut t0 = (min_bound - origin_component) * inv_dir;
                let mut t1 = (max_bound - origin_component) * inv_dir;

                // Ensure t0 <= t1
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }

                // Update intersection interval
                t_min = t_min.max(t0);
                t_max = t_max.min(t1);

                // Early exit if interval becomes invalid
                if t_min > t_max {
                    return false;
                }
            }
        }

        // Check if intersection is in front of ray origin
        t_max >= 0.0
    }

    /// Traverse the k-d tree to find triangle candidates for ray intersection
    pub fn traverse<F>(&self, ray_origin: &Point, ray_direction: &Vec3, mut callback: F)
    where
        F: FnMut(&[usize]),
    {
        if let Some(ref root) = self.root {
            self.traverse_recursive(root, ray_origin, ray_direction, &mut callback);
        }
    }

    #[allow(dead_code, clippy::only_used_in_recursion)]
    fn traverse_recursive_with_count<F>(
        &self,
        node: &KdNode,
        ray_origin: &Point,
        ray_direction: &Vec3,
        callback: &mut F,
        node_count: &mut usize,
        leaf_count: &mut usize,
    ) where
        F: FnMut(&[usize]),
    {
        *node_count += 1;

        match node {
            KdNode::Leaf { triangles, bounds } => {
                *leaf_count += 1;
                // Check if ray intersects this leaf's bounds
                if Self::ray_intersects_bounds(ray_origin, ray_direction, bounds) {
                    callback(triangles);
                } else {
                    // Debug: check what triangles are in this leaf
                    println!("Leaf bounds check failed for leaf with {} triangles, bounds=({:.2},{:.2},{:.2}) to ({:.2},{:.2},{:.2})",
                        triangles.len(),
                        bounds.0.x, bounds.0.y, bounds.0.z,
                        bounds.1.x, bounds.1.y, bounds.1.z);

                    // Let's check the first few triangles in this leaf to see their actual bounds
                    #[allow(clippy::unused_enumerate_index)]
                    for (_i, &_tri_idx) in triangles.iter().take(2).enumerate() {
                        // This is a problem - we can't access the triangles array from here
                        // But we can check if any triangles have z-coordinates that would intersect our ray
                    }
                }
            }
            KdNode::Internal {
                axis,
                split_pos,
                left,
                right,
                bounds: _,
            } => {
                let origin_pos = ray_origin[*axis];
                let dir = ray_direction[*axis];

                // If ray is parallel to the splitting plane, only traverse the side it's on
                if dir.abs() < 1e-9 {
                    if origin_pos <= *split_pos {
                        self.traverse_recursive_with_count(
                            left.as_ref(),
                            ray_origin,
                            ray_direction,
                            callback,
                            node_count,
                            leaf_count,
                        );
                    } else {
                        self.traverse_recursive_with_count(
                            right.as_ref(),
                            ray_origin,
                            ray_direction,
                            callback,
                            node_count,
                            leaf_count,
                        );
                    }
                    return;
                }

                // Calculate where ray intersects the splitting plane
                let t_split = (*split_pos - origin_pos) / dir;

                // Traverse children in order based on ray direction
                // Always traverse the near child first, then the far child if the ray crosses the plane
                if origin_pos <= *split_pos {
                    // Ray starts in left child region
                    self.traverse_recursive_with_count(
                        left.as_ref(),
                        ray_origin,
                        ray_direction,
                        callback,
                        node_count,
                        leaf_count,
                    );
                    if t_split >= 0.0 {
                        self.traverse_recursive_with_count(
                            right.as_ref(),
                            ray_origin,
                            ray_direction,
                            callback,
                            node_count,
                            leaf_count,
                        );
                    }
                } else {
                    // Ray starts in right child region
                    self.traverse_recursive_with_count(
                        right.as_ref(),
                        ray_origin,
                        ray_direction,
                        callback,
                        node_count,
                        leaf_count,
                    );
                    if t_split >= 0.0 {
                        self.traverse_recursive_with_count(
                            left.as_ref(),
                            ray_origin,
                            ray_direction,
                            callback,
                            node_count,
                            leaf_count,
                        );
                    }
                }
            }
        }
    }

    /// Traverse the k-d tree with debug output
    pub fn traverse_debug<F>(&self, ray_origin: &Point, ray_direction: &Vec3, mut callback: F)
    where
        F: FnMut(&[usize]),
    {
        if let Some(ref root) = self.root {
            println!("Starting k-d tree traversal...");
            self.traverse_recursive_debug(root, ray_origin, ray_direction, &mut callback, 0);
        }
    }

    /// Optimized recursive traversal of the k-d tree with better branch prediction
    #[allow(clippy::only_used_in_recursion)]
    fn traverse_recursive<F>(
        &self,
        node: &KdNode,
        ray_origin: &Point,
        ray_direction: &Vec3,
        callback: &mut F,
    ) where
        F: FnMut(&[usize]),
    {
        match node {
            KdNode::Leaf { triangles, bounds } => {
                // Only call the expensive bounds check if we have triangles
                if !triangles.is_empty() && Self::ray_intersects_bounds(ray_origin, ray_direction, bounds) {
                    callback(triangles);
                }
            }
            KdNode::Internal {
                axis,
                split_pos,
                left,
                right,
                bounds: _,
            } => {
                let origin_pos = ray_origin[*axis];
                let dir = ray_direction[*axis];

                // Optimized traversal order for better cache locality
                if dir.abs() < 1e-10 {
                    // Ray parallel to splitting plane
                    if origin_pos <= *split_pos {
                        self.traverse_recursive(left.as_ref(), ray_origin, ray_direction, callback);
                    } else {
                        self.traverse_recursive(right.as_ref(), ray_origin, ray_direction, callback);
                    }
                } else {
                    // Calculate intersection with splitting plane
                    let t_split = (*split_pos - origin_pos) / dir;
                    
                    // Traverse near child first, then far child if ray crosses plane
                    let (first, second) = if origin_pos <= *split_pos {
                        (left.as_ref(), right.as_ref())
                    } else {
                        (right.as_ref(), left.as_ref())
                    };

                    // Always traverse the near child
                    self.traverse_recursive(first, ray_origin, ray_direction, callback);
                    
                    // Only traverse far child if ray actually crosses the splitting plane
                    if t_split >= 0.0 {
                        self.traverse_recursive(second, ray_origin, ray_direction, callback);
                    }
                }
            }
        }
    }

    /// Calculate ray-box intersection and return (t_near, t_far) if intersection exists
    #[allow(dead_code)]
    fn ray_bounds_intersection(
        ray_origin: &Point,
        ray_direction: &Vec3,
        bounds: &(Point, Point),
    ) -> Option<(f64, f64)> {
        let (min, max) = bounds;

        let mut t_min = f64::NEG_INFINITY;
        let mut t_max = f64::INFINITY;

        for axis in 0..3 {
            if ray_direction[axis].abs() < 1e-9 {
                // Ray is parallel to the slab
                if ray_origin[axis] < min[axis] || ray_origin[axis] > max[axis] {
                    return None;
                }
            } else {
                let inv_dir = 1.0 / ray_direction[axis];
                let mut t0 = (min[axis] - ray_origin[axis]) * inv_dir;
                let mut t1 = (max[axis] - ray_origin[axis]) * inv_dir;

                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }

                t_min = t_min.max(t0);
                t_max = t_max.min(t1);

                // Only check for invalid intersection after processing this axis
                if t_min > t_max {
                    return None;
                }
            }
        }

        // Check if the intersection is in front of the ray (t_max >= 0)
        if t_max >= 0.0 {
            Some((t_min.max(0.0), t_max))
        } else {
            None
        }
    }

    /// Recursive traversal of the k-d tree with debug output
    #[allow(clippy::only_used_in_recursion)]
    fn traverse_recursive_debug<F>(
        &self,
        node: &KdNode,
        ray_origin: &Point,
        ray_direction: &Vec3,
        callback: &mut F,
        depth: usize,
    ) where
        F: FnMut(&[usize]),
    {
        let indent = "  ".repeat(depth);

        match node {
            KdNode::Leaf { triangles, bounds } => {
                println!(
                    "{}Leaf: {} triangles, bounds=({:.1},{:.1},{:.1}) to ({:.1},{:.1},{:.1})",
                    indent,
                    triangles.len(),
                    bounds.0.x,
                    bounds.0.y,
                    bounds.0.z,
                    bounds.1.x,
                    bounds.1.y,
                    bounds.1.z
                );

                // Check if ray intersects this leaf's bounds
                let intersects = Self::ray_intersects_bounds(ray_origin, ray_direction, bounds);
                println!("{}  Ray intersects leaf bounds: {}", indent, intersects);

                if intersects {
                    println!(
                        "{}  Calling callback with {} triangles",
                        indent,
                        triangles.len()
                    );
                    callback(triangles);
                }
            }
            KdNode::Internal {
                axis,
                split_pos,
                left,
                right,
                bounds,
            } => {
                let axis_name = match axis {
                    0 => "X",
                    1 => "Y",
                    2 => "Z",
                    _ => "?",
                };
                println!("{}Internal: split on {} axis at {:.1}, bounds=({:.1},{:.1},{:.1}) to ({:.1},{:.1},{:.1})",
                    indent, axis_name, split_pos,
                    bounds.0.x, bounds.0.y, bounds.0.z,
                    bounds.1.x, bounds.1.y, bounds.1.z);

                // Only show detailed traversal for first few levels
                if depth < 3 {
                    let origin_pos = ray_origin[*axis];
                    let dir = ray_direction[*axis];

                    println!(
                        "{}  Ray origin on {} axis: {:.1}, direction: {:.6}",
                        indent, axis_name, origin_pos, dir
                    );

                    // If ray is parallel to the splitting plane
                    if dir.abs() < 1e-9 {
                        println!("{}  Ray is parallel to splitting plane", indent);
                        if origin_pos <= *split_pos {
                            println!("{}  Traversing LEFT child only", indent);
                            self.traverse_recursive_debug(
                                left.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                        } else {
                            println!("{}  Traversing RIGHT child only", indent);
                            self.traverse_recursive_debug(
                                right.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                        }
                        return;
                    }

                    // Calculate where ray intersects the splitting plane
                    let t_split = (*split_pos - origin_pos) / dir;
                    println!(
                        "{}  t_split = ({:.1} - {:.1}) / {:.6} = {:.6}",
                        indent, split_pos, origin_pos, dir, t_split
                    );

                    // Traverse children based on ray origin position
                    if origin_pos <= *split_pos {
                        println!(
                            "{}  Ray starts in LEFT region, traversing LEFT first",
                            indent
                        );
                        self.traverse_recursive_debug(
                            left.as_ref(),
                            ray_origin,
                            ray_direction,
                            callback,
                            depth + 1,
                        );
                        if t_split >= 0.0 {
                            println!("{}  t_split >= 0, also traversing RIGHT", indent);
                            self.traverse_recursive_debug(
                                right.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                        } else {
                            println!("{}  t_split < 0, NOT traversing RIGHT", indent);
                        }
                    } else {
                        println!(
                            "{}  Ray starts in RIGHT region, traversing RIGHT first",
                            indent
                        );
                        self.traverse_recursive_debug(
                            right.as_ref(),
                            ray_origin,
                            ray_direction,
                            callback,
                            depth + 1,
                        );
                        if t_split >= 0.0 {
                            println!("{}  t_split >= 0, also traversing LEFT", indent);
                            self.traverse_recursive_debug(
                                left.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                        } else {
                            println!("{}  t_split < 0, NOT traversing LEFT", indent);
                        }
                    }
                } else {
                    // Just traverse without detailed output for deeper levels
                    let origin_pos = ray_origin[*axis];
                    let dir = ray_direction[*axis];

                    if dir.abs() < 1e-9 {
                        if origin_pos <= *split_pos {
                            self.traverse_recursive_debug(
                                left.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                        } else {
                            self.traverse_recursive_debug(
                                right.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                        }
                    } else {
                        let t_split = (*split_pos - origin_pos) / dir;
                        if origin_pos <= *split_pos {
                            self.traverse_recursive_debug(
                                left.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                            if t_split >= 0.0 {
                                self.traverse_recursive_debug(
                                    right.as_ref(),
                                    ray_origin,
                                    ray_direction,
                                    callback,
                                    depth + 1,
                                );
                            }
                        } else {
                            self.traverse_recursive_debug(
                                right.as_ref(),
                                ray_origin,
                                ray_direction,
                                callback,
                                depth + 1,
                            );
                            if t_split >= 0.0 {
                                self.traverse_recursive_debug(
                                    left.as_ref(),
                                    ray_origin,
                                    ray_direction,
                                    callback,
                                    depth + 1,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Immutable mesh object containing triangles
#[derive(Debug, Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub bounds_min: Point,
    pub bounds_max: Point,
    pub kdtree: KdTree,
}

impl Mesh {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self {
            triangles: Vec::new(),
            bounds_min: Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            bounds_max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
            kdtree: KdTree::new(&[], 16, 10), // Empty k-d tree
        }
    }

    /// Load mesh from STL file (auto-detects binary vs ASCII)
    pub fn from_stl_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(&path)?;

        // Try to determine if this is ASCII or binary STL
        let mut header = [0u8; 80];
        file.read_exact(&mut header)?;

        let header_str = String::from_utf8_lossy(&header);
        if header_str.trim_start().starts_with("solid") {
            // Might be ASCII, but we need to check if it's actually ASCII throughout
            file.seek(SeekFrom::Start(0))?;
            if Self::is_ascii_stl(&mut file)? {
                file.seek(SeekFrom::Start(0))?;
                return Self::load_ascii_stl(file);
            }
        }

        // Binary STL
        file.seek(SeekFrom::Start(0))?;
        Self::load_binary_stl(file)
    }

    /// Load mesh from STL byte buffer (auto-detects binary vs ASCII)
    pub fn from_stl_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() < 80 {
            return Err("STL data too short".into());
        }

        let header_str = String::from_utf8_lossy(&bytes[0..80]);
        if header_str.trim_start().starts_with("solid") && Self::is_ascii_stl_bytes(bytes)? {
            Self::load_ascii_stl_bytes(bytes)
        } else {
            Self::load_binary_stl_bytes(bytes)
        }
    }

    /// Check if STL file is ASCII format by looking for ASCII markers
    fn is_ascii_stl(file: &mut File) -> Result<bool, Box<dyn std::error::Error>> {
        let reader = BufReader::new(file);
        let mut line_count = 0;

        for line in reader.lines() {
            let line = line?;
            line_count += 1;

            if line_count > 10 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.starts_with("facet normal")
                || trimmed == "outer loop"
                || trimmed == "endloop"
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if STL bytes represent ASCII format
    fn is_ascii_stl_bytes(bytes: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        let content = String::from_utf8_lossy(bytes);
        let lines: Vec<&str> = content.lines().take(10).collect();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("facet normal")
                || trimmed == "outer loop"
                || trimmed == "endloop"
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Load ASCII STL format
    fn load_ascii_stl(mut file: File) -> Result<Self, Box<dyn std::error::Error>> {
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Self::load_ascii_stl_bytes(content.as_bytes())
    }

    /// Load ASCII STL from bytes
    fn load_ascii_stl_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let content = String::from_utf8_lossy(bytes);
        let lines: Vec<&str> = content.lines().collect();

        let mut mesh = Mesh::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("facet normal") {
                // Parse normal vector
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 5 {
                    return Err("Invalid facet normal format".into());
                }

                let nx: f64 = parts[2].parse()?;
                let ny: f64 = parts[3].parse()?;
                let nz: f64 = parts[4].parse()?;
                let normal = Vec3::new(nx, ny, nz);

                i += 1; // Skip "outer loop"
                if i >= lines.len() || lines[i].trim() != "outer loop" {
                    return Err("Expected 'outer loop' after facet normal".into());
                }

                // Parse three vertices
                let mut vertices = [Point::origin(); 3];
                #[allow(clippy::needless_range_loop)]
                for j in 0..3 {
                    i += 1;
                    if i >= lines.len() {
                        return Err("Unexpected end of file while reading vertex".into());
                    }

                    let vertex_line = lines[i].trim();
                    if !vertex_line.starts_with("vertex") {
                        return Err("Expected vertex line".into());
                    }

                    let parts: Vec<&str> = vertex_line.split_whitespace().collect();
                    if parts.len() != 4 {
                        return Err("Invalid vertex format".into());
                    }

                    let x: f64 = parts[1].parse()?;
                    let y: f64 = parts[2].parse()?;
                    let z: f64 = parts[3].parse()?;
                    vertices[j] = Point::new(x, y, z);
                }

                i += 1; // Skip "endloop"
                if i >= lines.len() || lines[i].trim() != "endloop" {
                    return Err("Expected 'endloop'".into());
                }

                i += 1; // Skip "endfacet"
                if i >= lines.len() || lines[i].trim() != "endfacet" {
                    return Err("Expected 'endfacet'".into());
                }

                mesh.add_triangle(Triangle { vertices, normal });
            }

            i += 1;
        }

        mesh.compute_bounds();
        mesh.build_kdtree();
        Ok(mesh)
    }

    /// Load binary STL format
    fn load_binary_stl(mut file: File) -> Result<Self, Box<dyn std::error::Error>> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Self::load_binary_stl_bytes(&bytes)
    }

    /// Load binary STL from bytes
    fn load_binary_stl_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() < 84 {
            return Err("Binary STL too short".into());
        }

        // Skip 80-byte header, read triangle count
        let triangle_count =
            u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;

        let expected_size = 84 + triangle_count * 50;
        if bytes.len() < expected_size {
            return Err(format!(
                "Binary STL size mismatch: expected {}, got {}",
                expected_size,
                bytes.len()
            )
            .into());
        }

        let mut mesh = Mesh::new();
        let mut offset = 84;

        for _ in 0..triangle_count {
            if offset + 50 > bytes.len() {
                return Err("Unexpected end of binary STL data".into());
            }

            // Read normal (3 * f32)
            let nx = f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as f64;
            let ny = f32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as f64;
            let nz = f32::from_le_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]) as f64;
            let normal = Vec3::new(nx, ny, nz);
            offset += 12;

            // Read three vertices (3 * 3 * f32)
            let mut vertices = [Point::origin(); 3];
            #[allow(clippy::needless_range_loop)]
            for i in 0..3 {
                let x = f32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]) as f64;
                let y = f32::from_le_bytes([
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]) as f64;
                let z = f32::from_le_bytes([
                    bytes[offset + 8],
                    bytes[offset + 9],
                    bytes[offset + 10],
                    bytes[offset + 11],
                ]) as f64;
                vertices[i] = Point::new(x, y, z);
                offset += 12;
            }

            // Skip 2-byte attribute
            offset += 2;

            mesh.add_triangle(Triangle { vertices, normal });
        }

        mesh.compute_bounds();
        mesh.build_kdtree();
        Ok(mesh)
    }

    /// Add a triangle to the mesh
    fn add_triangle(&mut self, triangle: Triangle) {
        // Update bounding box
        for vertex in &triangle.vertices {
            self.bounds_min.coords = self.bounds_min.coords.inf(&vertex.coords);
            self.bounds_max.coords = self.bounds_max.coords.sup(&vertex.coords);
        }

        self.triangles.push(triangle);
    }

    /// Compute bounding box for the mesh
    fn compute_bounds(&mut self) {
        if self.triangles.is_empty() {
            self.bounds_min = Point::origin();
            self.bounds_max = Point::origin();
            return;
        }

        self.bounds_min = Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        self.bounds_max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for triangle in &self.triangles {
            for vertex in &triangle.vertices {
                self.bounds_min.coords = self.bounds_min.coords.inf(&vertex.coords);
                self.bounds_max.coords = self.bounds_max.coords.sup(&vertex.coords);
            }
        }
    }

    /// Build k-d tree for accelerating ray intersections with optimized parameters
    fn build_kdtree(&mut self) {
        // Better optimized parameters based on triangle count
        let triangle_count = self.triangles.len();
        let (max_depth, max_triangles_per_leaf) = if triangle_count < 100 {
            // Very small meshes: simple shallow tree
            (8, 32)
        } else if triangle_count < 1000 {
            // Small meshes: balanced approach
            (12, 20)
        } else if triangle_count < 10000 {
            // Medium meshes: deeper tree, fewer triangles per leaf
            (16, 15)
        } else if triangle_count < 100000 {
            // Large meshes: deeper tree, fewer triangles per leaf
            (20, 10)
        } else {
            // Very large meshes: conservative approach
            (24, 8)
        };
        
        self.kdtree = KdTree::new(&self.triangles, max_depth, max_triangles_per_leaf);
    }

    /// Get the number of triangles in the mesh
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Get mesh bounding box
    pub fn bounds(&self) -> (Point, Point) {
        (self.bounds_min, self.bounds_max)
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_mesh() {
        let mesh = Mesh::new();
        assert_eq!(mesh.triangle_count(), 0);
    }

    #[test]
    fn test_mesh_bounds() {
        let mut mesh = Mesh::new();
        let triangle = Triangle {
            vertices: [
                Point::new(-1.0, -1.0, -1.0),
                Point::new(1.0, -1.0, -1.0),
                Point::new(0.0, 1.0, -1.0),
            ],
            normal: Vec3::new(0.0, 0.0, 1.0),
        };

        mesh.add_triangle(triangle);
        mesh.compute_bounds();

        let (min, max) = mesh.bounds();
        assert_eq!(min, Point::new(-1.0, -1.0, -1.0));
        assert_eq!(max, Point::new(1.0, 1.0, -1.0));
    }

    #[test]
    fn test_ascii_detection() {
        let ascii_content = b"solid test
facet normal 0 0 1
  outer loop
    vertex -1 -1 0
    vertex 1 -1 0
    vertex 0 1 0
  endloop
endfacet
endsolid test";

        assert!(Mesh::is_ascii_stl_bytes(ascii_content).unwrap());
    }

    #[test]
    fn test_ascii_stl_parsing() {
        let ascii_content = b"solid test
facet normal 0 0 1
  outer loop
    vertex -1 -1 0
    vertex 1 -1 0
    vertex 0 1 0
  endloop
endfacet
facet normal 0 0 -1
  outer loop
    vertex 0 1 0
    vertex 1 -1 0
    vertex -1 -1 0
  endloop
endfacet
endsolid test";

        let mesh = Mesh::from_stl_bytes(ascii_content).unwrap();
        assert_eq!(mesh.triangle_count(), 2);

        // Check first triangle
        assert_eq!(mesh.triangles[0].vertices[0], Point::new(-1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[1], Point::new(1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[2], Point::new(0.0, 1.0, 0.0));
        assert_eq!(mesh.triangles[0].normal, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_binary_stl_parsing() {
        // Create a simple binary STL with one triangle
        let mut binary_data = vec![0u8; 80]; // header
        binary_data.extend_from_slice(&1u32.to_le_bytes()); // triangle count

        // Triangle data: normal + 3 vertices + attribute
        let normal = [0.0f32, 0.0f32, 1.0f32];
        let vertex1 = [-1.0f32, -1.0f32, 0.0f32];
        let vertex2 = [1.0f32, -1.0f32, 0.0f32];
        let vertex3 = [0.0f32, 1.0f32, 0.0f32];
        let attribute = 0u16;

        // Add normal
        for &f in &normal {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        // Add vertices
        for &f in &vertex1 {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        for &f in &vertex2 {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        for &f in &vertex3 {
            binary_data.extend_from_slice(&f.to_le_bytes());
        }
        // Add attribute
        binary_data.extend_from_slice(&attribute.to_le_bytes());

        let mesh = Mesh::from_stl_bytes(&binary_data).unwrap();
        assert_eq!(mesh.triangle_count(), 1);

        // Check triangle data
        assert_eq!(mesh.triangles[0].vertices[0], Point::new(-1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[1], Point::new(1.0, -1.0, 0.0));
        assert_eq!(mesh.triangles[0].vertices[2], Point::new(0.0, 1.0, 0.0));
        assert_eq!(mesh.triangles[0].normal, Vec3::new(0.0, 0.0, 1.0));
    }
}
