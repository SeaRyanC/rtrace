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

/// Fast scalar dot product of two [f64; 3] arrays.
#[inline(always)]
fn dot3(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Fast scalar cross product of two [f64; 3] arrays.
#[inline(always)]
fn cross3(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Fast subtraction of two [f64; 3] arrays.
#[inline(always)]
fn sub3(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Compute the half-surface-area of an AABB (used by SAH cost function).
#[inline]
fn aabb_half_area(min: &[f32; 3], max: &[f32; 3]) -> f32 {
    let dx = (max[0] - min[0]).max(0.0);
    let dy = (max[1] - min[1]).max(0.0);
    let dz = (max[2] - min[2]).max(0.0);
    dx * dy + dy * dz + dz * dx
}

// ─── Flat SAH BVH ─────────────────────────────────────────────────────────────

/// A flat BVH node occupying exactly 32 bytes (2 per 64-byte cache line).
///
/// **Internal node** (`tri_count == 0`): left child is always at index
/// `this_index + 1` (DFS-order), right child is at index `right_or_first`.
///
/// **Leaf node** (`tri_count > 0`): triangle slots are
/// `tri_indices[right_or_first .. right_or_first + tri_count]`.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub(crate) struct BvhNode {
    pub aabb_min: [f32; 3],
    /// Internal: index of right child.  Leaf: first index into `tri_indices`.
    pub right_or_first: u32,
    pub aabb_max: [f32; 3],
    /// 0 = internal node; > 0 = leaf holding this many triangles.
    pub tri_count: u32,
}

impl BvhNode {
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.tri_count > 0
    }

    /// Ray–AABB slab test in f32. Takes pre-converted f32 ray parameters to avoid
    /// f32→f64 widening in the traversal hot path. Returns `t_near` on a hit.
    #[inline]
    pub fn intersect_aabb_f32(
        &self,
        origin_f32: &[f32; 3],
        inv_dir_f32: &[f32; 3],
        t_min: f32,
        best_t: f32,
    ) -> Option<f32> {
        let mut t_near = t_min;
        let mut t_far = best_t;

        // All arithmetic in f32 — avoids f32→f64 widening, LLVM can vectorise.
        let tx0 = (self.aabb_min[0] - origin_f32[0]) * inv_dir_f32[0];
        let tx1 = (self.aabb_max[0] - origin_f32[0]) * inv_dir_f32[0];
        t_near = t_near.max(tx0.min(tx1));
        t_far = t_far.min(tx0.max(tx1));

        let ty0 = (self.aabb_min[1] - origin_f32[1]) * inv_dir_f32[1];
        let ty1 = (self.aabb_max[1] - origin_f32[1]) * inv_dir_f32[1];
        t_near = t_near.max(ty0.min(ty1));
        t_far = t_far.min(ty0.max(ty1));

        let tz0 = (self.aabb_min[2] - origin_f32[2]) * inv_dir_f32[2];
        let tz1 = (self.aabb_max[2] - origin_f32[2]) * inv_dir_f32[2];
        t_near = t_near.max(tz0.min(tz1));
        t_far = t_far.min(tz0.max(tz1));

        if t_near <= t_far {
            Some(t_near)
        } else {
            None
        }
    }

    /// Ray–AABB slab test (f64, kept for reference). Returns `t_near` on a hit.
    #[inline]
    #[allow(dead_code)]
    pub fn intersect_aabb(&self, origin: &[f64; 3], inv_dir: &[f64; 3], t_min: f64, best_t: f64) -> Option<f64> {
        let mut t_near = t_min;
        let mut t_far = best_t;

        // Unrolled over the 3 axes for the compiler to vectorise.
        let tx0 = (self.aabb_min[0] as f64 - origin[0]) * inv_dir[0];
        let tx1 = (self.aabb_max[0] as f64 - origin[0]) * inv_dir[0];
        t_near = t_near.max(tx0.min(tx1));
        t_far = t_far.min(tx0.max(tx1));

        let ty0 = (self.aabb_min[1] as f64 - origin[1]) * inv_dir[1];
        let ty1 = (self.aabb_max[1] as f64 - origin[1]) * inv_dir[1];
        t_near = t_near.max(ty0.min(ty1));
        t_far = t_far.min(ty0.max(ty1));

        let tz0 = (self.aabb_min[2] as f64 - origin[2]) * inv_dir[2];
        let tz1 = (self.aabb_max[2] as f64 - origin[2]) * inv_dir[2];
        t_near = t_near.max(tz0.min(tz1));
        t_far = t_far.min(tz0.max(tz1));

        if t_near <= t_far {
            Some(t_near)
        } else {
            None
        }
    }
}

/// Precomputed per-triangle data for fast Möller–Trumbore intersection.
///
/// Storing `edge1` and `edge2` avoids recomputing them for every ray–triangle
/// test. The `normal` field is the normalised front-face geometric normal
/// `normalize(edge1 × edge2)`; it is flipped when `a < 0` (back-face hit).
#[derive(Debug, Clone, Copy)]
pub(crate) struct PrecomputedTri {
    pub v0: [f64; 3],
    pub edge1: [f64; 3],
    pub edge2: [f64; 3],
    /// Normalised front-face normal: `normalize(edge1 × edge2)`.
    pub normal: [f64; 3],
}

/// Möller–Trumbore ray–triangle intersection (returns hit + normal + UV).
#[inline]
fn mt_intersect(
    tri: &PrecomputedTri,
    origin: &[f64; 3],
    dir: &[f64; 3],
    t_min: f64,
    t_max: f64,
) -> Option<(f64, [f64; 3], f64, f64)> {
    let h = cross3(dir, &tri.edge2);
    let a = dot3(&tri.edge1, &h);

    if a > -1e-8 && a < 1e-8 {
        return None; // ray parallel to triangle
    }

    let f = 1.0 / a;
    let s = sub3(origin, &tri.v0);
    let u = f * dot3(&s, &h);

    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = cross3(&s, &tri.edge1);
    let v = f * dot3(dir, &q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * dot3(&tri.edge2, &q);
    if t <= t_min || t >= t_max {
        return None;
    }

    // Flip normal for back-face hits (a < 0 means clockwise winding from ray's POV).
    let normal = if a >= 0.0 {
        tri.normal
    } else {
        [-tri.normal[0], -tri.normal[1], -tri.normal[2]]
    };

    Some((t, normal, u, v))
}

/// Shadow-ray variant: only checks if an intersection exists, skips UV / normal.
#[inline]
fn mt_intersect_any(
    tri: &PrecomputedTri,
    origin: &[f64; 3],
    dir: &[f64; 3],
    t_min: f64,
    t_max: f64,
) -> bool {
    let h = cross3(dir, &tri.edge2);
    let a = dot3(&tri.edge1, &h);

    if a > -1e-8 && a < 1e-8 {
        return false;
    }

    let f = 1.0 / a;
    let s = sub3(origin, &tri.v0);
    let u = f * dot3(&s, &h);

    if !(0.0..=1.0).contains(&u) {
        return false;
    }

    let q = cross3(&s, &tri.edge1);
    let v = f * dot3(dir, &q);

    if v < 0.0 || u + v > 1.0 {
        return false;
    }

    let t = f * dot3(&tri.edge2, &q);
    t > t_min && t < t_max
}

// ── BVH construction ──────────────────────────────────────────────────────────

const MAX_LEAF_TRIS: usize = 4;
const NUM_BINS: usize = 8;
/// Relative cost of traversing one BVH node vs. intersecting one triangle.
const SAH_TRAVERSAL_COST: f32 = 0.3;

struct BvhBuilder {
    nodes: Vec<BvhNode>,
    tri_indices: Vec<u32>,
}

impl BvhBuilder {
    fn build(precomputed: &[PrecomputedTri]) -> (Vec<BvhNode>, Vec<u32>) {
        let n = precomputed.len();
        if n == 0 {
            return (vec![], vec![]);
        }

        // Precompute centroids: centroid = v0 + (edge1 + edge2) / 3
        let centroids: Vec<[f32; 3]> = precomputed
            .iter()
            .map(|t| {
                [
                    (t.v0[0] + (t.edge1[0] + t.edge2[0]) / 3.0) as f32,
                    (t.v0[1] + (t.edge1[1] + t.edge2[1]) / 3.0) as f32,
                    (t.v0[2] + (t.edge1[2] + t.edge2[2]) / 3.0) as f32,
                ]
            })
            .collect();

        let mut builder = BvhBuilder {
            nodes: Vec::with_capacity(2 * n),
            tri_indices: (0..n as u32).collect(),
        };
        builder.build_node(&centroids, precomputed, 0, n);
        (builder.nodes, builder.tri_indices)
    }

    fn build_node(
        &mut self,
        centroids: &[[f32; 3]],
        precomputed: &[PrecomputedTri],
        start: usize,
        count: usize,
    ) {
        let node_idx = self.nodes.len();
        self.nodes.push(BvhNode::default()); // placeholder

        // Compute tight AABB over triangles in this node.
        let (aabb_min, aabb_max) = Self::compute_aabb(&self.tri_indices[start..start + count], precomputed);

        if count <= MAX_LEAF_TRIS {
            // Small enough to be a leaf without evaluating SAH.
            self.nodes[node_idx] = BvhNode {
                aabb_min,
                aabb_max,
                right_or_first: start as u32,
                tri_count: count as u32,
            };
            return;
        }

        // Attempt SAH-optimal split.
        let split_result = Self::find_sah_split(
            &self.tri_indices[start..start + count],
            centroids,
            precomputed,
            &aabb_min,
            &aabb_max,
            count,
        );

        let mid = match split_result {
            Some((axis, split_pos)) => {
                let left = partition_by_centroid(
                    &mut self.tri_indices[start..start + count],
                    centroids,
                    axis,
                    split_pos,
                );
                start + left
            }
            None => 0, // 0 signals "make a leaf"
        };

        if mid == 0 || mid == start || mid == start + count {
            // Degenerate – just create a leaf.
            self.nodes[node_idx] = BvhNode {
                aabb_min,
                aabb_max,
                right_or_first: start as u32,
                tri_count: count as u32,
            };
            return;
        }

        // Build left subtree – it starts at node_idx + 1 (DFS order).
        self.build_node(centroids, precomputed, start, mid - start);

        // Right child index is known only after the left subtree is built.
        let right_child = self.nodes.len() as u32;
        self.build_node(centroids, precomputed, mid, start + count - mid);

        // Patch in the internal node now that we know the right-child index.
        self.nodes[node_idx] = BvhNode {
            aabb_min,
            aabb_max,
            right_or_first: right_child,
            tri_count: 0, // internal
        };
    }

    /// Compute a tight f32 AABB over a set of triangles.
    fn compute_aabb(tris: &[u32], precomputed: &[PrecomputedTri]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];

        for &idx in tris {
            let t = &precomputed[idx as usize];
            let v1 = [
                t.v0[0] + t.edge1[0],
                t.v0[1] + t.edge1[1],
                t.v0[2] + t.edge1[2],
            ];
            let v2 = [
                t.v0[0] + t.edge2[0],
                t.v0[1] + t.edge2[1],
                t.v0[2] + t.edge2[2],
            ];

            for a in 0..3 {
                let lo = t.v0[a].min(v1[a]).min(v2[a]) as f32;
                let hi = t.v0[a].max(v1[a]).max(v2[a]) as f32;
                min[a] = min[a].min(lo);
                max[a] = max[a].max(hi);
            }
        }

        // Tiny expansion guards against f64→f32 rounding artefacts.
        const EPS: f32 = 1e-4;
        (
            [min[0] - EPS, min[1] - EPS, min[2] - EPS],
            [max[0] + EPS, max[1] + EPS, max[2] + EPS],
        )
    }

    /// Binned SAH split search.  Returns `Some((axis, split_pos))` when a split
    /// is cheaper than a leaf, otherwise `None`.
    fn find_sah_split(
        tris: &[u32],
        centroids: &[[f32; 3]],
        precomputed: &[PrecomputedTri],
        node_min: &[f32; 3],
        node_max: &[f32; 3],
        count: usize,
    ) -> Option<(usize, f32)> {
        let parent_area = aabb_half_area(node_min, node_max);
        if parent_area < 1e-10 {
            return None;
        }

        let leaf_cost = count as f32; // cost of not splitting
        let mut best_cost = leaf_cost;
        let mut best: Option<(usize, f32)> = None;

        // Compute centroid bounds for bin layout.
        let mut cmin = [f32::MAX; 3];
        let mut cmax = [f32::MIN; 3];
        for &idx in tris {
            let c = centroids[idx as usize];
            for a in 0..3 {
                cmin[a] = cmin[a].min(c[a]);
                cmax[a] = cmax[a].max(c[a]);
            }
        }

        for axis in 0..3 {
            let extent = cmax[axis] - cmin[axis];
            if extent < 1e-6 {
                continue;
            }
            let inv_extent = NUM_BINS as f32 / extent;

            // Initialise bins.
            let mut bin_count = [0u32; NUM_BINS];
            let mut bin_min = [[f32::MAX; 3]; NUM_BINS];
            let mut bin_max = [[f32::MIN; 3]; NUM_BINS];

            for &idx in tris {
                let c = centroids[idx as usize];
                let bin = ((c[axis] - cmin[axis]) * inv_extent) as usize;
                let bin = bin.min(NUM_BINS - 1);
                bin_count[bin] += 1;

                let t = &precomputed[idx as usize];
                let v1 = [
                    t.v0[0] + t.edge1[0],
                    t.v0[1] + t.edge1[1],
                    t.v0[2] + t.edge1[2],
                ];
                let v2 = [
                    t.v0[0] + t.edge2[0],
                    t.v0[1] + t.edge2[1],
                    t.v0[2] + t.edge2[2],
                ];
                for a in 0..3 {
                    let lo = t.v0[a].min(v1[a]).min(v2[a]) as f32;
                    let hi = t.v0[a].max(v1[a]).max(v2[a]) as f32;
                    bin_min[bin][a] = bin_min[bin][a].min(lo);
                    bin_max[bin][a] = bin_max[bin][a].max(hi);
                }
            }

            // Evaluate NUM_BINS-1 candidate split planes.
            // Precompute left-sweep AABB and counts.
            let mut left_count = [0u32; NUM_BINS - 1];
            let mut left_min = [[f32::MAX; 3]; NUM_BINS - 1];
            let mut left_max = [[f32::MIN; 3]; NUM_BINS - 1];

            let mut running_min = [f32::MAX; 3];
            let mut running_max = [f32::MIN; 3];
            let mut running_count = 0u32;

            for b in 0..NUM_BINS - 1 {
                running_count += bin_count[b];
                if bin_count[b] > 0 {
                    for a in 0..3 {
                        running_min[a] = running_min[a].min(bin_min[b][a]);
                        running_max[a] = running_max[a].max(bin_max[b][a]);
                    }
                }
                left_count[b] = running_count;
                left_min[b] = running_min;
                left_max[b] = running_max;
            }

            // Right-sweep and evaluate.
            let mut r_min = [f32::MAX; 3];
            let mut r_max = [f32::MIN; 3];
            let mut r_count = 0u32;

            for b in (0..NUM_BINS - 1).rev() {
                let rb = b + 1; // right bins start one past the split
                r_count += bin_count[rb];
                if bin_count[rb] > 0 {
                    for a in 0..3 {
                        r_min[a] = r_min[a].min(bin_min[rb][a]);
                        r_max[a] = r_max[a].max(bin_max[rb][a]);
                    }
                }

                if left_count[b] == 0 || r_count == 0 {
                    continue;
                }

                let left_area = aabb_half_area(&left_min[b], &left_max[b]);
                let right_area = aabb_half_area(&r_min, &r_max);
                let cost = SAH_TRAVERSAL_COST
                    + (left_area * left_count[b] as f32 + right_area * r_count as f32)
                        / parent_area;

                if cost < best_cost {
                    best_cost = cost;
                    // Split plane sits between bin b and b+1 in centroid space.
                    let split_pos = cmin[axis] + (b + 1) as f32 / inv_extent;
                    best = Some((axis, split_pos));
                }
            }
        }

        best
    }
}

/// Unstable in-place partition of `tris` by centroid on `axis` relative to
/// `split_pos`. Returns the number of elements placed in the left partition.
fn partition_by_centroid(
    tris: &mut [u32],
    centroids: &[[f32; 3]],
    axis: usize,
    split_pos: f32,
) -> usize {
    let n = tris.len();
    let mut l = 0;
    let mut r = n;

    while l < r {
        if centroids[tris[l] as usize][axis] < split_pos {
            l += 1;
        } else {
            r -= 1;
            tris.swap(l, r);
        }
    }

    l
}

/// Flat cache-friendly SAH BVH with iterative traversal.
#[derive(Clone, Debug)]
pub struct Bvh {
    pub(crate) nodes: Vec<BvhNode>,
    pub(crate) tri_indices: Vec<u32>,
    pub(crate) precomputed: Vec<PrecomputedTri>,
}

impl Bvh {
    /// Build a BVH from a slice of triangles.
    pub fn new(triangles: &[Triangle]) -> Self {
        let precomputed: Vec<PrecomputedTri> = triangles
            .iter()
            .map(|tri| {
                let v0 = tri.vertices[0];
                let v1 = tri.vertices[1];
                let v2 = tri.vertices[2];
                let edge1 = [v1.x - v0.x, v1.y - v0.y, v1.z - v0.z];
                let edge2 = [v2.x - v0.x, v2.y - v0.y, v2.z - v0.z];
                let raw_normal = cross3(&edge1, &edge2);
                let len = dot3(&raw_normal, &raw_normal).sqrt().max(1e-30);
                let normal = [
                    raw_normal[0] / len,
                    raw_normal[1] / len,
                    raw_normal[2] / len,
                ];
                PrecomputedTri {
                    v0: [v0.x, v0.y, v0.z],
                    edge1,
                    edge2,
                    normal,
                }
            })
            .collect();

        let (nodes, tri_indices) = BvhBuilder::build(&precomputed);
        Self {
            nodes,
            tri_indices,
            precomputed,
        }
    }

    /// Find the closest triangle intersection.
    ///
    /// Returns `(t, triangle_index, normal, u, v)` or `None`.
    pub fn hit_closest(
        &self,
        origin: &[f64; 3],
        dir: &[f64; 3],
        inv_dir: &[f64; 3],
        t_min: f64,
        t_max: f64,
    ) -> Option<(f64, usize, [f64; 3], f64, f64)> {
        if self.nodes.is_empty() {
            return None;
        }

        // Convert ray params to f32 once for the AABB slab tests.
        // AABB bounds are already f32, so f32 arithmetic avoids widening conversions
        // and gives the compiler more room to autovectorise.
        let origin_f32 = [origin[0] as f32, origin[1] as f32, origin[2] as f32];
        let inv_dir_f32 = [inv_dir[0] as f32, inv_dir[1] as f32, inv_dir[2] as f32];
        let t_min_f32 = t_min as f32;

        let mut best_t = t_max;
        let mut best: Option<(f64, usize, [f64; 3], f64, f64)> = None;

        // Explicit traversal stack (depth never exceeds ~50 for typical meshes).
        let mut stack = [0u32; 64];
        let mut top = 0usize;
        stack[top] = 0;
        top += 1;

        while top > 0 {
            top -= 1;
            let node_idx = stack[top] as usize;
            let node = &self.nodes[node_idx];

            // Re-test with the current (possibly tightened) best_t in f32.
            if node.intersect_aabb_f32(&origin_f32, &inv_dir_f32, t_min_f32, best_t as f32).is_none() {
                continue;
            }

            if node.is_leaf() {
                let first = node.right_or_first as usize;
                let end = first + node.tri_count as usize;
                for i in first..end {
                    let tri_idx = self.tri_indices[i] as usize;
                    if let Some((t, normal, u, v)) =
                        mt_intersect(&self.precomputed[tri_idx], origin, dir, t_min, best_t)
                    {
                        best_t = t;
                        best = Some((t, tri_idx, normal, u, v));
                    }
                }
            } else {
                let left_idx = node_idx + 1;
                let right_idx = node.right_or_first as usize;

                let t_left = self.nodes[left_idx].intersect_aabb_f32(&origin_f32, &inv_dir_f32, t_min_f32, best_t as f32);
                let t_right = self.nodes[right_idx].intersect_aabb_f32(&origin_f32, &inv_dir_f32, t_min_f32, best_t as f32);

                // Push far child first so the near child is popped (processed) first.
                match (t_left, t_right) {
                    (None, None) => {}
                    (Some(_), None) => {
                        stack[top] = left_idx as u32;
                        top += 1;
                    }
                    (None, Some(_)) => {
                        stack[top] = right_idx as u32;
                        top += 1;
                    }
                    (Some(tl), Some(tr)) => {
                        if tl <= tr {
                            // left is nearer – push right (far) then left (near)
                            stack[top] = right_idx as u32;
                            top += 1;
                            stack[top] = left_idx as u32;
                            top += 1;
                        } else {
                            stack[top] = left_idx as u32;
                            top += 1;
                            stack[top] = right_idx as u32;
                            top += 1;
                        }
                    }
                }
            }
        }

        best
    }

    /// Shadow-ray query: returns `true` as soon as any intersection is found.
    pub fn hit_any(
        &self,
        origin: &[f64; 3],
        dir: &[f64; 3],
        inv_dir: &[f64; 3],
        t_min: f64,
        t_max: f64,
    ) -> bool {
        if self.nodes.is_empty() {
            return false;
        }

        // Convert to f32 for AABB tests — all AABB bounds are already f32.
        let origin_f32 = [origin[0] as f32, origin[1] as f32, origin[2] as f32];
        let inv_dir_f32 = [inv_dir[0] as f32, inv_dir[1] as f32, inv_dir[2] as f32];
        let t_min_f32 = t_min as f32;
        let t_max_f32 = t_max as f32;

        let mut stack = [0u32; 64];
        let mut top = 0usize;
        stack[top] = 0;
        top += 1;

        while top > 0 {
            top -= 1;
            let node_idx = stack[top] as usize;
            let node = &self.nodes[node_idx];

            if node.intersect_aabb_f32(&origin_f32, &inv_dir_f32, t_min_f32, t_max_f32).is_none() {
                continue;
            }

            if node.is_leaf() {
                let first = node.right_or_first as usize;
                let end = first + node.tri_count as usize;
                for i in first..end {
                    let tri_idx = self.tri_indices[i] as usize;
                    if mt_intersect_any(&self.precomputed[tri_idx], origin, dir, t_min, t_max) {
                        return true;
                    }
                }
            } else {
                // Order doesn't matter for shadow rays – just push both.
                stack[top] = (node_idx + 1) as u32; // left
                top += 1;
                stack[top] = node.right_or_first; // right
                top += 1;
            }
        }

        false
    }
}

/// Public wrappers so `ray.rs` can call the MT functions for the brute-force path.
pub(crate) fn mt_intersect_pub(
    tri: &PrecomputedTri,
    origin: &[f64; 3],
    dir: &[f64; 3],
    t_min: f64,
    t_max: f64,
) -> Option<(f64, [f64; 3], f64, f64)> {
    mt_intersect(tri, origin, dir, t_min, t_max)
}

pub(crate) fn mt_intersect_any_pub(
    tri: &PrecomputedTri,
    origin: &[f64; 3],
    dir: &[f64; 3],
    t_min: f64,
    t_max: f64,
) -> bool {
    mt_intersect_any(tri, origin, dir, t_min, t_max)
}

/// Immutable mesh object containing triangles
#[derive(Debug, Clone)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub bounds_min: Point,
    pub bounds_max: Point,
    pub(crate) bvh: Bvh,
}

impl Mesh {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self {
            triangles: Vec::new(),
            bounds_min: Point::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            bounds_max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
            bvh: Bvh {
                nodes: Vec::new(),
                tri_indices: Vec::new(),
                precomputed: Vec::new(),
            },
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
    pub fn compute_bounds(&mut self) {
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

    /// Build the BVH acceleration structure for this mesh.
    pub fn build_kdtree(&mut self) {
        self.bvh = Bvh::new(&self.triangles);
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
