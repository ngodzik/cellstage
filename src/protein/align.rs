//! Laying one structure over another: the rotation and shift that bring
//! paired atoms closest, by Horn's method.
//!
//! Two crystal structures of the same protein are measured in frames of
//! their own. To see what changes between them, the parts that do not change
//! must first be put on top of each other.

use bevy::math::{DQuat, DVec3, Quat, Vec3};

/// The rotation and shift that carry points onto others.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Applied first, about the origin.
    pub rotation: Quat,
    /// Added after.
    pub shift: Vec3,
}

impl Fit {
    /// Where the fit carries `p`.
    pub fn apply(&self, p: Vec3) -> Vec3 {
        self.rotation * p + self.shift
    }
}

/// The rigid motion that carries `moving` closest to `target`, point for
/// point, in the least squares sense; `None` for fewer than three pairs.
///
/// Horn (J Opt Soc Am A, 1987): the best rotation, as a quaternion, is the
/// eigenvector of the largest eigenvalue of a four by four symmetric matrix
/// built from the two centred sets.
pub fn superpose(moving: &[Vec3], target: &[Vec3]) -> Option<Fit> {
    let n = moving.len().min(target.len());
    if n < 3 {
        return None;
    }
    let mean = |set: &[Vec3]| set[..n].iter().map(|p| p.as_dvec3()).sum::<DVec3>() / n as f64;
    let (cm, ct) = (mean(moving), mean(target));
    // Sum over pairs of the outer product of the centred points.
    let mut s = [[0.0f64; 3]; 3];
    for (p, q) in moving[..n].iter().zip(&target[..n]) {
        let (p, q) = (p.as_dvec3() - cm, q.as_dvec3() - ct);
        let (p, q) = (p.to_array(), q.to_array());
        for (a, row) in s.iter_mut().enumerate() {
            for (b, cell) in row.iter_mut().enumerate() {
                *cell += p[a] * q[b];
            }
        }
    }
    let [[xx, xy, xz], [yx, yy, yz], [zx, zy, zz]] = s;
    let horn = [
        [xx + yy + zz, yz - zy, zx - xz, xy - yx],
        [yz - zy, xx - yy - zz, xy + yx, zx + xz],
        [zx - xz, xy + yx, -xx + yy - zz, yz + zy],
        [xy - yx, zx + xz, yz + zy, -xx - yy + zz],
    ];
    let [w, x, y, z] = largest_eigenvector(horn);
    let rotation = DQuat::from_xyzw(x, y, z, w).normalize();
    let shift = ct - rotation * cm;
    Some(Fit {
        rotation: rotation.as_quat(),
        shift: shift.as_vec3(),
    })
}

/// The eigenvector of a symmetric four by four matrix with the largest
/// eigenvalue, by Jacobi rotations.
fn largest_eigenvector(mut a: [[f64; 4]; 4]) -> [f64; 4] {
    let mut v = [[0.0f64; 4]; 4];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for _ in 0..64 {
        let off: f64 = (0..4)
            .flat_map(|i| (0..4).filter(move |&j| j != i).map(move |j| (i, j)))
            .map(|(i, j)| a[i][j] * a[i][j])
            .sum();
        if off < 1e-22 {
            break;
        }
        for p in 0..3 {
            for q in p + 1..4 {
                if a[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for row in &mut a {
                    let (akp, akq) = (row[p], row[q]);
                    row[p] = c * akp - s * akq;
                    row[q] = s * akp + c * akq;
                }
                let (row_p, row_q) = (a[p], a[q]);
                for k in 0..4 {
                    a[p][k] = c * row_p[k] - s * row_q[k];
                    a[q][k] = s * row_p[k] + c * row_q[k];
                }
                for row in &mut v {
                    let (vp, vq) = (row[p], row[q]);
                    row[p] = c * vp - s * vq;
                    row[q] = s * vp + c * vq;
                }
            }
        }
    }
    let best = (0..4)
        .max_by(|&i, &j| a[i][i].total_cmp(&a[j][j]))
        .unwrap_or(0);
    [v[0][best], v[1][best], v[2][best], v[3][best]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rotated_and_shifted_copy_is_brought_back_exactly() {
        let points: Vec<Vec3> = (0..12)
            .map(|i| {
                let t = i as f32;
                Vec3::new(t.sin() * 5.0, t * 0.7, (t * 1.3).cos() * 3.0)
            })
            .collect();
        let turn = Quat::from_euler(bevy::math::EulerRot::XYZ, 0.7, -1.9, 2.4);
        let moved: Vec<Vec3> = points
            .iter()
            .map(|p| turn * *p + Vec3::new(10.0, -4.0, 2.5))
            .collect();
        let fit = superpose(&moved, &points).unwrap();
        for (m, p) in moved.iter().zip(&points) {
            assert!(
                fit.apply(*m).distance(*p) < 1e-3,
                "{} against {}",
                fit.apply(*m),
                p
            );
        }
    }

    #[test]
    fn fewer_than_three_pairs_fix_no_rotation() {
        assert!(superpose(&[Vec3::ZERO, Vec3::X], &[Vec3::ZERO, Vec3::Y]).is_none());
    }
}
